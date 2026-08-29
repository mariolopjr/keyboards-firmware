//! Key selection over a shift-register chain, one stage per key, with no
//! row/column grid
//!
//! A single token is clocked along the chain. Every key returns on one shared
//! line. That line is a push-pull output while the token is clocked and a
//! pulled-up input while a position is read. A closed switch pulls it low
//!
//! ```text
//!   clock  ----------->  shift clock
//!   sense  <---------->  data in / shared return
//! ```
//!
//! This type walks the token and turns a raw sample into a `KeyboardEvent`.
//! Debounce policy belongs to the debouncer

use cortex_m::asm::nop;
use embassy_stm32::gpio::{Flex, Pull, Speed};
use embassy_time::{Duration, Instant, Timer, block_for};
use embedded_hal::digital::OutputPin;
use rmk::debounce::{DebounceState, DebouncerTrait};
use rmk::event::KeyboardEvent;
use rmk::macros::input_device;
use rmk::matrix::{KeyState, MatrixTrait};

/// Shortest gap between the start of one pass and the next
const SCAN_INTERVAL_MS: u64 = 1;

/// Idle between releasing the shared line and reading it. The line was driven
/// low. A closed position is already low. An open one has to charge the trace
/// back up through the internal pull-up, and that is the slow case. The vendor
/// firmware reads a few hundred nanoseconds after the release and works on
/// this board. Costs `ROW * COL * SETTLE_US` per scan interval
const SETTLE_US: u64 = 5;

/// Level that marks the token on the data line. ver5020 clocks a high token,
/// ver595 inverts it
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokenPolarity {
    High,
    Low,
}

impl TokenPolarity {
    fn token_level(self) -> bool {
        self == TokenPolarity::High
    }
}

/// The shared line, alternating between driving the chain and reading the
/// selected position
pub trait SenseLine {
    /// Drive the line push-pull
    fn drive(&mut self, high: bool);

    /// Release the line to a pulled-up input
    fn release(&mut self);

    /// Read the line while it is released
    fn is_low(&self) -> bool;
}

#[cfg(feature = "matrix-instrument")]
struct Instrument<const ROW: usize, const COL: usize> {
    /// Previous raw sample, for counting transitions
    prev_closed: [[bool; ROW]; COL],
    /// Sample transitions since the last release
    flips: [[u16; ROW]; COL],
    /// Whether a run of open samples is in progress
    in_burst: [[bool; ROW]; COL],
    /// Start of the run in progress, in ms
    burst_start_ms: [[u32; ROW]; COL],
    /// Longest run of open samples since the last release, in ms
    max_burst_ms: [[u16; ROW]; COL],
}

/// Matrix over a chain of `ROW * COL` positions. Position `p` maps to row
/// `p / COL`, col `p % COL`. Arrays are indexed `[col][row]`, matching the RMK
/// matrices
#[input_device(publish = KeyboardEvent)]
pub struct ShiftMatrix<
    CLK: OutputPin,
    SENSE: SenseLine,
    D: DebouncerTrait<ROW, COL>,
    const ROW: usize,
    const COL: usize,
> {
    clock: CLK,
    sense: SENSE,
    debouncer: D,
    polarity: TokenPolarity,
    key_states: [[KeyState; ROW]; COL],
    /// Position the next sample reads. A pass resumes here after an event
    /// instead of reloading the token
    next_pos: usize,
    /// Earliest start of the next pass
    next_pass: Instant,
    #[cfg(feature = "matrix-instrument")]
    instrument: Instrument<ROW, COL>,
}

impl<
    CLK: OutputPin,
    SENSE: SenseLine,
    D: DebouncerTrait<ROW, COL>,
    const ROW: usize,
    const COL: usize,
