//! Terminal key encoding: maps keyboard events to VT/ghostty byte sequences.

use crate::backend::ghostty_vt::ffi;
use super::{TerminalKey, TerminalModifiers};

/// Named keys that still need VT-compatible bytes when libghostty returns empty (no modifiers).
/// **Not** function keys — those must stay on the encoder path only.
pub(crate) fn whitelisted_minimal_named_key_fallback(key: TerminalKey) -> Option<Vec<u8>> {
    let bytes: &[u8] = match key {
        TerminalKey::Enter => b"\r",
        TerminalKey::Backspace => &[0x7f],
        TerminalKey::Tab => b"\t",
        TerminalKey::Escape => b"\x1b",
        TerminalKey::Space => b" ",
        TerminalKey::ArrowUp => b"\x1b[A",
        TerminalKey::ArrowDown => b"\x1b[B",
        TerminalKey::ArrowRight => b"\x1b[C",
        TerminalKey::ArrowLeft => b"\x1b[D",
        TerminalKey::Home => b"\x1b[H",
        TerminalKey::End => b"\x1b[F",
        TerminalKey::PageUp => b"\x1b[5~",
        TerminalKey::PageDown => b"\x1b[6~",
        TerminalKey::Insert => b"\x1b[2~",
        TerminalKey::Delete => b"\x1b[3~",
        TerminalKey::Function(_) => return None,
    };
    Some(bytes.to_vec())
}

#[inline]
pub(crate) fn ghostty_function_key(n: u8) -> Option<ffi::GhosttyKey> {
    if n == 0 || n > 25 {
        return None;
    }
    Some(ffi::GhosttyKey_GHOSTTY_KEY_F1 + (n as u32) - 1)
}

