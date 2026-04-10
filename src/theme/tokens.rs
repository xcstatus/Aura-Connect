//! 配色方案系统：统一控制 UI 颜色和终端颜色。
//!
//! 核心数据结构：
//! - `ColorScheme`: 完整配色方案，包含 UI 颜色 + 终端颜色（可独立覆盖）
//! - `DesignTokens`: UI 专用色票（从 ColorScheme 派生）
//!
//! 终端颜色衍生规则：
//! - `term_bg` → 衍生自 `bg_primary`（可覆盖）
//! - `term_fg` → 衍生自 `text_primary`（可覆盖）
//! - `term_cursor` → 衍生自 `accent_base`（可覆盖）
//! - `term_cursor_bg` → 衍生自 `accent_base`（可覆盖）
//! - `term_selection_bg` → 衍生自 `selection_bg`（可覆盖）
//! - `term_selection_fg` → 衍生自 `text_primary`（可覆盖）
//! - `term_scrollbar_bg` → 衍生自 `surface_1`（可覆盖）
//! - `term_scrollbar_fg` → 衍生自 `surface_3`（可覆盖）
//!
//! 终端颜色协议（VT 字节）：
//! - OSC 4 (index;color): ANSI 0-15
//! - OSC 10: Foreground (FG, 前景色)
//! - OSC 11: Background (BG, 背景色)
//! - OSC 12: Cursor (CU, 光标色)
//! - OSC 17: Cursor color (块光标前景色, CA)
//! - OSC 19: Selection background (SB)
//! - OSC 20: Selection foreground (SF)

use iced::Color;

/// 完整配色方案：UI 颜色 + 终端颜色（可独立覆盖）
///
/// 每个预设方案同时定义：
/// - UI 颜色：控制整个应用窗口框架的视觉风格
/// - 终端颜色：通过 VT 字节发送给终端模拟器
///
/// 终端颜色使用 `Option<Color>` 表示：
/// - `None` = 自动衍生自对应的 UI 颜色
/// - `Some(color)` = 使用自定义值覆盖衍生
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorScheme {
    /// 配色方案 ID（唯一标识）
    pub id: &'static str,
    /// 配色方案显示名称
    pub name: &'static str,

    // === UI 颜色 ===
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_header: Color,
    pub bg_popup: Color,
    pub surface_1: Color,
    pub surface_2: Color,
    pub surface_3: Color,
    pub tab_active_bg: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_disabled: Color,
    pub border_default: Color,
    pub border_subtle: Color,
    pub accent_base: Color,
    pub accent_hover: Color,
    pub accent_active: Color,
    /// Primary 实体按钮上的标签色（暗色绿钮为黑，浅色主题多为白）
    pub on_accent_label: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub line_highlight: Color,
    /// UI 层选中区域背景（用于非终端区域）
    pub selection_bg: Color,

    // === 终端颜色（可独立覆盖） ===
    /// BG - 终端背景色 (OSC 11)，None = 衍生自 bg_primary
    pub term_bg: Option<Color>,
    /// FG - 终端前景色 (OSC 10)，None = 衍生自 text_primary
    pub term_fg: Option<Color>,
    /// CU - 终端光标色 (OSC 12)，None = 衍生自 accent_base
    pub term_cursor: Option<Color>,
    /// CA - 块光标前景色 (OSC 17)，None = 衍生自 accent_base
    pub term_cursor_bg: Option<Color>,
    /// SB - 选中区域背景色 (OSC 19)，None = 衍生自 selection_bg
    pub term_selection_bg: Option<Color>,
    /// SF - 选中区域前景色 (OSC 20)，None = 衍生自 text_primary
    pub term_selection_fg: Option<Color>,
    /// 终端滚动条背景，None = 衍生自 surface_1
    pub term_scrollbar_bg: Option<Color>,
    /// 终端滚动条前景（滑块），None = 衍生自 surface_3
    pub term_scrollbar_fg: Option<Color>,

    // === ANSI 16 色调色板 ===
    /// ANSI 颜色索引 0-15
    pub ansi: [Color; 16],
}

