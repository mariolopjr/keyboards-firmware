//! Puts memory.x where the linker can find it and sets the linker flags

use std::path::PathBuf;
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=memory.x");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out.join("memory.x"), include_bytes!("memory.x")).unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // --nmagic is required when section addresses are not aligned to 0x10000
    println!("cargo:rustc-link-arg=--nmagic");
    // linker script from cortex-m-rt
    println!("cargo:rustc-link-arg=-Tlink.x");
}
