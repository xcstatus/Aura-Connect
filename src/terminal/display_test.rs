/// Build a deterministic VT payload for terminal display verification.
///
/// Enable injection by setting:
/// - `RUST_SSH_TERM_DISPLAY_TEST=1` (includes combining marks; should force CPU fallback)
/// - `RUST_SSH_TERM_DISPLAY_TEST=2` (omits combining marks; should allow GPU text)
pub fn test_pattern_bytes() -> Vec<u8> {
    let omit_marks = std::env::var("RUST_SSH_TERM_DISPLAY_TEST")
        .ok()
        .as_deref()
        == Some("2");

    let mut lines = vec![
        "\x1b[2J\x1b[H\x1b[0m",
        "==== RustSSH Terminal Display Test ====\r\n",
        "ASCII: The quick brown fox jumps over the lazy dog 0123456789\r\n",
        "Punct: !@#$%^&*()_+-=[]{};:'\",.<>/?\\|`~\r\n",
        "Width: iiiii lllll WWWWW MMMMM 00000 OOOOO\r\n",
        "Mix  : RustSSH 你好 世界 中英混排 ABC123\r\n",
        "Box  : ┌─┬─┐ │终│端│ └─┴─┘\r\n",
        "Color: \x1b[31mRed\x1b[0m \x1b[32mGreen\x1b[0m \x1b[34mBlue\x1b[0m \x1b[1mBold\x1b[0m \x1b[4mUnderline\x1b[0m\r\n",
        "Cursor check line -> type here...\r\n",
    ];

    if !omit_marks {
        lines.insert(
            8,
            "Marks: e\u{301} a\u{308} n\u{303} (combining marks)\r\n",
        );
    }

    lines.concat().into_bytes()
}

#[cfg(test)]
mod tests {
    use super::test_pattern_bytes;

    #[test]
    fn display_test_payload_is_non_empty_and_contains_header() {
        let payload = test_pattern_bytes();
        assert!(!payload.is_empty());
        let s = String::from_utf8_lossy(&payload);
        assert!(s.contains("RustSSH Terminal Display Test"));
        assert!(s.contains("中英混排"));
    }
}