impl ColorScheme {
    /// Terminal Dark - 经典深色终端
    pub const fn terminal_dark() -> Self {
        let ansi = [
            Color::from_rgb8(0x0A, 0x0A, 0x0F), // 0: Black
            Color::from_rgb8(0xFF, 0x3B, 0x30), // 1: Red
            Color::from_rgb8(0x00, 0xFF, 0x80), // 2: Green
            Color::from_rgb8(0xFF, 0xCC, 0x00), // 3: Yellow
            Color::from_rgb8(0x00, 0x7A, 0xFF), // 4: Blue
            Color::from_rgb8(0xAF, 0x52, 0xDE), // 5: Magenta
            Color::from_rgb8(0x5A, 0xC8, 0xFA), // 6: Cyan
            Color::from_rgb8(0xE6, 0xE6, 0xE6), // 7: White
            Color::from_rgb8(0x4C, 0x4C, 0x4C), // 8: Bright Black
            Color::from_rgb8(0xFF, 0x6B, 0x6B), // 9: Bright Red
            Color::from_rgb8(0x6B, 0xFF, 0x9E), // 10: Bright Green
            Color::from_rgb8(0xFF, 0xDD, 0x66), // 11: Bright Yellow
            Color::from_rgb8(0x66, 0xB3, 0xFF), // 12: Bright Blue
            Color::from_rgb8(0xDA, 0x8B, 0xFF), // 13: Bright Magenta
            Color::from_rgb8(0x8B, 0xE8, 0xFF), // 14: Bright Cyan
            Color::from_rgb8(0xFF, 0xFF, 0xFF), // 15: Bright White
        ];
        Self {
            id: "terminal_dark",
            name: "Terminal Dark",
            // UI 颜色 - 经典深色
            bg_primary: Color::from_rgb8(0x1C, 0x1C, 0x1E),
            bg_secondary: Color::from_rgb8(0x23, 0x23, 0x26),
            bg_header: Color::from_rgb8(0x28, 0x28, 0x2D),
            bg_popup: Color::from_rgb8(0x2C, 0x2C, 0x2E),
            surface_1: Color::from_rgb8(0x2C, 0x2C, 0x2E),
            surface_2: Color::from_rgb8(0x3A, 0x3A, 0x3C),
            surface_3: Color::from_rgb8(0x48, 0x48, 0x4A),
            tab_active_bg: Color::BLACK,
            text_primary: Color::WHITE,
            text_secondary: Color::from_rgb8(0xA0, 0xA0, 0xA5),
            text_disabled: Color::from_rgb8(0x6C, 0x6C, 0x70),
            border_default: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
            border_subtle: Color::from_rgba(1.0, 1.0, 1.0, 0.04),
            accent_base: Color::from_rgb8(0x00, 0xFF, 0x80),
            accent_hover: Color::from_rgb8(0x33, 0xFF, 0x99),
            accent_active: Color::from_rgb8(0x00, 0xCC, 0x66),
            on_accent_label: Color::BLACK,
            success: Color::from_rgb8(0x00, 0xFF, 0x80),
            warning: Color::from_rgb8(0xFF, 0xCC, 0x00),
            error: Color::from_rgb8(0xFF, 0x3B, 0x30),
            info: Color::from_rgb8(0x5A, 0xC8, 0xFA),
            line_highlight: Color::from_rgba(1.0, 1.0, 1.0, 0.03),
            selection_bg: Color::from_rgba(0.0, 1.0, 0.5, 0.25),
            // 终端颜色 - 使用 Option
            // term_bg 使用纯黑，非衍生
            term_bg: Some(Color::from_rgb8(0x0A, 0x0A, 0x0F)),
            term_fg: None,
            term_cursor: None,
            term_cursor_bg: None,
            term_selection_bg: None,
            term_selection_fg: None,
            term_scrollbar_bg: None,
            term_scrollbar_fg: None,
            ansi,
        }
    }

    /// Terminal Light - 浅色终端
    pub const fn terminal_light() -> Self {
        let ansi = [
            Color::from_rgb8(0xFF, 0xFF, 0xFF), // 0: Black (白色作为背景)
            Color::from_rgb8(0xD1, 0x28, 0x21), // 1: Red
            Color::from_rgb8(0x26, 0xA2, 0x69), // 2: Green
            Color::from_rgb8(0xB5, 0x86, 0x00), // 3: Yellow
            Color::from_rgb8(0x1C, 0x6C, 0xD1), // 4: Blue
            Color::from_rgb8(0xA0, 0x3B, 0xBE), // 5: Magenta
            Color::from_rgb8(0x1C, 0x9B, 0xA8), // 6: Cyan
            Color::from_rgb8(0x1C, 0x1C, 0x1E), // 7: White (深色作为前景)
            Color::from_rgb8(0x8C, 0x8C, 0x8C), // 8: Bright Black
            Color::from_rgb8(0xE5, 0x4D, 0x48), // 9: Bright Red
            Color::from_rgb8(0x48, 0xC7, 0x8E), // 10: Bright Green
            Color::from_rgb8(0xD4, 0xA8, 0x00), // 11: Bright Yellow
            Color::from_rgb8(0x34, 0x90, 0xE8), // 12: Bright Blue
            Color::from_rgb8(0xC4, 0x6D, 0xD9), // 13: Bright Magenta
            Color::from_rgb8(0x3D, 0xB5, 0xC1), // 14: Bright Cyan
            Color::from_rgb8(0x3C, 0x3C, 0x3E), // 15: Bright White
        ];
        Self {
            id: "terminal_light",
            name: "Terminal Light",
            // UI 颜色 - 浅色
            bg_primary: Color::WHITE,
            bg_secondary: Color::from_rgb8(0xFA, 0xFA, 0xFA),
            bg_header: Color::from_rgb8(0xF2, 0xF2, 0xF7),
            bg_popup: Color::from_rgb8(0xF2, 0xF2, 0xF7),
            surface_1: Color::from_rgb8(0xF2, 0xF2, 0xF7),
            surface_2: Color::from_rgb8(0xE5, 0xE5, 0xEA),
            surface_3: Color::from_rgb8(0xD1, 0xD1, 0xD6),
            tab_active_bg: Color::WHITE,
            text_primary: Color::from_rgb8(0x1C, 0x1C, 0x1E),
            text_secondary: Color::from_rgb8(0x6E, 0x6E, 0x73),
            text_disabled: Color::from_rgb8(0xA1, 0xA1, 0xA6),
            border_default: Color::from_rgba(0.0, 0.0, 0.0, 0.08),
            border_subtle: Color::from_rgba(0.0, 0.0, 0.0, 0.04),
            accent_base: Color::from_rgb8(0x00, 0x7A, 0xFF),
            accent_hover: Color::from_rgb8(0x33, 0x95, 0xFF),
            accent_active: Color::from_rgb8(0x00, 0x5F, 0xCC),
            on_accent_label: Color::WHITE,
            success: Color::from_rgb8(0x00, 0xFF, 0x80),
            warning: Color::from_rgb8(0xFF, 0xCC, 0x00),
            error: Color::from_rgb8(0xFF, 0x3B, 0x30),
            info: Color::from_rgb8(0x5A, 0xC8, 0xFA),
            line_highlight: Color::from_rgba(0.0, 0.0, 0.0, 0.03),
            selection_bg: Color::from_rgba(0.0, 0.478, 1.0, 0.25),
            // 终端颜色 - 使用 Option
            term_bg: None,
            term_fg: None,
            term_cursor: None,
            term_cursor_bg: None,
            term_selection_bg: None,
            term_selection_fg: None,
            term_scrollbar_bg: None,
            term_scrollbar_fg: None,
            ansi,
        }
    }

