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
        "Default" => {
            push_palette_reset(&mut buf);
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
        _ => {
            push_palette_reset(&mut buf);
        }
    }
    buf
}
