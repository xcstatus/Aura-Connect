//! Local VT palette via OSC 4 (xterm). Fed into libghostty through `write_vt` — does not go to SSH.

fn push_osc4(buf: &mut Vec<u8>, index: u8, color: &str) {
    buf.extend_from_slice(b"\x1b]4;");
    buf.extend_from_slice(index.to_string().as_bytes());
    buf.push(b';');
    buf.extend_from_slice(color.as_bytes());
    buf.extend_from_slice(b"\x1b\\");
}

/// Reset dynamic colors to implementation default (xterm `OSC 104`).
fn push_palette_reset(buf: &mut Vec<u8>) {
    buf.extend_from_slice(b"\x1b]104\x07");
}

/// VT bytes to apply the named scheme to the **local** terminal emulator only.
pub fn scheme_vt_bytes(name: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(512);
    match name {
        // 重置到实现默认值
        "Default" => {
            push_palette_reset(&mut buf);
        }
        // Terminal Dark - 经典深色终端
        "TerminalDark" => {
            push_palette_reset(&mut buf);
            push_osc4(&mut buf, 0, "#0A0A0F"); // Black
            push_osc4(&mut buf, 1, "#FF3B30"); // Red
            push_osc4(&mut buf, 2, "#00FF80"); // Green
            push_osc4(&mut buf, 3, "#FFCC00"); // Yellow
            push_osc4(&mut buf, 4, "#007AFF"); // Blue
            push_osc4(&mut buf, 5, "#AF52DE"); // Magenta
            push_osc4(&mut buf, 6, "#5AC8FA"); // Cyan
            push_osc4(&mut buf, 7, "#E6E6E6"); // White
            push_osc4(&mut buf, 8, "#4C4C4C"); // Bright Black
            push_osc4(&mut buf, 9, "#FF6B6B"); // Bright Red
            push_osc4(&mut buf, 10, "#6BFF9E"); // Bright Green
            push_osc4(&mut buf, 11, "#FFDD66"); // Bright Yellow
            push_osc4(&mut buf, 12, "#66B3FF"); // Bright Blue
            push_osc4(&mut buf, 13, "#DA8BFF"); // Bright Magenta
            push_osc4(&mut buf, 14, "#8BE8FF"); // Bright Cyan
            push_osc4(&mut buf, 15, "#FFFFFF"); // Bright White
        }
        // Terminal Light - 浅色终端
        "TerminalLight" => {
            push_palette_reset(&mut buf);
            push_osc4(&mut buf, 0, "#FFFFFF"); // Black (白色作为背景)
            push_osc4(&mut buf, 1, "#D12821"); // Red
            push_osc4(&mut buf, 2, "#26A269"); // Green
            push_osc4(&mut buf, 3, "#B58600"); // Yellow
            push_osc4(&mut buf, 4, "#1C6CD1"); // Blue
            push_osc4(&mut buf, 5, "#A03BBE"); // Magenta
            push_osc4(&mut buf, 6, "#1C9BA8"); // Cyan
            push_osc4(&mut buf, 7, "#1C1C1E"); // White (深色作为前景)
            push_osc4(&mut buf, 8, "#8C8C8C"); // Bright Black
            push_osc4(&mut buf, 9, "#E54D48"); // Bright Red
            push_osc4(&mut buf, 10, "#48C78E"); // Bright Green
            push_osc4(&mut buf, 11, "#D4A800"); // Bright Yellow
            push_osc4(&mut buf, 12, "#3490E8"); // Bright Blue
            push_osc4(&mut buf, 13, "#C46DD9"); // Bright Magenta
            push_osc4(&mut buf, 14, "#3DB5C1"); // Bright Cyan
            push_osc4(&mut buf, 15, "#3C3C3E"); // Bright White
        }
        "Nord" => {
            push_palette_reset(&mut buf);
            // Nord palette (16 ANSI) — common SSH / TUI expectations
            push_osc4(&mut buf, 0, "#2E3440");
            push_osc4(&mut buf, 1, "#BF616A");
            push_osc4(&mut buf, 2, "#A3BE8C");
            push_osc4(&mut buf, 3, "#EBCB8B");
            push_osc4(&mut buf, 4, "#81A1C1");
            push_osc4(&mut buf, 5, "#B48EAD");
            push_osc4(&mut buf, 6, "#88C0D0");
            push_osc4(&mut buf, 7, "#ECEFF4");
            push_osc4(&mut buf, 8, "#4C566A");
            push_osc4(&mut buf, 9, "#BF616A");
            push_osc4(&mut buf, 10, "#A3BE8C");
            push_osc4(&mut buf, 11, "#EBCB8B");
            push_osc4(&mut buf, 12, "#81A1C1");
            push_osc4(&mut buf, 13, "#B48EAD");
            push_osc4(&mut buf, 14, "#8FBCBB");
            push_osc4(&mut buf, 15, "#ECEFF4");
        }
        "Solarized" => {
            push_palette_reset(&mut buf);
            // Solarized Dark — ANSI 0–15
            push_osc4(&mut buf, 0, "#073642");
            push_osc4(&mut buf, 1, "#DC322F");
            push_osc4(&mut buf, 2, "#859900");
            push_osc4(&mut buf, 3, "#B58900");
            push_osc4(&mut buf, 4, "#268BD2");
            push_osc4(&mut buf, 5, "#D33682");
            push_osc4(&mut buf, 6, "#2AA198");
            push_osc4(&mut buf, 7, "#EEE8D5");
            push_osc4(&mut buf, 8, "#002B36");
            push_osc4(&mut buf, 9, "#CB4B16");
            push_osc4(&mut buf, 10, "#586E75");
            push_osc4(&mut buf, 11, "#657B83");
            push_osc4(&mut buf, 12, "#839496");
            push_osc4(&mut buf, 13, "#6C71C4");
            push_osc4(&mut buf, 14, "#93A1A1");
            push_osc4(&mut buf, 15, "#FDF6E3");
        }
        "Monokai" => {
            push_palette_reset(&mut buf);
            push_osc4(&mut buf, 0, "#272822");
            push_osc4(&mut buf, 1, "#F92672");
            push_osc4(&mut buf, 2, "#A6E22E");
            push_osc4(&mut buf, 3, "#F4BF75");
            push_osc4(&mut buf, 4, "#66D9EF");
            push_osc4(&mut buf, 5, "#AE81FF");
            push_osc4(&mut buf, 6, "#A1EFE4");
            push_osc4(&mut buf, 7, "#F8F8F2");
            push_osc4(&mut buf, 8, "#75715E");
            push_osc4(&mut buf, 9, "#F92672");
            push_osc4(&mut buf, 10, "#A6E22E");
            push_osc4(&mut buf, 11, "#F4BF75");
            push_osc4(&mut buf, 12, "#66D9EF");
            push_osc4(&mut buf, 13, "#AE81FF");
            push_osc4(&mut buf, 14, "#A1EFE4");
            push_osc4(&mut buf, 15, "#F9F8F5");
        }
        // Gruvbox Dark - 暖色复古
        "Gruvbox" => {
            push_palette_reset(&mut buf);
            push_osc4(&mut buf, 0, "#282828"); // Black
            push_osc4(&mut buf, 1, "#CC241D"); // Red
            push_osc4(&mut buf, 2, "#98971A"); // Green
            push_osc4(&mut buf, 3, "#D79921"); // Yellow
            push_osc4(&mut buf, 4, "#45858B"); // Blue
            push_osc4(&mut buf, 5, "#B16286"); // Magenta
            push_osc4(&mut buf, 6, "#689D96"); // Cyan
            push_osc4(&mut buf, 7, "#EBDBB2"); // White
            push_osc4(&mut buf, 8, "#928374"); // Bright Black
            push_osc4(&mut buf, 9, "#FB4934"); // Bright Red
            push_osc4(&mut buf, 10, "#B8BB26"); // Bright Green
            push_osc4(&mut buf, 11, "#FABB2F"); // Bright Yellow
            push_osc4(&mut buf, 12, "#83A598"); // Bright Blue
            push_osc4(&mut buf, 13, "#D3869B"); // Bright Magenta
            push_osc4(&mut buf, 14, "#8ECCC6"); // Bright Cyan
            push_osc4(&mut buf, 15, "#FFFFFF"); // Bright White
        }
        // Dracula - 紫色主题
        "Dracula" => {
            push_palette_reset(&mut buf);
            push_osc4(&mut buf, 0, "#282A36"); // Black
            push_osc4(&mut buf, 1, "#FF557E"); // Red
            push_osc4(&mut buf, 2, "#50FA7B"); // Green
            push_osc4(&mut buf, 3, "#FFB86C"); // Yellow
            push_osc4(&mut buf, 4, "#BD93F9"); // Blue
            push_osc4(&mut buf, 5, "#FF79C6"); // Magenta
            push_osc4(&mut buf, 6, "#8BF9F1"); // Cyan
            push_osc4(&mut buf, 7, "#F8F8F2"); // White
            push_osc4(&mut buf, 8, "#62757E"); // Bright Black
            push_osc4(&mut buf, 9, "#FF7A93"); // Bright Red
            push_osc4(&mut buf, 10, "#69FF94"); // Bright Green
            push_osc4(&mut buf, 11, "#FFCC8C"); // Bright Yellow
            push_osc4(&mut buf, 12, "#C9A2FC"); // Bright Blue
            push_osc4(&mut buf, 13, "#FF92D0"); // Bright Magenta
            push_osc4(&mut buf, 14, "#A4FFF9"); // Bright Cyan
            push_osc4(&mut buf, 15, "#FFFFFF"); // Bright White
        }
        _ => {
            push_palette_reset(&mut buf);
        }
    }
    buf
}
