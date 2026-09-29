// SPDX-License-Identifier: MIT OR Apache-2.0

use core::future::Future;
use std::error::Error;
use std::io;

use bt_hci::controller::ExternalController;

use crate::hci::{Target, Transport};

pub fn run<F, Fut>(application: F) -> Result<(), Box<dyn Error>>
where
    F: FnOnce(ExternalController<Transport, 8>) -> Fut,
    Fut: Future<Output = ()>,
{
    env_logger::init();
    let mut arguments = std::env::args().skip(1);
    let target = Target::parse(arguments.next().as_deref())?;
    if let Some(argument) = arguments.next() {
        return Err(format!("unknown argument: {argument}").into());
    }

    let runtime = tokio::runtime::Builder::new_current_thread().enable_io().build()?;
    runtime.block_on(async move {
        let transport = Transport::connect(target).await?;
        application(ExternalController::new(transport)).await;
        Ok::<(), io::Error>(())
    })?;
    Ok(())
}
