//! Debouncer for a return line that reads open while a key is still held
//!
//! A press commits on two closed samples in a row, the same threshold the
//! vendor firmware uses (`DEBOUNCE_NK` of 1). A release commits only after the
//! position reads open for `release_quiet_ms`. A run of open samples during a
//! hold is not long enough to count as a release

use embassy_time::Instant;
use rmk::debounce::{DebounceState, DebouncerTrait};
use rmk::matrix::KeyState;

/// Arrays are indexed `[col][row]`, matching the RMK debouncers
pub struct QuietReleaseDebouncer<const ROW: usize, const COL: usize> {
    /// Previous raw sample. A press needs two closed samples in a row
    prev_closed: [[bool; ROW]; COL],
    /// When each position last read closed, in ms. Only read while the
    /// position is pressed. A position cannot be pressed before a closed
    /// sample has written this, and the starting value is never compared
    last_closed_ms: [[u32; ROW]; COL],
    release_quiet_ms: u32,
}

impl<const ROW: usize, const COL: usize> QuietReleaseDebouncer<ROW, COL> {
    /// `release_quiet_ms` is how long a position must read open before a
    /// release commits. It is also the release latency, and the shortest gap
    /// between two taps of one key that still reads as two presses
    pub fn new(release_quiet_ms: u32) -> Self {
        Self {
            prev_closed: [[false; ROW]; COL],
            last_closed_ms: [[0; ROW]; COL],
            release_quiet_ms,
        }
    }
}

impl<const ROW: usize, const COL: usize> DebouncerTrait<ROW, COL>
    for QuietReleaseDebouncer<ROW, COL>
{
    fn detect_change_with_debounce(
        &mut self,
        row_idx: usize,
        col_idx: usize,
        key_active: bool,
        key_state: &KeyState,
    ) -> DebounceState {
        let prev_closed = self.prev_closed[col_idx][row_idx];
        self.prev_closed[col_idx][row_idx] = key_active;

        let now = Instant::now().as_millis() as u32;
        if key_active {
            self.last_closed_ms[col_idx][row_idx] = now;
        }

        if !key_state.pressed {
            return match (key_active, prev_closed) {
                (true, true) => DebounceState::Debounced,
                (true, false) => DebounceState::InProgress,
                (false, _) => DebounceState::Ignored,
            };
        }

        if key_active {
            return DebounceState::Ignored;
        }

        let quiet = now.wrapping_sub(self.last_closed_ms[col_idx][row_idx]);
        if quiet >= self.release_quiet_ms {
            DebounceState::Debounced
        } else {
            DebounceState::InProgress
        }
    }
}