    /// Nord - 北欧蓝调
    pub const fn nord() -> Self {
        let ansi = [
            Color::from_rgb8(0x2E, 0x34, 0x40), // 0: Black
            Color::from_rgb8(0xBF, 0x61, 0x6A), // 1: Red
            Color::from_rgb8(0xA3, 0xBE, 0x8C), // 2: Green
            Color::from_rgb8(0xEB, 0xCB, 0x8B), // 3: Yellow
            Color::from_rgb8(0x81, 0xA1, 0xC1), // 4: Blue
            Color::from_rgb8(0xB4, 0x8E, 0xAD), // 5: Magenta
            Color::from_rgb8(0x88, 0xC0, 0xD0), // 6: Cyan
            Color::from_rgb8(0xEC, 0xEF, 0xF4), // 7: White
            Color::from_rgb8(0x4C, 0x56, 0x6A), // 8: Bright Black
            Color::from_rgb8(0xBF, 0x61, 0x6A), // 9: Bright Red
            Color::from_rgb8(0xA3, 0xBE, 0x8C), // 10: Bright Green
            Color::from_rgb8(0xEB, 0xCB, 0x8B), // 11: Bright Yellow
            Color::from_rgb8(0x81, 0xA1, 0xC1), // 12: Bright Blue
            Color::from_rgb8(0xB4, 0x8E, 0xAD), // 13: Bright Magenta
            Color::from_rgb8(0x8F, 0xBC, 0xBB), // 14: Bright Cyan
            Color::from_rgb8(0xEC, 0xEF, 0xF4), // 15: Bright White
        ];
        Self {
            id: "nord",
            name: "Nord",
            // UI 颜色 - Nord 风格
            bg_primary: Color::from_rgb8(0x2E, 0x34, 0x40),
            bg_secondary: Color::from_rgb8(0x3B, 0x42, 0x4F),
            bg_header: Color::from_rgb8(0x43, 0x4C, 0x5E),
            bg_popup: Color::from_rgb8(0x43, 0x4C, 0x5E),
            surface_1: Color::from_rgb8(0x43, 0x4C, 0x5E),
            surface_2: Color::from_rgb8(0x4C, 0x56, 0x6A),
            surface_3: Color::from_rgb8(0x5E, 0x66, 0x7C),
            tab_active_bg: Color::from_rgb8(0x2E, 0x34, 0x40),
            text_primary: Color::from_rgb8(0xEC, 0xEF, 0xF4),
            text_secondary: Color::from_rgb8(0xD8, 0xDE, 0xE9),
            text_disabled: Color::from_rgb8(0x81, 0xA1, 0xC1),
            border_default: Color::from_rgba(0.878, 0.922, 0.957, 0.10),
            border_subtle: Color::from_rgba(0.878, 0.922, 0.957, 0.06),
            accent_base: Color::from_rgb8(0x88, 0xC0, 0xD0),
            accent_hover: Color::from_rgb8(0x81, 0xA1, 0xC1),
            accent_active: Color::from_rgb8(0x5E, 0x81, 0xAC),
            on_accent_label: Color::from_rgb8(0x2E, 0x34, 0x40),
            success: Color::from_rgb8(0xA3, 0xBE, 0x8C),
            warning: Color::from_rgb8(0xEB, 0xCB, 0x8B),
            error: Color::from_rgb8(0xBF, 0x61, 0x6A),
            info: Color::from_rgb8(0x88, 0xC0, 0xD0),
            line_highlight: Color::from_rgba(0.878, 0.922, 0.957, 0.05),
            selection_bg: Color::from_rgba(0.298, 0.337, 0.416, 0.40),
            // 终端颜色 - 使用 Option
            term_bg: None,
            term_fg: None,
            term_cursor: None,
            term_cursor_bg: None,
            term_selection_bg: None,
            term_selection_fg: None,
            term_scrollbar_bg: None,
            term_scrollbar_fg: None,
            ansi,
        }
    }

