//! Passes the linker scripts and flags for the STM32F401RC

fn main() {
    println!("cargo:rerun-if-changed=keyboard.toml");

    // `--nmagic` is required if memory section addresses are not aligned to 0x10000
    println!("cargo:rustc-link-arg=--nmagic");
    // Linker script provided by cortex-m-rt.
    println!("cargo:rustc-link-arg=-Tlink.x");
}
