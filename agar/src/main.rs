#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::peripherals::USB;
use embassy_stm32::rcc::{AHBPrescaler, APBPrescaler, Pll, PllMul, PllPreDiv, PllSource, Sysclk};
use embassy_stm32::usb::{Driver, InterruptHandler};
use embassy_stm32::{Config, bind_interrupts};
use embassy_time::Timer;
use panic_halt as _;
use rmk::config::DeviceConfig;

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

    loop {
        Timer::after_secs(1).await;
    }
}
