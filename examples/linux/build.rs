// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(all(feature = "discrete-time", not(feature = "wall-clock")))]
use std::env;
#[cfg(all(feature = "discrete-time", not(feature = "wall-clock")))]
use std::path::{Path, PathBuf};

#[cfg(all(feature = "discrete-time", not(feature = "wall-clock")))]
fn required_dir(name: &str) -> PathBuf {
    let path = PathBuf::from(env::var_os(name).unwrap_or_else(|| panic!("{name} is not set")));
    assert!(path.is_dir(), "{name} is not a directory: {}", path.display());
    path
}

#[cfg(all(feature = "discrete-time", not(feature = "wall-clock")))]
fn required_file(path: &Path) {
    assert!(path.is_file(), "required file is missing: {}", path.display());
}

#[cfg(all(feature = "discrete-time", not(feature = "wall-clock")))]
fn main() {
    println!("cargo:rerun-if-env-changed=BSIM_COMPONENTS_PATH");
    println!("cargo:rerun-if-env-changed=BSIM_OUT_PATH");

    let components = required_dir("BSIM_COMPONENTS_PATH");
    let output = required_dir("BSIM_OUT_PATH");
    let phy = output.join("lib/libPhyComv1.a");
    let util = output.join("lib/libUtilv1.a");
    required_file(&phy);
    required_file(&util);

    cc::Build::new()
        .file("src/bsim_shim.c")
        .include(components.join("libUtilv1/src"))
        .include(components.join("libPhyComv1/src"))
        .define("_POSIX_C_SOURCE", "200809L")
        .warnings(true)
        .compile("trouble_bsim_shim");

    println!("cargo:rerun-if-changed=src/bsim_shim.c");
    println!("cargo:rerun-if-changed={}", phy.display());
    println!("cargo:rerun-if-changed={}", util.display());
    println!("cargo:rustc-link-arg=-Wl,--start-group");
    println!("cargo:rustc-link-arg={}", phy.display());
    println!("cargo:rustc-link-arg={}", util.display());
    println!("cargo:rustc-link-arg=-Wl,--end-group");
}

#[cfg(any(not(feature = "discrete-time"), feature = "wall-clock"))]
fn main() {}