    /// Solarized Dark - 护眼暗色
    pub const fn solarized() -> Self {
        let ansi = [
            Color::from_rgb8(0x07, 0x36, 0x42), // 0: Black
            Color::from_rgb8(0xDC, 0x32, 0x2F), // 1: Red
            Color::from_rgb8(0x85, 0x99, 0x00), // 2: Green
            Color::from_rgb8(0xB5, 0x89, 0x00), // 3: Yellow
            Color::from_rgb8(0x26, 0x8B, 0xD2), // 4: Blue
            Color::from_rgb8(0xD3, 0x36, 0x82), // 5: Magenta
            Color::from_rgb8(0x2A, 0xA1, 0x98), // 6: Cyan
            Color::from_rgb8(0xEE, 0xE8, 0xD5), // 7: White
            Color::from_rgb8(0x00, 0x2B, 0x36), // 8: Bright Black
            Color::from_rgb8(0xCB, 0x4B, 0x16), // 9: Bright Red
            Color::from_rgb8(0x58, 0x6E, 0x75), // 10: Bright Green
            Color::from_rgb8(0x65, 0x7B, 0x83), // 11: Bright Yellow
            Color::from_rgb8(0x83, 0x94, 0x96), // 12: Bright Blue
            Color::from_rgb8(0x6C, 0x71, 0xC4), // 13: Bright Magenta
            Color::from_rgb8(0x93, 0xA1, 0xA1), // 14: Bright Cyan
            Color::from_rgb8(0xFD, 0xF6, 0xE3), // 15: Bright White
        ];
        Self {
            id: "solarized",
            name: "Solarized",
            // UI 颜色 - Solarized 风格
            bg_primary: Color::from_rgb8(0x00, 0x2B, 0x36),
            bg_secondary: Color::from_rgb8(0x07, 0x36, 0x42),
            bg_header: Color::from_rgb8(0x0A, 0x3D, 0x4A),
            bg_popup: Color::from_rgb8(0x0A, 0x3D, 0x4A),
            surface_1: Color::from_rgb8(0x0A, 0x3D, 0x4A),
            surface_2: Color::from_rgb8(0x0D, 0x44, 0x52),
            surface_3: Color::from_rgb8(0x14, 0x53, 0x62),
            tab_active_bg: Color::from_rgb8(0x00, 0x2B, 0x36),
            text_primary: Color::from_rgb8(0x83, 0x94, 0x96),
            text_secondary: Color::from_rgb8(0x65, 0x7B, 0x83),
            text_disabled: Color::from_rgb8(0x58, 0x6E, 0x75),
            border_default: Color::from_rgba(0.514, 0.580, 0.588, 0.10),
            border_subtle: Color::from_rgba(0.514, 0.580, 0.588, 0.06),
            accent_base: Color::from_rgb8(0x26, 0x8B, 0xD2),
            accent_hover: Color::from_rgb8(0x2A, 0xA1, 0x98),
            accent_active: Color::from_rgb8(0xD3, 0x36, 0x82),
            on_accent_label: Color::from_rgb8(0x00, 0x2B, 0x36),
            success: Color::from_rgb8(0x85, 0x99, 0x00),
            warning: Color::from_rgb8(0xB5, 0x89, 0x00),
            error: Color::from_rgb8(0xDC, 0x32, 0x2F),
            info: Color::from_rgb8(0x26, 0x8B, 0xD2),
            line_highlight: Color::from_rgba(0.514, 0.580, 0.588, 0.05),
            selection_bg: Color::from_rgba(0.027, 0.212, 0.259, 0.60),
            // 终端颜色 - 使用 Option
            term_bg: None,
            term_fg: None,
            term_cursor: None,
            term_cursor_bg: None,
            term_selection_bg: None,
            term_selection_fg: None,
            term_scrollbar_bg: None,
            term_scrollbar_fg: None,
            ansi,
        }
    }

    /// Monokai - 经典程序员风格
    pub const fn monokai() -> Self {
        let ansi = [
            Color::from_rgb8(0x27, 0x28, 0x22), // 0: Black
            Color::from_rgb8(0xF9, 0x26, 0x72), // 1: Red
            Color::from_rgb8(0xA6, 0xE2, 0x2E), // 2: Green
            Color::from_rgb8(0xF4, 0xBF, 0x75), // 3: Yellow
            Color::from_rgb8(0x66, 0xD9, 0xEF), // 4: Blue
            Color::from_rgb8(0xAE, 0x81, 0xFF), // 5: Magenta
            Color::from_rgb8(0xA1, 0xEF, 0xE4), // 6: Cyan
            Color::from_rgb8(0xF8, 0xF8, 0xF2), // 7: White
            Color::from_rgb8(0x75, 0x71, 0x5E), // 8: Bright Black
            Color::from_rgb8(0xF9, 0x26, 0x72), // 9: Bright Red
            Color::from_rgb8(0xA6, 0xE2, 0x2E), // 10: Bright Green
            Color::from_rgb8(0xF4, 0xBF, 0x75), // 11: Bright Yellow
            Color::from_rgb8(0x66, 0xD9, 0xEF), // 12: Bright Blue
            Color::from_rgb8(0xAE, 0x81, 0xFF), // 13: Bright Magenta
            Color::from_rgb8(0xA1, 0xEF, 0xE4), // 14: Bright Cyan
            Color::from_rgb8(0xF9, 0xF8, 0xF5), // 15: Bright White
        ];
        Self {
            id: "monokai",
            name: "Monokai",
            // UI 颜色 - Monokai 风格
            bg_primary: Color::from_rgb8(0x27, 0x28, 0x22),
            bg_secondary: Color::from_rgb8(0x2D, 0x2E, 0x28),
            bg_header: Color::from_rgb8(0x31, 0x32, 0x2C),
            bg_popup: Color::from_rgb8(0x35, 0x36, 0x30),
            surface_1: Color::from_rgb8(0x35, 0x36, 0x30),
            surface_2: Color::from_rgb8(0x49, 0x48, 0x3E),
            surface_3: Color::from_rgb8(0x75, 0x71, 0x5E),
            tab_active_bg: Color::from_rgb8(0x27, 0x28, 0x22),
            text_primary: Color::from_rgb8(0xF8, 0xF8, 0xF2),
            text_secondary: Color::from_rgb8(0x75, 0x71, 0x5E),
            text_disabled: Color::from_rgb8(0x49, 0x48, 0x3E),
            border_default: Color::from_rgba(0.973, 0.973, 0.949, 0.08),
            border_subtle: Color::from_rgba(0.973, 0.973, 0.949, 0.04),
            accent_base: Color::from_rgb8(0x66, 0xD9, 0xEF),
            accent_hover: Color::from_rgb8(0xA1, 0xEF, 0xE4),
            accent_active: Color::from_rgb8(0xAE, 0x81, 0xFF),
            on_accent_label: Color::from_rgb8(0x27, 0x28, 0x22),
            success: Color::from_rgb8(0xA6, 0xE2, 0x2E),
            warning: Color::from_rgb8(0xF4, 0xBF, 0x75),
            error: Color::from_rgb8(0xF9, 0x26, 0x72),
            info: Color::from_rgb8(0x66, 0xD9, 0xEF),
            line_highlight: Color::from_rgba(0.973, 0.973, 0.949, 0.03),
            selection_bg: Color::from_rgba(0.286, 0.282, 0.243, 0.50),
            // 终端颜色 - 使用 Option
            term_bg: None,
            term_fg: None,
            term_cursor: None,
            term_cursor_bg: None,
            term_selection_bg: None,
            term_selection_fg: None,
            term_scrollbar_bg: None,
            term_scrollbar_fg: None,
            ansi,
        }
    }

