//! Layout for a 60% HHKB/tsangan board on the 72-position chain
//!
//! Position `p` in the chain maps to row `p / COL`, col `p % COL`. The chain
//! order is wiring order. Matrix coordinates have no relation to the physical
//! rows. The physical board is
//!
//! ```text
//!   Esc  1  2  3  4  5  6  7  8  9  0  -  =  \  `
//!   Tab  Q  W  E  R  T  Y  U  I  O  P  [  ]  Bspc
//!   Ctrl A  S  D  F  G  H  J  K  L  ;  '  Enter
//!   Shift Z X  C  V  B  N  M  ,  .  /  Shift Fn
//!   ----  Alt Gui  ------- Space -------  Gui Alt ----
//! ```
//!
//! 62 of the 72 positions carry a switch. The two 1.5u ends of the bottom row
//! are switches with nothing on them, matching the blank caps that ship with
//! this bottom row

use embassy_time::Duration;
use rmk::config::{BehaviorConfig, MorsesConfig};
use rmk::types::action::{Action, KeyAction};
use rmk::types::constants::MORSE_PROFILE_MAX_NUM;
use rmk::types::keycode::{HidKeyCode, KeyCode};
use rmk::types::modifier::ModifierCombination;
use rmk::types::morse::{MorseMode, MorseProfile};
use rmk::{a, k, kbctrl, layer, mo};

pub const ROW: usize = 9;
pub const COL: usize = 8;
pub const NUM_LAYER: usize = 2;

/// Falls through to the layer below
const ___: KeyAction = a!(Transparent);

/// No switch or a switch with nothing on it
const XXX: KeyAction = a!(No);

/// Morse profiles in the order `behavior_config` loads them. A
/// `KeyAction::TapHold` carries an index into this table
const PROFILES: [MorseProfile; 1] = [
    // NAV: timing for the Escape/Ctrl position. `HoldOnOtherPress` resolves to
    // Ctrl the moment another key goes down. That is right for a modifier only
    // ever used with the other hand. `hold_timeout` is what a hold with no
    // other key costs before it counts as Ctrl
    //
    // Flow tap is off here. It is checked before the mode and wins outright.
    // With it on, a Ctrl press within `prior_idle_time` of the previous
    // keystroke sends Escape and the chord is lost. It guards same-hand rolls
    // on home-row mods and has nothing to do on a dedicated Ctrl key
    MorseProfile::new(None, Some(MorseMode::HoldOnOtherPress), Some(180), None)
        .with_enable_flow_tap(Some(false)),
];

/// Index of the nav profile in [`PROFILES`]
const NAV: u8 = 0;

// an index past the end of the table resolves to the default profile at
// runtime with no error. Bound it here instead
const _: () = assert!((NAV as usize) < PROFILES.len());
const _: () = assert!(PROFILES.len() <= MORSE_PROFILE_MAX_NUM);

/// Reboots into the UF2 bootloader. `bootloader::register` supplies the
/// sequence. Without it this keycode only reboots
const BOOT: KeyAction = kbctrl!(Bootloader);

/// Tap for Escape, hold for Ctrl. The board has no dedicated Caps position,
/// so this is the HHKB Ctrl below Tab
const ESC_CTRL: KeyAction = KeyAction::TapHold(
    Action::Key(KeyCode::Hid(HidKeyCode::Escape)),
    Action::Modifier(ModifierCombination::LCTRL),
    NAV,
);

/// RMK reads a debounce window from `[rmk] debounce_time` in a
/// `keyboard.toml`. Only `DefaultDebouncer` and `FastDebouncer` use it. This
/// crate has no `keyboard.toml` and does not set `KEYBOARD_TOML_PATH`, which
/// leaves every one of those constants at the upstream default. The matrix
/// runs `QuietReleaseDebouncer`. The window is not in the path either way
pub fn behavior_config() -> BehaviorConfig {
    let mut morse = MorsesConfig {
        // a tap-hold pressed within `prior_idle_time` of the previous keypress
        // resolves as a tap, for any profile that does not opt out. Pinned
        // rather than inherited. An upstream change to the default does not
        // move it
        enable_flow_tap: true,
        prior_idle_time: Duration::from_millis(120),
        ..Default::default()
    };
    // cannot fail, the assert above bounds PROFILES by the vec capacity
    let _ = morse.profiles.extend_from_slice(&PROFILES);

    BehaviorConfig {
        morse,
        ..Default::default()
    }
}

/// Layer 1 puts the function row on the number row and an arrow cluster on the
/// right of the alpha block, and [`BOOT`] on B
///
/// ```text
///        [ Up      ] PgUp
///   L Home  ; Left   ' Right
///   , End   . PgDn   / Down
/// ```
#[rustfmt::skip]
pub const fn get_default_keymap() -> [[[KeyAction; COL]; ROW]; NUM_LAYER] {
    [
        layer!([
          // col 0             col 1             col 2             col 3             col 4             col 5             col 6             col 7
            [k!(Escape),       k!(Tab),          k!(Q),            k!(W),            k!(Kc1),          k!(Kc2),          k!(Kc3),          k!(E)],
            [k!(Z),            k!(LGui),         k!(S),            XXX,              XXX,              k!(LShift),       k!(A),            ESC_CTRL],
            [k!(Kc4),          k!(R),            k!(Kc5),          k!(T),            k!(Kc6),          k!(Kc7),          k!(Y),            k!(U)],
            [k!(B),            k!(G),            k!(V),            k!(C),            k!(LAlt),         k!(X),            k!(F),            k!(D)],
            [k!(Kc8),          k!(I),            k!(Kc9),          k!(Kc0),          k!(Minus),        k!(O),            k!(P),            k!(LeftBracket)],
            [k!(L),            k!(Comma),        k!(M),            k!(Space),        k!(N),            k!(K),            k!(J),            k!(H)],
            [k!(Equal),        k!(Backslash),    k!(Grave),        k!(RightBracket), k!(Backspace),    k!(Enter),        k!(RShift),       XXX],
            [mo!(1),           XXX,              k!(RGui),         k!(RAlt),         k!(Slash),        k!(Dot),          k!(Quote),        k!(Semicolon)],
            [XXX,              XXX,              XXX,              XXX,              XXX,              XXX,              XXX,              XXX]
        ]),
        layer!([
          // col 0             col 1             col 2             col 3             col 4             col 5             col 6             col 7
            [___,              k!(CapsLock),     ___,              ___,              k!(F1),           k!(F2),           k!(F3),           ___],
            [___,              ___,              ___,              XXX,              XXX,              ___,              ___,              ___],
            [k!(F4),           ___,              k!(F5),           ___,              k!(F6),           k!(F7),           ___,              ___],
            [BOOT,             ___,              ___,              ___,              ___,              ___,              ___,              ___],
            [k!(F8),           ___,              k!(F9),           k!(F10),          k!(F11),          ___,              ___,              k!(Up)],
            [k!(Home),         k!(End),          ___,              ___,              ___,              ___,              ___,              ___],
            [k!(F12),          k!(Insert),       k!(Delete),       k!(PageUp),       ___,              ___,              ___,              XXX],
            [___,              XXX,              ___,              ___,              k!(Down),         k!(PageDown),     k!(Right),        k!(Left)],
            [XXX,              XXX,              XXX,              XXX,              XXX,              XXX,              XXX,              XXX]
        ]),
    ]
}
