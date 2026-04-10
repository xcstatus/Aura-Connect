//! Local VT palette via OSC 4/10/11/12/17/19/20 (xterm).
//! Fed into libghostty through `write_vt` - does not go to SSH.
//!
//! VT Protocol:
//! - OSC 4;index;color: ANSI 0-15
//! - OSC 10;color: Foreground (FG)
//! - OSC 11;color: Background (BG)
//! - OSC 12;color: Cursor (CU)
//! - OSC 17;color: Cursor color (CA, block cursor foreground)
//! - OSC 19;color: Selection background (SB)
//! - OSC 20;color: Selection foreground (SF)

use crate::theme::ColorScheme;

/// OSC 4: 设置 ANSI 颜色索引 (0-15)
fn push_osc4(buf: &mut Vec<u8>, index: u8, color: &str) {
    buf.extend_from_slice(b"\x1b]4;");
    buf.extend_from_slice(index.to_string().as_bytes());
    buf.push(b';');
    buf.extend_from_slice(color.as_bytes());
    buf.extend_from_slice(b"\x1b\\");
}

/// OSC 10: 设置前景色 (FG)
fn push_osc10(buf: &mut Vec<u8>, color: &str) {
    buf.extend_from_slice(b"\x1b]10;");
    buf.extend_from_slice(color.as_bytes());
    buf.extend_from_slice(b"\x1b\\");
}

/// OSC 11: 设置背景色 (BG)
fn push_osc11(buf: &mut Vec<u8>, color: &str) {
    buf.extend_from_slice(b"\x1b]11;");
    buf.extend_from_slice(color.as_bytes());
    buf.extend_from_slice(b"\x1b\\");
}

/// OSC 12: 设置光标色 (CU)
fn push_osc12(buf: &mut Vec<u8>, color: &str) {
    buf.extend_from_slice(b"\x1b]12;");
    buf.extend_from_slice(color.as_bytes());
    buf.extend_from_slice(b"\x1b\\");
}

/// OSC 17: 设置块光标前景色 (CA)
fn push_osc17(buf: &mut Vec<u8>, color: &str) {
    buf.extend_from_slice(b"\x1b]17;");
    buf.extend_from_slice(color.as_bytes());
    buf.extend_from_slice(b"\x1b\\");
}

/// OSC 19: 设置选中区域背景色 (SB)
fn push_osc19(buf: &mut Vec<u8>, color: &str) {
    buf.extend_from_slice(b"\x1b]19;");
    buf.extend_from_slice(color.as_bytes());
    buf.extend_from_slice(b"\x1b\\");
}

/// OSC 20: 设置选中区域前景色 (SF)
fn push_osc20(buf: &mut Vec<u8>, color: &str) {
    buf.extend_from_slice(b"\x1b]20;");
    buf.extend_from_slice(color.as_bytes());
    buf.extend_from_slice(b"\x1b\\");
}

/// Reset dynamic colors to implementation default (xterm `OSC 104`).
fn push_palette_reset(buf: &mut Vec<u8>) {
    buf.extend_from_slice(b"\x1b]104\x07");
}

/// 将颜色转换为 HEX（忽略 alpha，仅用于 VT 协议）
fn color_to_hex_ignore_alpha(color: iced::Color) -> String {
    let r = (color.r.clamp(0.0, 1.0) * 255.0) as u8;
    let g = (color.g.clamp(0.0, 1.0) * 255.0) as u8;
    let b = (color.b.clamp(0.0, 1.0) * 255.0) as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

/// VT bytes to apply the color scheme to the **local** terminal emulator only.
pub fn scheme_vt_bytes(scheme: &ColorScheme) -> Vec<u8> {
    let mut buf = Vec::with_capacity(512);

    // 重置 ANSI 0-15
    push_palette_reset(&mut buf);

    // 设置 ANSI 0-15
    for (i, color) in scheme.ansi.iter().enumerate() {
        push_osc4(&mut buf, i as u8, &color_to_hex_ignore_alpha(*color));
    }

    // 设置 FG / BG / CU / CA / SB / SF
    push_osc10(&mut buf, &color_to_hex_ignore_alpha(scheme.term_fg));    // FG
    push_osc11(&mut buf, &color_to_hex_ignore_alpha(scheme.term_bg));    // BG
    push_osc12(&mut buf, &color_to_hex_ignore_alpha(scheme.term_cursor)); // CU
    push_osc17(&mut buf, &color_to_hex_ignore_alpha(scheme.term_cursor_bg)); // CA
    push_osc19(&mut buf, &color_to_hex_ignore_alpha(scheme.term_selection_bg)); // SB
    push_osc20(&mut buf, &color_to_hex_ignore_alpha(scheme.term_selection_fg)); // SF

    buf
}
