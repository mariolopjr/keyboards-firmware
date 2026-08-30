#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::gpio::{Flex, Input, Level, Output, Pull, Speed};
use embassy_stm32::peripherals::{DMA1_CH5, SPI2, USB};
use embassy_stm32::rcc::{AHBPrescaler, APBPrescaler, Pll, PllMul, PllPreDiv, PllSource, Sysclk};
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::time::Hertz;
use embassy_stm32::usb::{Driver, InterruptHandler};
use embassy_stm32::{Config, bind_interrupts, dma, rcc};
use embassy_time::Timer;
use panic_halt as _;
use rmk::config::{DeviceConfig, PositionalConfig};
use rmk::keyboard::Keyboard;
use rmk::processor::builtin::led_indicator::KeyboardIndicatorProcessor;
use rmk::types::led_indicator::LedIndicatorType;
use rmk::usb::UsbTransport;
use rmk::{KeymapData, initialize_keymap, run_all};

mod bootloader;
mod keymap;
mod leds;
mod quiet_release;
mod shift_matrix;

use keymap::{COL, ROW};
use leds::{ChainIndicator, IndicatorStyle, Rgb, Ws2812};
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

/// One SPI byte carries one WS2812 bit. That puts the chain at 6 MHz. PCLK1
/// is 24 MHz and the SPI prescalers are powers of two. /4 lands on it exactly
const WS2812_HZ: u32 = 6_000_000;

/// What the underglow shows
const UNDERGLOW: Rgb = Rgb::new(8, 8, 8);

/// The addressable half of the caps lock indicator
const CAPS_LOCK: Rgb = Rgb::new(64, 64, 64);

bind_interrupts!(struct Irqs {
    USB_LP_CAN1_RX0 => InterruptHandler<USB>;
    DMA1_CHANNEL5 => dma::InterruptHandler<DMA1_CH5>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // The bootloader read its request word long before this. Clearing it stops
    // a stale request from sending the next reset back to the bootloader
    bootloader::clear_request();
    bootloader::register();

    let mut config = Config::default();

    // The board has no crystal. The PLL runs off HSI. RM0008 7.3.2:
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

    // B9 and B8 are straps, both read against the internal pull-up. A board
    // that does not tie one low reads it high. The 10 ms lets the pull-up
    // charge the trace
    let (polarity, indicators) = {
        let revision = Input::new(p.PB9.reborrow(), Pull::Up);
        let led_type = Input::new(p.PB8.reborrow(), Pull::Up);
        Timer::after_millis(10).await;
        // B9 low is ver5020. It clocks a high token, ver595 inverts it
        let polarity = if revision.is_low() {
            TokenPolarity::High
        } else {
            TokenPolarity::Low
        };
        (polarity, IndicatorStyle::read(led_type.is_low()))
    };

    // Claimed before the matrix pins. Out of reset B15 is a floating input, a
    // 40k pull-up on the data in of 18 WS2812s, next to the pins the matrix
    // scan toggles hardest. SPI2 drives it push-pull from here on. The strap
    // read above costs it 10 ms
    //
    // embassy divides the SPI kernel clock by a power of two and rounds the
    // divider up. A clock tree that stops dividing cleanly gives out-of-spec
    // WS2812 timing and reports nothing. Halt at boot instead
    let kernel_hz = rcc::frequency::<SPI2>().0;
    assert!(kernel_hz.is_multiple_of(WS2812_HZ) && (kernel_hz / WS2812_HZ).is_power_of_two());

    let mut spi_config = spi::Config::default();
    spi_config.frequency = Hertz(WS2812_HZ);
    let mut chain = Ws2812::new(
        Spi::new_txonly_nosck(p.SPI2, p.PB15, p.DMA1_CH5, Irqs, spi_config),
        indicators,
    );
    chain.reset().await;

    // D+ carries a fixed pull-up, and a reset leaves the host still holding
    // the previous device. Driving the line low for the 100 ms connect
    // debounce in USB 2.0 7.1.7.3 forces a disconnect first
    {
        let _dp = Output::new(p.PA12.reborrow(), Level::Low, Speed::Low);
        Timer::after_millis(100).await;
    }

    // The driver on its own leaves CNTR.FRES set. That holds the peripheral
    // in reset. Building the UsbTransport clears it and puts the device on the
    // bus, which is why that one is built last
    let driver = Driver::new(p.USB, Irqs, p.PA12, p.PA11);

    let device_config = DeviceConfig {
        vid: 0x9D5B,
        pid: 0x240B,
        manufacturer: "KBDFans_YDKB",
        product_name: "Agar Keyboard",
        ..Default::default()
    };

    // B12 clocks the chain, B13 carries the token in and the switch return out
    let clock = Output::new(p.PB12, Level::Low, Speed::VeryHigh);
    let sense = Flex::new(p.PB13);
    let debouncer = QuietReleaseDebouncer::new(RELEASE_QUIET_MS);
    let mut matrix = ShiftMatrix::<_, _, _, ROW, COL>::new(clock, sense, debouncer, polarity);

    // The keymap borrows both of these, and the keyboard borrows the keymap
    let mut keymap_data = KeymapData::new(keymap::get_default_keymap());
    let mut behavior_config = keymap::behavior_config();
    // Per-key overrides on the behavior config. Nothing needs one yet
    let positional_config = PositionalConfig::default();
    let keymap =
        initialize_keymap(&mut keymap_data, &mut behavior_config, &positional_config).await;
    let mut keyboard = Keyboard::new(&keymap);

    // Both indicators start off. Low is the off level per the vendor's
    // `single_color_indicator_set`
    let mut caps_gpio = KeyboardIndicatorProcessor::new(
        Output::new(p.PB14, Level::Low, Speed::Low),
        false,
        LedIndicatorType::CapsLock,
    );
    let mut scroll_gpio = KeyboardIndicatorProcessor::new(
        Output::new(p.PA8, Level::Low, Speed::Low),
        false,
        LedIndicatorType::ScrollLock,
    );

    // Owns the chain from here. Caps lock lights B14 and chain position 0 off
    // the one event
    let mut chain_indicator = ChainIndicator::new(chain, UNDERGLOW, CAPS_LOCK);
    chain_indicator.paint(false).await;

    // Puts the device on the bus
    let mut usb_transport = UsbTransport::new(driver, device_config);

    run_all!(
        matrix,
        keyboard,
        usb_transport,
        caps_gpio,
        scroll_gpio,
        chain_indicator
    )
    .await;
    // Unreachable while `Runnable::run` returns `!`. It stands in case that
    // changes upstream. Without it main returns, drops the chain and hands
    // B15 back as a floating input
    core::future::pending().await
}