    /// Gruvbox Dark - 暖色复古风格
    pub const fn gruvbox() -> Self {
        let ansi = [
            Color::from_rgb8(0x28, 0x28, 0x28), // 0: Black
            Color::from_rgb8(0xCC, 0x24, 0x1D), // 1: Red
            Color::from_rgb8(0x98, 0x97, 0x1A), // 2: Green
            Color::from_rgb8(0xD7, 0x99, 0x21), // 3: Yellow
            Color::from_rgb8(0x45, 0x8A, 0xB3), // 4: Blue
            Color::from_rgb8(0xB1, 0x62, 0x86), // 5: Magenta
            Color::from_rgb8(0x68, 0x9C, 0x96), // 6: Cyan
            Color::from_rgb8(0xEB, 0xDB, 0xB2), // 7: White
            Color::from_rgb8(0x92, 0x83, 0x5E), // 8: Bright Black
            Color::from_rgb8(0xFB, 0x49, 0x34), // 9: Bright Red
            Color::from_rgb8(0xB8, 0xBB, 0x26), // 10: Bright Green
            Color::from_rgb8(0xFA, 0xBD, 0x2F), // 11: Bright Yellow
            Color::from_rgb8(0x83, 0xA5, 0x98), // 12: Bright Blue
            Color::from_rgb8(0xD3, 0x86, 0x9B), // 13: Bright Magenta
            Color::from_rgb8(0x8E, 0xCC, 0xC6), // 14: Bright Cyan
            Color::from_rgb8(0xFF, 0xFF, 0xFF), // 15: Bright White
        ];
        Self {
            id: "gruvbox",
            name: "Gruvbox",
            // UI 颜色 - Gruvbox 风格
            bg_primary: Color::from_rgb8(0x28, 0x28, 0x28),
            bg_secondary: Color::from_rgb8(0x30, 0x30, 0x2A),
            bg_header: Color::from_rgb8(0x35, 0x33, 0x2E),
            bg_popup: Color::from_rgb8(0x3C, 0x3A, 0x32),
            surface_1: Color::from_rgb8(0x3C, 0x3A, 0x32),
            surface_2: Color::from_rgb8(0x50, 0x49, 0x3F),
            surface_3: Color::from_rgb8(0x66, 0x5E, 0x4C),
            tab_active_bg: Color::from_rgb8(0x28, 0x28, 0x28),
            text_primary: Color::from_rgb8(0xEB, 0xDB, 0xB2),
            text_secondary: Color::from_rgb8(0xA8, 0x99, 0x84),
            text_disabled: Color::from_rgb8(0x66, 0x5E, 0x4C),
            border_default: Color::from_rgba(0.922, 0.859, 0.698, 0.10),
            border_subtle: Color::from_rgba(0.922, 0.859, 0.698, 0.06),
            accent_base: Color::from_rgb8(0xFB, 0x49, 0x34),
            accent_hover: Color::from_rgb8(0xFE, 0xA0, 0x4A),
            accent_active: Color::from_rgb8(0xCC, 0x24, 0x1D),
            on_accent_label: Color::from_rgb8(0x28, 0x28, 0x28),
            success: Color::from_rgb8(0x98, 0x97, 0x1A),
            warning: Color::from_rgb8(0xD7, 0x99, 0x21),
            error: Color::from_rgb8(0xCC, 0x24, 0x1D),
            info: Color::from_rgb8(0x83, 0xA5, 0x98),
            line_highlight: Color::from_rgba(0.922, 0.859, 0.698, 0.04),
            selection_bg: Color::from_rgba(0.235, 0.220, 0.214, 0.60),
            // 终端颜色 - 使用 Option
            term_bg: None,
            term_fg: None,
            term_cursor: None,
            term_cursor_bg: None,
            term_selection_bg: None,
            term_selection_fg: None,
            term_scrollbar_bg: None,
            term_scrollbar_fg: None,
            ansi,
        }
    }

