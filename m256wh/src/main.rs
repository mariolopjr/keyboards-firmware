#![no_main]
#![no_std]

use rmk::macros::rmk_keyboard;

// The `rmk_keyboard` macro reads keyboard.toml at compile time and generates
// everything needed for it to work and flash
#[rmk_keyboard]
mod keyboard {}
