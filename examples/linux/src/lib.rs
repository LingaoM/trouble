// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(all(feature = "wall-clock", feature = "discrete-time"))]
compile_error!("features `wall-clock` and `discrete-time` cannot be enabled together");

#[cfg(not(any(feature = "wall-clock", feature = "discrete-time")))]
compile_error!("enable either the `wall-clock` or `discrete-time` feature");

#[cfg(all(feature = "discrete-time", not(feature = "wall-clock")))]
mod discrete_time;
mod hci;
#[cfg(all(feature = "wall-clock", not(feature = "discrete-time")))]
mod wall_clock;

#[cfg(all(feature = "discrete-time", not(feature = "wall-clock")))]
pub use discrete_time::run;
#[cfg(all(feature = "wall-clock", not(feature = "discrete-time")))]
pub use wall_clock::run;

#[cfg(feature = "security")]
use {embassy_crypto_rand as _, embassy_crypto_rustcrypto as _};
