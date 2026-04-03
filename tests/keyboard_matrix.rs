use rust_ssh::settings::TerminalSettings;
use rust_ssh::terminal_core::{TerminalController, TerminalModifiers};

// This is a regression matrix for key encoding invariants (vim/tmux/top friendly).
// It does not try to fully emulate apps; it locks down our encoder's basic guarantees.

#[test]
fn key_encoding_invariants() {
    let settings = TerminalSettings::default();
    let mut term = TerminalController::new(&settings);

    // Arrow keys must encode to ESC-prefixed sequences (common terminal convention).
    for code in [
        iced::keyboard::key::Code::ArrowUp,
        iced::keyboard::key::Code::ArrowDown,
        iced::keyboard::key::Code::ArrowLeft,
        iced::keyboard::key::Code::ArrowRight,
    ] {
        let bytes = term
            .encode_keypress_from_physical(code, TerminalModifiers::default(), None)
            .expect("arrow key must encode");
        assert!(
            bytes.starts_with(b"\x1b"),
            "arrow key should start with ESC: {:?}",
            bytes
        );
    }

    // Ctrl chords with text should produce non-empty sequences.
    let bytes = term
        .encode_keypress_from_physical(
            iced::keyboard::key::Code::KeyC,
            TerminalModifiers {
                ctrl: true,
                ..TerminalModifiers::default()
            },
            Some("c"),
        )
        .expect("ctrl-c should encode");
    assert!(!bytes.is_empty());

    // IME/unidentified committed text should be accepted (no control chars).
    let bytes = term
        .encode_unidentified_text_keypress("hello")
        .or_else(|| term.encode_text_raw_committed_utf8("hello"))
        .expect("unidentified text should encode");
    assert!(!bytes.is_empty());
}

#[test]
fn focus_and_paste_mode_invariants() {
    let mut settings = TerminalSettings::default();
    settings.bracketed_paste = true;
    let mut term = TerminalController::new(&settings);

    // Focus report is emitted only when DECSET 1004 is enabled.
    assert!(
        term.encode_focus_event(true).is_empty(),
        "focus report should be empty before 1004h"
    );
    term.inject_local_output(b"\x1b[?1004h");
    assert!(
        !term.encode_focus_event(true).is_empty(),
        "focus gained should encode after 1004h"
    );
    assert!(
        !term.encode_focus_event(false).is_empty(),
        "focus lost should encode after 1004h"
    );

    // Bracketed paste is emitted only when both setting=true and DECSET 2004 is on.
    let plain = term.encode_paste("abc");
    assert_eq!(plain, b"abc");
    term.inject_local_output(b"\x1b[?2004h");
    let bracketed = term.encode_paste("abc");
    assert!(
        bracketed.starts_with(b"\x1b[200~") && bracketed.ends_with(b"\x1b[201~"),
        "paste should be wrapped when 2004 is enabled"
    );
}
