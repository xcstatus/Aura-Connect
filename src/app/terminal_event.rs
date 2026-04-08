use iced::Point;
use iced::keyboard;
use iced::keyboard::key::Physical;
use iced::keyboard::{Key, Modifiers};
use iced::mouse;

/// Terminal input/output events normalized from Iced events.
///
/// Goal: keep `update.rs` free from VT/protocol branching; map UI events once, then
/// forward to terminal core/controller in a single place.
#[derive(Debug, Clone)]
pub(crate) enum TerminalEvent {
    /// Effective focus for terminal protocol (DECSET 1004), after UI gating.
    FocusChanged(bool),
    /// Clipboard paste text (already read from system clipboard).
    Paste(String),
    /// A keyboard key press with optional committed text.
    KeyPressed {
        key: Key,
        physical_key: Physical,
        modifiers: Modifiers,
        text: Option<String>,
    },
    MouseLeftDown(Point),
    MouseLeftUp(Point),
    MouseMoved(Point),
    MouseRightClick(Point),
    MouseWheel {
        delta: mouse::ScrollDelta,
        at: Point,
    },
}

impl TerminalEvent {
    pub(crate) fn from_keyboard_event(ev: &keyboard::Event) -> Option<Self> {
        match ev {
            keyboard::Event::KeyPressed {
                key,
                physical_key,
                modifiers,
                text,
                ..
            } => Some(TerminalEvent::KeyPressed {
                key: key.clone(),
                physical_key: *physical_key,
                modifiers: *modifiers,
                text: text.as_ref().map(|s| s.to_string()),
            }),
            _ => None,
        }
    }
}