    /// Dracula - 紫色主题
    pub const fn dracula() -> Self {
        let ansi = [
            Color::from_rgb8(0x28, 0x2A, 0x36), // 0: Black
            Color::from_rgb8(0xFF, 0x55, 0x7E), // 1: Red
            Color::from_rgb8(0x50, 0xFA, 0x7B), // 2: Green
            Color::from_rgb8(0xFF, 0xB8, 0x6C), // 3: Yellow
            Color::from_rgb8(0xBD, 0x93, 0xF9), // 4: Blue
            Color::from_rgb8(0xFF, 0x79, 0xC6), // 5: Magenta
            Color::from_rgb8(0x8B, 0xF9, 0xF1), // 6: Cyan
            Color::from_rgb8(0xF8, 0xF8, 0xF2), // 7: White
            Color::from_rgb8(0x62, 0x75, 0x7E), // 8: Bright Black
            Color::from_rgb8(0xFF, 0x7A, 0x93), // 9: Bright Red
            Color::from_rgb8(0x69, 0xFF, 0x94), // 10: Bright Green
            Color::from_rgb8(0xFF, 0xCC, 0x8C), // 11: Bright Yellow
            Color::from_rgb8(0xC9, 0xA2, 0xFC), // 12: Bright Blue
            Color::from_rgb8(0xFF, 0x92, 0xD0), // 13: Bright Magenta
            Color::from_rgb8(0xA4, 0xFF, 0xF9), // 14: Bright Cyan
            Color::from_rgb8(0xFF, 0xFF, 0xFF), // 15: Bright White
        ];
        Self {
            id: "dracula",
            name: "Dracula",
            // UI 颜色 - Dracula 风格
            bg_primary: Color::from_rgb8(0x28, 0x2A, 0x36),
            bg_secondary: Color::from_rgb8(0x31, 0x33, 0x40),
            bg_header: Color::from_rgb8(0x34, 0x36, 0x44),
            bg_popup: Color::from_rgb8(0x38, 0x3A, 0x4C),
            surface_1: Color::from_rgb8(0x38, 0x3A, 0x4C),
            surface_2: Color::from_rgb8(0x44, 0x47, 0x5A),
            surface_3: Color::from_rgb8(0x62, 0x75, 0x7E),
            tab_active_bg: Color::from_rgb8(0x28, 0x2A, 0x36),
            text_primary: Color::from_rgb8(0xF8, 0xF8, 0xF2),
            text_secondary: Color::from_rgb8(0x62, 0x75, 0x7E),
            text_disabled: Color::from_rgb8(0x44, 0x47, 0x5A),
            border_default: Color::from_rgba(0.973, 0.973, 0.949, 0.10),
            border_subtle: Color::from_rgba(0.973, 0.973, 0.949, 0.06),
            accent_base: Color::from_rgb8(0xBD, 0x93, 0xF9),
            accent_hover: Color::from_rgb8(0xC9, 0xA2, 0xFC),
            accent_active: Color::from_rgb8(0xFF, 0x79, 0xC6),
            on_accent_label: Color::from_rgb8(0x28, 0x2A, 0x36),
            success: Color::from_rgb8(0x50, 0xFA, 0x7B),
            warning: Color::from_rgb8(0xFF, 0xB8, 0x6C),
            error: Color::from_rgb8(0xFF, 0x55, 0x7E),
            info: Color::from_rgb8(0x8B, 0xF9, 0xF1),
            line_highlight: Color::from_rgba(0.973, 0.973, 0.949, 0.03),
            selection_bg: Color::from_rgb8(0x44, 0x47, 0x5A),
            // 终端颜色 - 使用 Option
            term_bg: None,
            term_fg: None,
            term_cursor: None,
            term_cursor_bg: None,
            term_selection_bg: None,
            term_selection_fg: None,
            term_scrollbar_bg: None,
            term_scrollbar_fg: None,
            ansi,
        }
    }

