#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::gpio::{Flex, Input, Level, Output, Pull, Speed};
use embassy_stm32::peripherals::USB;
use embassy_stm32::rcc::{AHBPrescaler, APBPrescaler, Pll, PllMul, PllPreDiv, PllSource, Sysclk};
use embassy_stm32::usb::{Driver, InterruptHandler};
use embassy_stm32::{Config, bind_interrupts};
use embassy_time::Timer;
use panic_halt as _;
use rmk::config::DeviceConfig;

mod quiet_release;
mod shift_matrix;

use quiet_release::QuietReleaseDebouncer;
use shift_matrix::{ShiftMatrix, TokenPolarity};

/// How long a position must read open before a release commits. The shared
/// line drops out in bursts while a key is held. A release counted in samples
/// fires mid-hold which reads as a repeat on a letter and a dropped layer on
/// a momentary-layer key
///
/// The cost is the same number in release latency. Two taps of one key closer
/// together than this read as one press. The vendor firmware releases after 5
/// open samples and blocks a re-press for 12 ms instead
const RELEASE_QUIET_MS: u32 = 30;

bind_interrupts!(struct Irqs {
    USB_LP_CAN1_RX0 => InterruptHandler<USB>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = Config::default();

    // The board has no crystal, so the PLL runs off HSI. RM0008 7.3.2:
    // RCC_CFGR PLLSRC (bit 16) at 0 feeds the PLL from HSI/2 and PLLXTPRE
    // (bit 17) divides HSE only. That leaves 4 MHz x12 as the one way to
    // 48 MHz. embassy calls the fixed /2 `prediv` and panics on any other value
    config.rcc.pll = Some(Pll {
        src: PllSource::HSI,
        prediv: PllPreDiv::DIV2,
        mul: PllMul::MUL12,
    });
    config.rcc.sys = Sysclk::PLL1_P;
    config.rcc.ahb_pre = AHBPrescaler::DIV1;
    // APB1 tops out at 36 MHz
    config.rcc.apb1_pre = APBPrescaler::DIV2;
    config.rcc.apb2_pre = APBPrescaler::DIV1;
    // USBPRE (bit 22) is not a config field. embassy reads the PLL output and
    // picks /1 at 48 MHz, the only setting that gives USB the 48 MHz it needs

    let mut p = embassy_stm32::init(config);

    // D+ carries a fixed pull-up, and a reset leaves the host still holding
    // the previous device. Driving the line low for the 100 ms connect
    // debounce in USB 2.0 7.1.7.3 forces a disconnect first
    {
        let _dp = Output::new(p.PA12.reborrow(), Level::Low, Speed::Low);
        Timer::after_millis(100).await;
    }

    // The driver on its own leaves CNTR.FRES set, holding the peripheral in
    // reset. Building a UsbTransport clears it and puts the device on the bus,
    // which needs something answering the control pipe first
    let _driver = Driver::new(p.USB, Irqs, p.PA12, p.PA11);

    let _device_config = DeviceConfig {
        vid: 0x9D5B,
        pid: 0x240B,
        manufacturer: "KBDFans_YDKB",
        product_name: "Agar Keyboard",
        ..Default::default()
    };

    // B9 is tied low on ver5020 and floats on ver595. The two revisions invert
    // the token level on the data line. The 10 ms lets the pull-up settle
    let polarity = {
        let revision = Input::new(p.PB9.reborrow(), Pull::Up);
        Timer::after_millis(10).await;
        if revision.is_low() {
            TokenPolarity::High
        } else {
            TokenPolarity::Low
        }
    };

    // B12 clocks the chain, B13 carries the token in and the switch return out
    let clock = Output::new(p.PB12, Level::Low, Speed::VeryHigh);
    let sense = Flex::new(p.PB13);
    let debouncer = QuietReleaseDebouncer::new(RELEASE_QUIET_MS);
    // the keyboard task that runs this, and the USB transport it publishes
    // through, are wired up in a later phase
    let _matrix = ShiftMatrix::<_, _, _, 9, 8>::new(clock, sense, debouncer, polarity);

    loop {
        Timer::after_secs(1).await;
    }
}
