use anyhow::Result;

use crate::backend::ssh_session::AsyncSession;
use crate::settings::TerminalSettings;

/// Framework-agnostic terminal input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalKey {
    Enter,
    Backspace,
    Tab,
    Escape,
    Space,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    /// F1..=F25 for libghostty.
    Function(u8),
}

/// Framework-agnostic modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerminalModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub logo: bool,
}

/// Framework-agnostic pointer event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TerminalPointerEvent {
    Press { x: f32, y: f32, button: u8 },
    Release { x: f32, y: f32, button: u8 },
    Move { x: f32, y: f32 },
    Wheel { delta_x: f32, delta_y: f32 },
}

/// Backend boundary: take remote output bytes, transform input events into session bytes.
pub trait TerminalBackend {
    /// Called when remote output arrives (from session).
    fn ingest_output(&mut self, bytes: &[u8]);

    /// Called when UI area changes; backend should resize its model.
    fn resize_cells(&mut self, cols: u16, rows: u16);

    /// Handle a key press/release. Returns bytes that should be sent to remote.
    fn on_key(&mut self, key: TerminalKey, pressed: bool, modifiers: TerminalModifiers) -> Vec<u8>;

    /// Handle text input (IME / composed). Returns bytes that should be sent to remote.
    fn on_text(&mut self, text: &str) -> Vec<u8>;

    /// Optional pointer event handling (selection, wheel).
    fn on_pointer_event(&mut self, _event: TerminalPointerEvent) -> Vec<u8> {
        Vec::new()
    }

    /// Return plain lines snapshot for UI rendering.
    fn snapshot_plain_lines(&mut self) -> Vec<String>;

    fn is_active(&self) -> bool;
}

#[cfg(feature = "ghostty-vt")]
pub struct GhosttyVtBackend {
    vt: Option<crate::backend::ghostty_vt::GhosttyVtTerminal>,
    plain_lines: Vec<String>,
    // We keep these to avoid recreating VT backend with 0 sizes.
    last_cols: u16,
    last_rows: u16,
}

#[cfg(feature = "ghostty-vt")]
impl Default for GhosttyVtBackend {
    fn default() -> Self {
        Self {
            vt: None,
            plain_lines: Vec::new(),
            last_cols: 0,
            last_rows: 0,
        }
    }
}

#[cfg(feature = "ghostty-vt")]
impl GhosttyVtBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_settings(&mut self, settings: &TerminalSettings) {
        self.ensure_created();
        if let Some(vt) = self.vt.as_mut() {
            let _ = vt.resize(self.last_cols.max(1), self.last_rows.max(1));
        }
        let _ = settings;
    }

    fn ensure_created(&mut self) {
        if self.vt.is_none() {
            let cols = self.last_cols.max(1);
            let rows = self.last_rows.max(1);
            self.vt = crate::backend::ghostty_vt::GhosttyVtTerminal::new(cols, rows, 10_000).ok();
        } else if self.last_cols > 0 && self.last_rows > 0 {
            if let Some(vt) = self.vt.as_mut() {
                let _ = vt.resize(self.last_cols, self.last_rows);
            }
        }
    }

    pub fn snapshot_plain_preview(&mut self, max_lines: usize) -> Option<String> {
        self.ensure_created();
        let vt = self.vt.as_mut()?;
        let _ = vt.update_render_state();
        if vt
            .update_dirty_plain_lines_and_clear_dirty(&mut self.plain_lines)
            .is_err()
        {
            return None;
        }
        let total = self.plain_lines.len();
        let start = total.saturating_sub(max_lines);
        Some(self.plain_lines[start..].join("\n"))
    }

    pub fn encode_paste(&mut self, text: &str) -> Vec<u8> {
        self.ensure_created();
        if let Some(vt) = self.vt.as_mut() {
            return vt.encode_paste(text);
        }
        text.as_bytes().to_vec()
    }

    pub fn encode_focus_event(&mut self, focused: bool) -> Vec<u8> {
        self.ensure_created();
        if let Some(vt) = self.vt.as_mut() {
            return vt.encode_focus_event(focused);
        }
        Vec::new()
    }

    pub fn set_keep_selection_highlight(&mut self, keep: bool) {
        let _ = keep;
    }
}

#[cfg(feature = "ghostty-vt")]
impl TerminalBackend for GhosttyVtBackend {
    fn ingest_output(&mut self, bytes: &[u8]) {
        self.ensure_created();
        if let Some(vt) = self.vt.as_mut() {
            vt.write_vt(bytes);
            let _ = vt.update_render_state();
            let _ = vt.update_dirty_plain_lines_and_clear_dirty(&mut self.plain_lines);
        }
    }