    /// 根据 ID 获取配色方案
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "terminal_dark" => Some(Self::terminal_dark()),
            "terminal_light" => Some(Self::terminal_light()),
            "nord" => Some(Self::nord()),
            "solarized" => Some(Self::solarized()),
            "monokai" => Some(Self::monokai()),
            "gruvbox" => Some(Self::gruvbox()),
            "dracula" => Some(Self::dracula()),
            // 向后兼容旧 ID
            "TerminalDark" => Some(Self::terminal_dark()),
            "TerminalLight" => Some(Self::terminal_light()),
            "Nord" => Some(Self::nord()),
            "Solarized" => Some(Self::solarized()),
            "Monokai" => Some(Self::monokai()),
            "Gruvbox" => Some(Self::gruvbox()),
            "Dracula" => Some(Self::dracula()),
            _ => None,
        }
    }

    // === 终端颜色衍生方法 ===
    // 如果字段为 Some(color) 则使用该值，否则使用对应的 UI 颜色

    /// 获取生效的终端背景色
    #[inline]
    pub fn effective_term_bg(&self) -> Color {
        self.term_bg.unwrap_or(self.bg_primary)
    }

    /// 获取生效的终端前景色
    #[inline]
    pub fn effective_term_fg(&self) -> Color {
        self.term_fg.unwrap_or(self.text_primary)
    }

    /// 获取生效的终端光标色
    #[inline]
    pub fn effective_term_cursor(&self) -> Color {
        self.term_cursor.unwrap_or(self.accent_base)
    }

    /// 获取生效的块光标背景色
    #[inline]
    pub fn effective_term_cursor_bg(&self) -> Color {
        self.term_cursor_bg.unwrap_or(self.accent_base)
    }

    /// 获取生效的终端选中区域背景色
    #[inline]
    pub fn effective_term_selection_bg(&self) -> Color {
        self.term_selection_bg.unwrap_or(self.selection_bg)
    }

    /// 获取生效的终端选中区域前景色
    #[inline]
    pub fn effective_term_selection_fg(&self) -> Color {
        self.term_selection_fg.unwrap_or(self.text_primary)
    }

    /// 获取生效的终端滚动条背景色
    #[inline]
    pub fn effective_term_scrollbar_bg(&self) -> Color {
        self.term_scrollbar_bg.unwrap_or(self.surface_1)
    }

    /// 获取生效的终端滚动条前景色（滑块）
    #[inline]
    pub fn effective_term_scrollbar_fg(&self) -> Color {
        self.term_scrollbar_fg.unwrap_or(self.surface_3)
    }
}

/// 所有预设配色方案
pub const COLOR_SCHEMES: &[ColorScheme] = &[
    ColorScheme::terminal_dark(),
    ColorScheme::terminal_light(),
    ColorScheme::nord(),
    ColorScheme::solarized(),
    ColorScheme::monokai(),
    ColorScheme::gruvbox(),
    ColorScheme::dracula(),
];

/// 与文档对齐的可编程色票（sRGB + 不透明；半透明边框单独字段）。
///
/// 从 ColorScheme 派生，用于 UI 渲染。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesignTokens {
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_header: Color,
    pub bg_popup: Color,
    pub surface_1: Color,
    pub surface_2: Color,
    pub surface_3: Color,
    pub tab_active_bg: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_disabled: Color,
    pub border_default: Color,
    pub border_subtle: Color,
    pub accent_base: Color,
    pub accent_hover: Color,
    pub accent_active: Color,
    /// Primary 实体按钮上的标签色（暗色绿钮为黑，浅色主题多为白）。
    pub on_accent_label: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub terminal_bg: Color,
    pub terminal_fg: Color,
    pub scrollbar_hover: Color,
    pub scrollbar_active: Color,
    pub line_highlight: Color,
    pub selection_bg: Color,
}

impl DesignTokens {
    /// 从配色方案 ID 获取 DesignTokens
    pub fn for_color_scheme(scheme_id: &str) -> Self {
        if let Some(scheme) = ColorScheme::from_id(scheme_id) {
            Self::from_scheme(&scheme)
        } else {
            Self::terminal_dark()
        }
    }

    /// 从 ColorScheme 转换为 DesignTokens
    ///
    /// 终端颜色使用 effective_* 方法获取衍生值
    pub fn from_scheme(scheme: &ColorScheme) -> Self {
        Self {
            bg_primary: scheme.bg_primary,
            bg_secondary: scheme.bg_secondary,
            bg_header: scheme.bg_header,
            bg_popup: scheme.bg_popup,
            surface_1: scheme.surface_1,
            surface_2: scheme.surface_2,
            surface_3: scheme.surface_3,
            tab_active_bg: scheme.tab_active_bg,
            text_primary: scheme.text_primary,
            text_secondary: scheme.text_secondary,
            text_disabled: scheme.text_disabled,
            border_default: scheme.border_default,
            border_subtle: scheme.border_subtle,
            accent_base: scheme.accent_base,
            accent_hover: scheme.accent_hover,
            accent_active: scheme.accent_active,
            on_accent_label: scheme.on_accent_label,
            success: scheme.success,
            warning: scheme.warning,
            error: scheme.error,
            info: scheme.info,
            // 使用衍生方法获取终端颜色
            terminal_bg: scheme.effective_term_bg(),
            terminal_fg: scheme.effective_term_fg(),
            scrollbar_hover: scheme.effective_term_scrollbar_fg(),
            scrollbar_active: scheme.effective_term_scrollbar_fg(),
            line_highlight: scheme.line_highlight,
            selection_bg: scheme.selection_bg,
        }
    }

