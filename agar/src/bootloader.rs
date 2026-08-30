//! Entry into the board's UF2 bootloader
//!
//! The bootloader owns 0x08000000..0x08004000 and reads one word of SRAM
//! before it initializes anything. Write the magic there, reset, and that is
//! the whole protocol
//!
//! It is not ST's ROM DFU. dfu-util against the ROM bootloader writes from
//! 0x08000000, and the first write would erase the UF2 bootloader. The board
//! has no SWD pads to recover through. Nothing here may reach 0x1FFF_F000

use core::ptr;

use cortex_m::peripheral::SCB;

/// The word the bootloader reads on the way up. `memory.x` ends RAM below it
/// and asserts that. Nothing else can land on this address
const REQUEST: *mut u32 = 0x2000_4000 as *mut u32;

/// What the bootloader looks for. Any other value boots the firmware
///
/// The high half is 0x9D5B, the same vendor USB ID this firmware reports. The
/// pair belongs to the vendor's bootloader rather than to this board, and
/// their other STM32F1 boards take the same one
const REQUEST_MAGIC: u32 = 0x9D5B_FC2B;

/// Clears a request the bootloader has already acted on
///
/// The word sits outside every section. cortex-m-rt zeroes .bss and never
/// reaches it. Nobody documents whether the bootloader clears it after a
/// flash, and a request left set sends every later reset back to the
/// bootloader
pub fn clear_request() {
    unsafe { ptr::write_volatile(REQUEST, 0) };
}

/// Writes the request and resets into the bootloader
fn enter_bootloader() -> ! {
    unsafe { ptr::write_volatile(REQUEST, REQUEST_MAGIC) };
    // sys_reset does the dsb that gets the write out before the reset
    SCB::sys_reset()
}

/// Points RMK at [`enter_bootloader`]
///
/// `boot::jump_to_bootloader` only supports adafruit, rp2040 and the ZSA Voyager,
/// and warns for anything else. Without this call the `Bootloader` keycode and
/// the via and rynk `BootloaderJump` commands just reboot
pub fn register() {
    rmk::boot::register_bootloader_jump(enter_bootloader);
}
