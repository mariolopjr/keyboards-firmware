//! Indicator LEDs and the addressable chain
//!
//! ```text
//!   B14  caps lock indicator, GPIO
//!   A8   scroll lock indicator, GPIO
//!   B15  WS2812 data in
//!   B8   LED-type strap
//! ```
//!
//! B8 says how the indicators are wired. Tied low, the chain is 16 underglow
//! positions and nothing else. Left floating, which is how this board has it,
//! position 0 is an indicator ahead of those 16. The chain is 17 positions
//! for 18 LEDs. Two indicator packages share position 0 and always show the
//! same color
//!
//! Caps lock lights B14 and chain position 0 together and scroll lock has a
//! GPIO on A8 that is unused
//!
//! SPI2 MOSI clocks the chain out. No clock pin, no slave. One SPI byte
//! carries one WS2812 bit:
//!
//! ```text
//!   6 MHz, 166.7ns per SPI bit
//!   0  0b1100_0000   333ns high, 1000ns low
//!   1  0b1111_1000   833ns high,  500ns low
//! ```
//!
//! The WS2812B datasheet asks for 400/850 and 800/450 with +-150ns. All four
//! land inside that. T0L is at the far edge. Every pattern ends in a low bit.
//! MOSI sits low between frames with nothing holding it there

use embassy_stm32::mode::Async;
use embassy_stm32::spi::Spi;
use embassy_stm32::spi::mode::Master;
use embassy_time::Timer;
use rmk::event::LedIndicatorEvent;
use rmk::macros::processor;

/// Underglow positions. Both revisions carry these
const UNDERGLOW: usize = 16;

/// Chain positions on a board with an addressable indicator
pub const MAX_LEDS: usize = UNDERGLOW + 1;

/// One SPI byte per WS2812 bit, three color bytes per position
const MAX_BYTES: usize = MAX_LEDS * 3 * 8;

const ZERO: u8 = 0b1100_0000;
const ONE: u8 = 0b1111_1000;

/// Low time that latches a frame. WS2812B asks for 50us, the V5 revision for
/// 280us. Nobody knows which parts are on this board. Use the longer one
const LATCH_US: u64 = 300;

/// How the board wires its indicators, read off the B8 strap
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IndicatorStyle {
    /// B8 floating. Chain position 0 is an addressable indicator
    Addressable,
    /// B8 tied low. The chain is underglow only
    Gpio,
}

impl IndicatorStyle {
    /// The strap is read against a pull-up. A board that does not tie it low
    /// reads high. The caller gives the pull-up its settle time
    pub fn read(strapped_low: bool) -> Self {
        if strapped_low {
            IndicatorStyle::Gpio
        } else {
            IndicatorStyle::Addressable
        }
    }

    /// Chain positions to clock out
    pub fn led_count(self) -> usize {
        match self {
            IndicatorStyle::Addressable => MAX_LEDS,
            IndicatorStyle::Gpio => UNDERGLOW,
        }
    }

    /// Chain position of the caps lock indicator. B14 carries it on both
    /// revisions. This board carries it on position 0 as well
    pub fn caps_position(self) -> Option<usize> {
        match self {
            IndicatorStyle::Addressable => Some(0),
            IndicatorStyle::Gpio => None,
        }
    }

    /// Underglow positions, after any addressable indicator
    pub fn underglow(self) -> core::ops::Range<usize> {
        match self {
            IndicatorStyle::Addressable => 1..MAX_LEDS,
            IndicatorStyle::Gpio => 0..UNDERGLOW,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const OFF: Rgb = Rgb::new(0, 0, 0);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// The WS2812 chain on B15
///
/// Dropping this hands B15 back as an input. That leaves 18 WS2812 data pins
/// on a floating trace next to the matrix lines. [`ChainIndicator`] holds it
/// for the life of the firmware
pub struct Ws2812<'d> {
    spi: Spi<'d, Async, Master>,
    buf: [u8; MAX_BYTES],
    style: IndicatorStyle,
}

impl<'d> Ws2812<'d> {
    pub fn new(spi: Spi<'d, Async, Master>, style: IndicatorStyle) -> Self {
        Self {
            spi,
            buf: [ZERO; MAX_BYTES],
            style,
        }
    }

    pub fn style(&self) -> IndicatorStyle {
        self.style
    }

    /// Clocks every position out. Positions past the end of `colors` go dark
    ///
    /// The low time after the frame latches it. Colors are not on the LEDs
    /// until this returns
    pub async fn write(&mut self, colors: &[Rgb]) {
        let count = self.style.led_count();
        for pos in 0..count {
            let color = colors.get(pos).copied().unwrap_or(Rgb::OFF);
            // WS2812 takes green first
            for (byte, value) in [color.g, color.r, color.b].into_iter().enumerate() {
                let at = (pos * 3 + byte) * 8;
                for bit in 0..8 {
                    self.buf[at + bit] = if value & (0x80 >> bit) != 0 {
                        ONE
                    } else {
                        ZERO
                    };
                }
            }
        }

        // 408 bytes at 6 MHz is 544us. DMA keeps that off the matrix scan
        self.spi.write(&self.buf[..count * 3 * 8]).await.ok();
        Timer::after_micros(LATCH_US).await;
    }

    /// Blanks the chain and leaves it ready for a frame
    ///
    /// B15 sits high on its boot pull-up until SPI2 claims the pin. At
    /// power-on the chain has never seen the low time that resets it, and the
    /// first frame would land against an unknown state. The wait at the front
    /// of this is the only reset the chain gets
    ///
    /// embassy sets SPE when it builds the SPI. The shift register holds 0
    /// out of reset. MOSI is low from the moment the pin goes to alternate
    /// function
    pub async fn reset(&mut self) {
        Timer::after_micros(LATCH_US).await;
        self.write(&[]).await;
    }
}

/// Paints the chain from the host's LED report
///
/// RMK's `KeyboardIndicatorProcessor` drives the two GPIO indicators. This
/// drives the other half of caps lock, chain position 0. It owns the
/// underglow too. Both live in one frame
#[processor(subscribe = [LedIndicatorEvent])]
pub struct ChainIndicator<'d> {
    chain: Ws2812<'d>,
    underglow: Rgb,
    caps: Rgb,
}

impl<'d> ChainIndicator<'d> {
    pub fn new(chain: Ws2812<'d>, underglow: Rgb, caps: Rgb) -> Self {
        Self {
            chain,
            underglow,
            caps,
        }
    }

    /// Writes one frame. Call once before
    /// [`run`](rmk::core_traits::Runnable::run). Otherwise the chain stays
    /// dark until the first host report, which can be a long wait
    pub async fn paint(&mut self, caps_lock: bool) {
        let style = self.chain.style();
        let mut frame = [Rgb::OFF; MAX_LEDS];
        frame[style.underglow()].fill(self.underglow);
        if let Some(pos) = style.caps_position()
            && caps_lock
        {
            frame[pos] = self.caps;
        }
        self.chain.write(&frame).await;
    }

    async fn on_led_indicator_event(&mut self, event: LedIndicatorEvent) {
        self.paint(event.caps_lock()).await;
    }
}