> ShiftMatrix<CLK, SENSE, D, ROW, COL>
{
    /// Chain positions, one stage per key. Also the flush length. The chain
    /// does not circulate. A shorter flush leaves stale tokens in the tail
    /// stages, and those read as phantom keys
    const POSITIONS: usize = ROW * COL;

    pub fn new(clock: CLK, sense: SENSE, debouncer: D, polarity: TokenPolarity) -> Self {
        Self {
            clock,
            sense,
            debouncer,
            polarity,
            key_states: [[KeyState::new(); ROW]; COL],
            next_pos: 0,
            next_pass: Instant::now(),
            #[cfg(feature = "matrix-instrument")]
            instrument: Instrument {
                prev_closed: [[false; ROW]; COL],
                flips: [[0; ROW]; COL],
                in_burst: [[false; ROW]; COL],
                burst_start_ms: [[0; ROW]; COL],
                max_burst_ms: [[0; ROW]; COL],
            },
        }
    }

    fn clock_pulse(&mut self) {
        self.clock.set_high().ok();
        self.clock.set_low().ok();
    }

    /// Clear every stage, then clock a single token into stage 0
    fn load_token(&mut self) {
        self.sense.drive(!self.polarity.token_level());
        for _ in 0..Self::POSITIONS {
            self.clock_pulse();
        }
        self.sense.drive(self.polarity.token_level());
        self.clock_pulse();
        // stage 0 is selected now. A switch held there would fight the
        // output driver
        self.sense.release();
    }

    /// Shift the token one stage along
    fn advance_token(&mut self) {
        self.sense.drive(!self.polarity.token_level());
        self.clock_pulse();
    }

    /// Release the shared line and read the selected position twice. A
    /// transient that lands on only one read is rejected before it reaches the
    /// debouncer
    fn sample(&mut self) -> bool {
        self.sense.release();
        block_for(Duration::from_micros(SETTLE_US));
        let first = self.sense.is_low();
        nop();
        nop();
        let second = self.sense.is_low();
        first && second
    }

    async fn read_keyboard_event(&mut self) -> KeyboardEvent {
        loop {
            if self.next_pos == 0 {
                Timer::at(self.next_pass).await;
                self.next_pass = Instant::now() + Duration::from_millis(SCAN_INTERVAL_MS);
                self.load_token();
            }

            while self.next_pos < Self::POSITIONS {
                let pos = self.next_pos;
                let row = pos / COL;
                let col = pos % COL;

                // a closed switch pulls the line low
                let closed = self.sample();
                self.advance_token();
                self.next_pos += 1;

                #[cfg(feature = "matrix-instrument")]
                self.record_sample(row, col, closed);

                let debounce_state = self.debouncer.detect_change_with_debounce(
                    row,
                    col,
                    closed,
                    &self.key_states[col][row],
                );

                if let DebounceState::Debounced = debounce_state {
                    self.key_states[col][row].toggle_pressed();
                    let pressed = self.key_states[col][row].pressed;
                    #[cfg(feature = "matrix-instrument")]
                    if !pressed {
                        self.report_release(row, col);
                    }
                    return KeyboardEvent::key(row as u8, col as u8, pressed);
                }
            }

            self.next_pos = 0;
        }
    }

    /// Track transitions and runs of open samples during a hold. Nothing is
    /// formatted here. `log::info!` takes longer than the settle it measures.
    /// Reporting waits until the release leaves the scan loop
    #[cfg(feature = "matrix-instrument")]
    fn record_sample(&mut self, row: usize, col: usize, closed: bool) {
        if closed != self.instrument.prev_closed[col][row] {
            self.instrument.flips[col][row] = self.instrument.flips[col][row].saturating_add(1);
        }
        self.instrument.prev_closed[col][row] = closed;

        // a burst is a run of open samples while the key is still held. The
        // quiet release rides these out
        if self.key_states[col][row].pressed && !closed {
            if !self.instrument.in_burst[col][row] {
                self.instrument.in_burst[col][row] = true;
                self.instrument.burst_start_ms[col][row] = now_ms();
            }
        } else {
            self.end_burst(row, col);
        }
    }

    /// Fold the run in progress, if any, into the longest one seen
    #[cfg(feature = "matrix-instrument")]
    fn end_burst(&mut self, row: usize, col: usize) {
        if !self.instrument.in_burst[col][row] {
            return;
        }
        self.instrument.in_burst[col][row] = false;
        let len = now_ms().wrapping_sub(self.instrument.burst_start_ms[col][row]) as u16;
        if len > self.instrument.max_burst_ms[col][row] {
            self.instrument.max_burst_ms[col][row] = len;
        }
    }

    #[cfg(feature = "matrix-instrument")]
    fn report_release(&mut self, row: usize, col: usize) {
        // the run that committed the release is still open, and it is at
        // least as long as the release quiet time
        self.end_burst(row, col);
        log::info!(
            "matrix p{} r{}c{} release, flips {}, longest dropout {}ms",
            row * COL + col,
            row,
            col,
            self.instrument.flips[col][row],
            self.instrument.max_burst_ms[col][row]
        );
        self.instrument.max_burst_ms[col][row] = 0;
        self.instrument.flips[col][row] = 0;
    }
}

/// The line only reports the position the token sits on. There is no idle edge
/// to wait on, and nothing to implement under `async_matrix`. A token in every
/// stage would give an edge, but not which key caused it
impl<
    CLK: OutputPin,
    SENSE: SenseLine,
    D: DebouncerTrait<ROW, COL>,
    const ROW: usize,
    const COL: usize,
> MatrixTrait<ROW, COL> for ShiftMatrix<CLK, SENSE, D, ROW, COL>
{
}

#[cfg(feature = "matrix-instrument")]
fn now_ms() -> u32 {
    Instant::now().as_millis() as u32
}

impl<'d> SenseLine for Flex<'d> {
    fn drive(&mut self, high: bool) {
        // gpio_v1 selects the pull with ODR and `set_as_input(Pull::Up)` left
        // ODR set. Pick the level first. Switching to output first drives the
        // pin high until the level lands
        if high {
            self.set_high();
        } else {
            self.set_low();
        }
        self.set_as_output(Speed::VeryHigh);
    }

    fn release(&mut self) {
        self.set_as_input(Pull::Up);
    }

    fn is_low(&self) -> bool {
        Flex::is_low(self)
    }
}