    /// Terminal Dark（默认）
    pub const fn terminal_dark() -> Self {
        Self {
            bg_primary: Color::from_rgb8(0x1C, 0x1C, 0x1E),
            bg_secondary: Color::from_rgb8(0x23, 0x23, 0x26),
            bg_header: Color::from_rgb8(0x28, 0x28, 0x2D),
            bg_popup: Color::from_rgb8(0x2C, 0x2C, 0x2E),
            surface_1: Color::from_rgb8(0x2C, 0x2C, 0x2E),
            surface_2: Color::from_rgb8(0x3A, 0x3A, 0x3C),
            surface_3: Color::from_rgb8(0x48, 0x48, 0x4A),
            tab_active_bg: Color::BLACK,
            text_primary: Color::WHITE,
            text_secondary: Color::from_rgb8(0xA0, 0xA0, 0xA5),
            text_disabled: Color::from_rgb8(0x6C, 0x6C, 0x70),
            border_default: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
            border_subtle: Color::from_rgba(1.0, 1.0, 1.0, 0.04),
            accent_base: Color::from_rgb8(0x00, 0xFF, 0x80),
            accent_hover: Color::from_rgb8(0x33, 0xFF, 0x99),
            accent_active: Color::from_rgb8(0x00, 0xCC, 0x66),
            on_accent_label: Color::BLACK,
            success: Color::from_rgb8(0x00, 0xFF, 0x80),
            warning: Color::from_rgb8(0xFF, 0xCC, 0x00),
            error: Color::from_rgb8(0xFF, 0x3B, 0x30),
            info: Color::from_rgb8(0x5A, 0xC8, 0xFA),
            terminal_bg: Color::from_rgb8(0x0A, 0x0A, 0x0F),
            terminal_fg: Color::from_rgb8(0xE6, 0xE6, 0xE6),
            scrollbar_hover: Color::from_rgba(1.0, 1.0, 1.0, 0.10),
            scrollbar_active: Color::from_rgba(1.0, 1.0, 1.0, 0.20),
            line_highlight: Color::from_rgba(1.0, 1.0, 1.0, 0.03),
            selection_bg: Color::from_rgba(0.0, 1.0, 0.5, 0.25),
        }
    }
}

/// 设计令牌辅助函数
impl DesignTokens {
    /// 判断是否为暗色主题
    pub fn is_dark(&self) -> bool {
        self.bg_primary.r * 0.2126 + self.bg_primary.g * 0.7152 + self.bg_primary.b * 0.0722 < 0.45
    }

    /// 获取遮罩层颜色（基于背景色的半透明版本）
    pub fn scrim(&self) -> Color {
        Color::from_rgba(self.bg_primary.r, self.bg_primary.g, self.bg_primary.b, 0.42)
    }

    /// 获取遮罩层颜色（带自定义透明度）
    pub fn scrim_with_alpha(&self, alpha: f32) -> Color {
        Color::from_rgba(self.bg_primary.r, self.bg_primary.g, self.bg_primary.b, alpha)
    }

    /// 层级遮罩层颜色 - 根据弹窗层级调整透明度
    /// level: 0 = 第一层弹窗, 1 = 嵌套弹窗, 以此类推
    /// 每增加一层透明度增加 0.15，上限 0.75
    pub fn layered_scrim(&self, level: usize) -> Color {
        let base_alpha = 0.35_f32;
        let alpha = (base_alpha + level as f32 * 0.15).min(0.75);
        Color::from_rgba(self.bg_primary.r, self.bg_primary.g, self.bg_primary.b, alpha)
    }

    /// 层级遮罩层颜色（带动态透明度 + 层级调整）
    pub fn layered_scrim_with_alpha(&self, alpha: f32, level: usize) -> Color {
        let level_adjustment = level as f32 * 0.1;
        let adjusted_alpha = (alpha - level_adjustment).max(0.2).min(0.85);
        Color::from_rgba(self.bg_primary.r, self.bg_primary.g, self.bg_primary.b, adjusted_alpha)
    }

    /// 终端默认单元格背景（深色，确保文字可见）
    pub fn terminal_default_bg(&self) -> Color {
        Color::from_rgb8(10, 10, 15)
    }
}

/// 颜色辅助函数
pub mod color {
    use iced::Color;

    /// 将 Color 转换为 HEX 字符串（格式: #RRGGBB）
    pub fn to_hex(c: Color) -> String {
        let r = (c.r.clamp(0.0, 1.0) * 255.0) as u8;
        let g = (c.g.clamp(0.0, 1.0) * 255.0) as u8;
        let b = (c.b.clamp(0.0, 1.0) * 255.0) as u8;
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    }

    /// 从 HEX 字符串解析 Color（格式: #RRGGBB 或 #RGB）
    pub fn from_hex(hex: &str) -> Option<Color> {
        let hex = hex.trim_start_matches('#');
        let (r, g, b) = match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                (r, g, b)
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                (r, g, b)
            }
            _ => return None,
        };
        Some(Color::from_rgb8(r, g, b))
    }
}

/// DebugOverlay 专用色票
///
/// 基于 DesignTokens 生成，确保开发者工具与主题风格一致。
/// 同时提供高对比度配色以保证调试信息的可读性。
#[derive(Debug, Clone, Copy)]
pub struct DebugTokens {
    /// 面板背景
    pub bg: Color,
    /// 标题文本
    pub text_title: Color,
    /// 普通文本
    pub text_normal: Color,
    /// 次要/静音文本
    pub text_muted: Color,
    /// 良好状态（绿色）
    pub text_good: Color,
    /// 警告状态（黄色）
    pub text_warn: Color,
    /// 错误状态（红色）
    pub text_error: Color,
    /// 第一个标签页（高亮）
    pub tab_first: Color,
    /// 其他标签页
    pub tab_other: Color,
}

impl DebugTokens {
    /// 基于 DesignTokens 生成 DebugTokens
    pub fn from_design_tokens(tokens: &DesignTokens) -> Self {
        Self {
            bg: Color::from_rgba(
                tokens.bg_primary.r,
                tokens.bg_primary.g,
                tokens.bg_primary.b,
                0.88,
            ),
            text_title: tokens.text_secondary,
            text_normal: tokens.text_primary,
            text_muted: tokens.text_disabled,
            text_good: tokens.success,
            text_warn: tokens.warning,
            text_error: tokens.error,
            tab_first: tokens.accent_base,
            tab_other: tokens.text_secondary,
        }
    }
}