    fn resize_cells(&mut self, cols: u16, rows: u16) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        self.last_cols = cols;
        self.last_rows = rows;
        self.ensure_created();
        if let Some(vt) = self.vt.as_mut() {
            let _ = vt.resize(cols, rows);
            let _ = vt.update_render_state();
            let _ = vt.update_dirty_plain_lines_and_clear_dirty(&mut self.plain_lines);
        }
    }

    fn on_key(&mut self, key: TerminalKey, pressed: bool, modifiers: TerminalModifiers) -> Vec<u8> {
        if !pressed {
            return Vec::new();
        }
        if let Some(vt) = self.vt.as_mut() {
            use crate::backend::ghostty_vt::ffi;
            let action = ffi::GhosttyKeyAction_GHOSTTY_KEY_ACTION_PRESS;
            let ghost_key = match key {
                TerminalKey::Enter => Some(ffi::GhosttyKey_GHOSTTY_KEY_ENTER),
                TerminalKey::Backspace => Some(ffi::GhosttyKey_GHOSTTY_KEY_BACKSPACE),
                TerminalKey::Tab => Some(ffi::GhosttyKey_GHOSTTY_KEY_TAB),
                TerminalKey::Escape => Some(ffi::GhosttyKey_GHOSTTY_KEY_ESCAPE),
                TerminalKey::Space => Some(ffi::GhosttyKey_GHOSTTY_KEY_SPACE),
                TerminalKey::ArrowUp => Some(ffi::GhosttyKey_GHOSTTY_KEY_ARROW_UP),
                TerminalKey::ArrowDown => Some(ffi::GhosttyKey_GHOSTTY_KEY_ARROW_DOWN),
                TerminalKey::ArrowLeft => Some(ffi::GhosttyKey_GHOSTTY_KEY_ARROW_LEFT),
                TerminalKey::ArrowRight => Some(ffi::GhosttyKey_GHOSTTY_KEY_ARROW_RIGHT),
                TerminalKey::Home => Some(ffi::GhosttyKey_GHOSTTY_KEY_HOME),
                TerminalKey::End => Some(ffi::GhosttyKey_GHOSTTY_KEY_END),
                TerminalKey::PageUp => Some(ffi::GhosttyKey_GHOSTTY_KEY_PAGE_UP),
                TerminalKey::PageDown => Some(ffi::GhosttyKey_GHOSTTY_KEY_PAGE_DOWN),
                TerminalKey::Insert => Some(ffi::GhosttyKey_GHOSTTY_KEY_INSERT),
                TerminalKey::Delete => Some(ffi::GhosttyKey_GHOSTTY_KEY_DELETE),
                TerminalKey::Function(n) => {
                    if n == 0 || n > 25 {
                        None
                    } else {
                        Some(ffi::GhosttyKey_GHOSTTY_KEY_F1 + (n as u32) - 1)
                    }
                }
            };
            if let Some(gkey) = ghost_key {
                let mut mods: ffi::GhosttyMods = 0;
                if modifiers.shift {
                    mods |= ffi::GHOSTTY_MODS_SHIFT as ffi::GhosttyMods;
                }
                if modifiers.ctrl {
                    mods |= ffi::GHOSTTY_MODS_CTRL as ffi::GhosttyMods;
                }
                if modifiers.alt {
                    mods |= ffi::GHOSTTY_MODS_ALT as ffi::GhosttyMods;
                }
                if modifiers.logo {
                    mods |= ffi::GHOSTTY_MODS_SUPER as ffi::GhosttyMods;
                }
                if let Ok(encoded) = vt.encode_key(action, gkey, mods) {
                    return encoded;
                }
            }
        }
        minimal_key_fallback(key, modifiers)
    }

    fn on_text(&mut self, text: &str) -> Vec<u8> {
        text.as_bytes().to_vec()
    }

    fn snapshot_plain_lines(&mut self) -> Vec<String> {
        self.plain_lines.clone()
    }

    fn is_active(&self) -> bool {
        self.vt.is_some()
    }
}

#[cfg(feature = "ghostty-vt")]
fn minimal_key_fallback(key: TerminalKey, _modifiers: TerminalModifiers) -> Vec<u8> {
    let bytes: Option<&[u8]> = match key {
        TerminalKey::Enter => Some(b"\r"),
        TerminalKey::Backspace => Some(&[0x7f]),
        TerminalKey::Tab => Some(b"\t"),
        TerminalKey::Escape => Some(&[0x1b]),
        TerminalKey::Space => Some(b" "),
        TerminalKey::ArrowUp => Some(b"\x1b[A"),
        TerminalKey::ArrowDown => Some(b"\x1b[B"),
        TerminalKey::ArrowRight => Some(b"\x1b[C"),
        TerminalKey::ArrowLeft => Some(b"\x1b[D"),
        TerminalKey::Home => Some(b"\x1b[H"),
        TerminalKey::End => Some(b"\x1b[F"),
        TerminalKey::PageUp => Some(b"\x1b[5~"),
        TerminalKey::PageDown => Some(b"\x1b[6~"),
        TerminalKey::Insert => Some(b"\x1b[2~"),
        TerminalKey::Delete => Some(b"\x1b[3~"),
        TerminalKey::Function(_) => None,
    };
    bytes.map(|b| b.to_vec()).unwrap_or_default()
}

/// Helper to write bytes to an optional session.
pub fn write_to_session(session: &mut Option<Box<dyn AsyncSession>>, bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() {
        return Ok(());
    }
    if let Some(s) = session.as_mut() {
        s.write_stream(bytes)?;
    }
    Ok(())
}