pub(crate) fn code_to_ghostty_key(code: iced::keyboard::key::Code) -> Option<ffi::GhosttyKey> {
    use iced::keyboard::key::Code::*;
    Some(match code {
        KeyA => ffi::GhosttyKey_GHOSTTY_KEY_A,
        KeyB => ffi::GhosttyKey_GHOSTTY_KEY_B,
        KeyC => ffi::GhosttyKey_GHOSTTY_KEY_C,
        KeyD => ffi::GhosttyKey_GHOSTTY_KEY_D,
        KeyE => ffi::GhosttyKey_GHOSTTY_KEY_E,
        KeyF => ffi::GhosttyKey_GHOSTTY_KEY_F,
        KeyG => ffi::GhosttyKey_GHOSTTY_KEY_G,
        KeyH => ffi::GhosttyKey_GHOSTTY_KEY_H,
        KeyI => ffi::GhosttyKey_GHOSTTY_KEY_I,
        KeyJ => ffi::GhosttyKey_GHOSTTY_KEY_J,
        KeyK => ffi::GhosttyKey_GHOSTTY_KEY_K,
        KeyL => ffi::GhosttyKey_GHOSTTY_KEY_L,
        KeyM => ffi::GhosttyKey_GHOSTTY_KEY_M,
        KeyN => ffi::GhosttyKey_GHOSTTY_KEY_N,
        KeyO => ffi::GhosttyKey_GHOSTTY_KEY_O,
        KeyP => ffi::GhosttyKey_GHOSTTY_KEY_P,
        KeyQ => ffi::GhosttyKey_GHOSTTY_KEY_Q,
        KeyR => ffi::GhosttyKey_GHOSTTY_KEY_R,
        KeyS => ffi::GhosttyKey_GHOSTTY_KEY_S,
        KeyT => ffi::GhosttyKey_GHOSTTY_KEY_T,
        KeyU => ffi::GhosttyKey_GHOSTTY_KEY_U,
        KeyV => ffi::GhosttyKey_GHOSTTY_KEY_V,
        KeyW => ffi::GhosttyKey_GHOSTTY_KEY_W,
        KeyX => ffi::GhosttyKey_GHOSTTY_KEY_X,
        KeyY => ffi::GhosttyKey_GHOSTTY_KEY_Y,
        KeyZ => ffi::GhosttyKey_GHOSTTY_KEY_Z,
        Digit0 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_0,
        Digit1 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_1,
        Digit2 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_2,
        Digit3 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_3,
        Digit4 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_4,
        Digit5 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_5,
        Digit6 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_6,
        Digit7 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_7,
        Digit8 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_8,
        Digit9 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_9,
        Backquote => ffi::GhosttyKey_GHOSTTY_KEY_BACKQUOTE,
        Minus => ffi::GhosttyKey_GHOSTTY_KEY_MINUS,
        Equal => ffi::GhosttyKey_GHOSTTY_KEY_EQUAL,
        BracketLeft => ffi::GhosttyKey_GHOSTTY_KEY_BRACKET_LEFT,
        BracketRight => ffi::GhosttyKey_GHOSTTY_KEY_BRACKET_RIGHT,
        Backslash => ffi::GhosttyKey_GHOSTTY_KEY_BACKSLASH,
        Semicolon => ffi::GhosttyKey_GHOSTTY_KEY_SEMICOLON,
        Quote => ffi::GhosttyKey_GHOSTTY_KEY_QUOTE,
        IntlBackslash => ffi::GhosttyKey_GHOSTTY_KEY_INTL_BACKSLASH,
        IntlRo => ffi::GhosttyKey_GHOSTTY_KEY_INTL_RO,
        IntlYen => ffi::GhosttyKey_GHOSTTY_KEY_INTL_YEN,
        Comma => ffi::GhosttyKey_GHOSTTY_KEY_COMMA,
        Period => ffi::GhosttyKey_GHOSTTY_KEY_PERIOD,
        Slash => ffi::GhosttyKey_GHOSTTY_KEY_SLASH,
        Space => ffi::GhosttyKey_GHOSTTY_KEY_SPACE,
        Enter => ffi::GhosttyKey_GHOSTTY_KEY_ENTER,
        Tab => ffi::GhosttyKey_GHOSTTY_KEY_TAB,
        Backspace => ffi::GhosttyKey_GHOSTTY_KEY_BACKSPACE,
        Escape => ffi::GhosttyKey_GHOSTTY_KEY_ESCAPE,
        ArrowDown => ffi::GhosttyKey_GHOSTTY_KEY_ARROW_DOWN,
        ArrowLeft => ffi::GhosttyKey_GHOSTTY_KEY_ARROW_LEFT,
        ArrowRight => ffi::GhosttyKey_GHOSTTY_KEY_ARROW_RIGHT,
        ArrowUp => ffi::GhosttyKey_GHOSTTY_KEY_ARROW_UP,
        Home => ffi::GhosttyKey_GHOSTTY_KEY_HOME,
        End => ffi::GhosttyKey_GHOSTTY_KEY_END,
        PageUp => ffi::GhosttyKey_GHOSTTY_KEY_PAGE_UP,
        PageDown => ffi::GhosttyKey_GHOSTTY_KEY_PAGE_DOWN,
        Insert => ffi::GhosttyKey_GHOSTTY_KEY_INSERT,
        Delete => ffi::GhosttyKey_GHOSTTY_KEY_DELETE,
        Numpad0 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_0,
        Numpad1 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_1,
        Numpad2 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_2,
        Numpad3 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_3,
        Numpad4 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_4,
        Numpad5 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_5,
        Numpad6 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_6,
        Numpad7 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_7,
        Numpad8 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_8,
        Numpad9 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_9,
        NumpadAdd => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_ADD,
        NumpadSubtract => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_SUBTRACT,
        NumpadMultiply => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_MULTIPLY,
        NumpadDivide => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_DIVIDE,
        NumpadDecimal => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_DECIMAL,
        NumpadEqual => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_EQUAL,
        NumpadEnter => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_ENTER,
        _ => return None,
    })
}

#[inline]
pub(crate) fn dec_private_mode(value: u16) -> ffi::GhosttyMode {
    (value & 0x7fff) as ffi::GhosttyMode
}

pub(crate) fn mods_to_ffi(m: TerminalModifiers) -> ffi::GhosttyMods {
    let mut mods: ffi::GhosttyMods = 0;
    if m.shift {
        mods |= ffi::GHOSTTY_MODS_SHIFT as ffi::GhosttyMods;
    }
    if m.ctrl {
        mods |= ffi::GHOSTTY_MODS_CTRL as ffi::GhosttyMods;
    }
    if m.alt {
        mods |= ffi::GHOSTTY_MODS_ALT as ffi::GhosttyMods;
    }
    if m.logo {
        mods |= ffi::GHOSTTY_MODS_SUPER as ffi::GhosttyMods;
    }
    mods
}
