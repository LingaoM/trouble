// SPDX-License-Identifier: MIT OR Apache-2.0

use core::future::Future;
use core::task::Waker;
use std::error::Error;
use std::ffi::CString;
use std::io;
use std::io::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use bt_hci::controller::ExternalController;
use embassy_time::Instant;
use embassy_time_driver::Driver;
use embassy_time_queue_utils::Queue;
use tokio::sync::Notify;

use crate::hci::{Target, Transport};

const MAX_STEP_US: u64 = 1_000;

unsafe extern "C" {
    fn trouble_bsim_init(
        sim_id: *const libc::c_char,
        phy_id: *const libc::c_char,
        device_id: libc::c_uint,
    ) -> libc::c_int;
    fn trouble_bsim_step(target_us: u64, actual_us: *mut u64) -> libc::c_int;
    fn trouble_bsim_disconnect();
}

struct BsimTimeDriver {
    now_us: AtomicU64,
    queue: Mutex<Queue>,
}

embassy_time_driver::time_driver_impl!(static DRIVER: BsimTimeDriver = BsimTimeDriver {
    now_us: AtomicU64::new(0),
    queue: Mutex::new(Queue::new()),
});

impl Driver for BsimTimeDriver {
    fn now(&self) -> u64 {
        self.now_us.load(Ordering::Acquire)
    }

    fn schedule_wake(&self, at: u64, waker: &Waker) {
        self.queue.lock().unwrap().schedule_wake(at, waker);
    }
}

impl BsimTimeDriver {
    fn reset(&self) {
        self.now_us.store(0, Ordering::Release);
    }

    fn advance_when_idle(&self) {
        let now = self.now();
        let next_timer = self.queue.lock().unwrap().next_expiration(now);
        let target = now.saturating_add(MAX_STEP_US).min(next_timer);
        let mut actual = now;
        let result = unsafe { trouble_bsim_step(target, &mut actual) };
        assert_eq!(result, 0, "BabbleSim time step failed: {result}");

        self.now_us.store(actual, Ordering::Release);
        self.queue.lock().unwrap().next_expiration(actual);
    }
}

struct Arguments {
    hci_target: Target,
    sim_id: String,
    phy_id: String,
    device_id: u32,
}

impl Arguments {
    fn parse() -> Result<Self, String> {
        let mut hci_target = None;
        let mut sim_id = None;
        let mut phy_id = None;
        let mut device_id = None;

        for argument in std::env::args().skip(1) {
            if let Some(value) = argument.strip_prefix("-s=") {
                sim_id = Some(value.to_owned());
            } else if let Some(value) = argument.strip_prefix("-p=") {
                phy_id = Some(value.to_owned());
            } else if let Some(value) = argument.strip_prefix("-d=") {
                device_id = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| format!("invalid BabbleSim device id: {value}"))?,
                );
            } else if argument.starts_with('-') {
                return Err(format!("unknown argument: {argument}"));
            } else if hci_target.is_none() {
                hci_target = Some(Target::parse(Some(&argument))?);
            } else {
                return Err(format!("unknown argument: {argument}"));
            }
        }

        Ok(Self {
            hci_target: hci_target.unwrap_or(Target::Bluez(0)),
            sim_id: sim_id.ok_or("-s is required")?,
            phy_id: phy_id.ok_or("-p is required")?,
            device_id: device_id.ok_or("-d is required")?,
        })
    }
}

struct Session;

impl Session {
    fn connect(arguments: &Arguments) -> io::Result<Self> {
        let sim_id = CString::new(arguments.sim_id.as_str())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid simulation id"))?;
        let phy_id = CString::new(arguments.phy_id.as_str())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid PHY id"))?;
        let result = unsafe { trouble_bsim_init(sim_id.as_ptr(), phy_id.as_ptr(), arguments.device_id) };
        if result != 0 {
            return Err(io::Error::other(format!("failed to join BabbleSim: {result}")));
        }
        DRIVER.reset();
        Ok(Self)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { trouble_bsim_disconnect() };
    }
}

fn run_runtime<F: Future>(arguments: &Arguments, future: F) -> io::Result<F::Output> {
    let _session = Session::connect(arguments)?;
    let reactor_turn = Arc::new(Notify::new());
    let wake_reactor = reactor_turn.clone();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .on_thread_park(move || {
            DRIVER.advance_when_idle();
            wake_reactor.notify_one();
        })
        .build()?;

    // A completed BabbleSim step may make HCI data ready without expiring an
    // Embassy timer. Wake one reusable task so Tokio polls I/O before stepping.
    runtime.spawn(async move {
        loop {
            reactor_turn.notified().await;
        }
    });
    Ok(runtime.block_on(future))
}

fn init_logger() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|formatter, record| {
            let now_us = Instant::now().as_micros();
            let hours = now_us / 3_600_000_000;
            let minutes = now_us / 60_000_000 % 60;
            let seconds = now_us / 1_000_000 % 60;
            let micros = now_us % 1_000_000;
            writeln!(
                formatter,
                "[{hours:02}:{minutes:02}:{seconds:02}.{micros:06} {:5} {}] {}",
                record.level(),
                record.target(),
                record.args()
            )
        })
        .init();
}

pub fn run<F, Fut>(application: F) -> Result<(), Box<dyn Error>>
where
    F: FnOnce(ExternalController<Transport, 8>) -> Fut,
    Fut: Future<Output = ()>,
{
    init_logger();
    let arguments = Arguments::parse()?;
    let hci_target = arguments.hci_target.clone();
    run_runtime(&arguments, async move {
        let transport = Transport::connect(hci_target).await?;
        application(ExternalController::new(transport)).await;
        Ok::<(), io::Error>(())
    })??;
    Ok(())
}
