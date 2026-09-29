// SPDX-License-Identifier: MIT OR Apache-2.0

mod hci;
mod linux;

pub use linux::run;

#[cfg(feature = "security")]
use {embassy_crypto_rand as _, embassy_crypto_rustcrypto as _};
