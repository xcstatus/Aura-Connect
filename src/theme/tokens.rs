//! 设计系统 v3 色卡：Foundation → Surface → Semantic（见 `doc/设计系统_色卡.md`）。

use iced::Color;

/// 应用 UI 主题变体（Iced 窗口级）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RustSshThemeId {
    #[default]
    Dark,
    Light,
    Warm,
}

/// 与文档对齐的可编程色票（sRGB + 不透明；半透明边框单独字段）。
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
    pub fn for_id(id: RustSshThemeId) -> Self {
        match id {
            RustSshThemeId::Dark => Self::dark(),
            RustSshThemeId::Light => Self::light(),
            RustSshThemeId::Warm => Self::warm(),
        }
    }

    /// 🌑 Dark（主力）
    pub const fn dark() -> Self {
        Self {
            bg_primary: Color::from_rgb8(0x1c, 0x1c, 0x1e),
            bg_secondary: Color::from_rgb8(0x23, 0x23, 0x26),
            bg_header: Color::from_rgb8(0x28, 0x28, 0x2d),
            bg_popup: Color::from_rgb8(0x2c, 0x2c, 0x2e),
            surface_1: Color::from_rgb8(0x2c, 0x2c, 0x2e),
            surface_2: Color::from_rgb8(0x3a, 0x3a, 0x3c),
            surface_3: Color::from_rgb8(0x48, 0x48, 0x4a),
            tab_active_bg: Color::BLACK,
            text_primary: Color::WHITE,
            text_secondary: Color::from_rgb8(0xa0, 0xa0, 0xa5),
            text_disabled: Color::from_rgb8(0x6c, 0x6c, 0x70),
            border_default: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
            border_subtle: Color::from_rgba(1.0, 1.0, 1.0, 0.04),
            accent_base: Color::from_rgb8(0x00, 0xff, 0x80),
            accent_hover: Color::from_rgb8(0x33, 0xff, 0x99),
            accent_active: Color::from_rgb8(0x00, 0xcc, 0x66),
            on_accent_label: Color::BLACK,
            success: Color::from_rgb8(0x00, 0xff, 0x80),
            warning: Color::from_rgb8(0xff, 0xcc, 0x00),
            error: Color::from_rgb8(0xff, 0x3b, 0x30),
            info: Color::from_rgb8(0x5a, 0xc8, 0xfa),
            terminal_bg: Color::BLACK,
            terminal_fg: Color::WHITE,
            scrollbar_hover: Color::from_rgba(1.0, 1.0, 1.0, 0.10),
            scrollbar_active: Color::from_rgba(1.0, 1.0, 1.0, 0.20),
            line_highlight: Color::from_rgba(1.0, 1.0, 1.0, 0.03),
            selection_bg: Color::from_rgba(0.0, 1.0, 0.5, 0.25),
        }
    }

    /// ☀️ Light
    pub const fn light() -> Self {
        Self {
            bg_primary: Color::WHITE,
            bg_secondary: Color::from_rgb8(0xfa, 0xfa, 0xfa),
            bg_header: Color::from_rgb8(0xf2, 0xf2, 0xf7),
            bg_popup: Color::from_rgb8(0xf2, 0xf2, 0xf7),
            surface_1: Color::from_rgb8(0xf2, 0xf2, 0xf7),
            surface_2: Color::from_rgb8(0xe5, 0xe5, 0xea),
            surface_3: Color::from_rgb8(0xd1, 0xd1, 0xd6),
            tab_active_bg: Color::WHITE,
            text_primary: Color::from_rgb8(0x1c, 0x1c, 0x1e),
            text_secondary: Color::from_rgb8(0x6e, 0x6e, 0x73),
            text_disabled: Color::from_rgb8(0xa1, 0xa1, 0xa6),
            border_default: Color::from_rgba(0.0, 0.0, 0.0, 0.08),
            border_subtle: Color::from_rgba(0.0, 0.0, 0.0, 0.04),
            accent_base: Color::from_rgb8(0x00, 0x7a, 0xff),
            accent_hover: Color::from_rgb8(0x33, 0x95, 0xff),
            accent_active: Color::from_rgb8(0x00, 0x5f, 0xcc),
            on_accent_label: Color::WHITE,
            success: Color::from_rgb8(0x00, 0xff, 0x80),
            warning: Color::from_rgb8(0xff, 0xcc, 0x00),
            error: Color::from_rgb8(0xff, 0x3b, 0x30),
            info: Color::from_rgb8(0x5a, 0xc8, 0xfa),
            terminal_bg: Color::BLACK,
            terminal_fg: Color::WHITE,
            scrollbar_hover: Color::from_rgba(0.0, 0.0, 0.0, 0.10),
            scrollbar_active: Color::from_rgba(0.0, 0.0, 0.0, 0.20),
            line_highlight: Color::from_rgba(0.0, 0.0, 0.0, 0.03),
            selection_bg: Color::from_rgba(0.0, 0.478, 1.0, 0.25),
        }
    }

    /// 🌙 Warm
    pub const fn warm() -> Self {
        Self {
            bg_primary: Color::from_rgb8(0x2d, 0x2a, 0x26),
            bg_secondary: Color::from_rgb8(0x33, 0x2f, 0x2b),
            bg_header: Color::from_rgb8(0x35, 0x32, 0x2e),
            bg_popup: Color::from_rgb8(0x3a, 0x36, 0x31),
            surface_1: Color::from_rgb8(0x3a, 0x36, 0x31),
            surface_2: Color::from_rgb8(0x4a, 0x44, 0x3e),
            surface_3: Color::from_rgb8(0x5a, 0x52, 0x4b),
            tab_active_bg: Color::from_rgb8(0x2d, 0x2a, 0x26),
            text_primary: Color::from_rgb8(0xd4, 0xbe, 0xab),
            text_secondary: Color::from_rgb8(0xb7, 0xa9, 0x99),
            text_disabled: Color::from_rgb8(0x8a, 0x7d, 0x72),
            border_default: Color::from_rgba(0.831, 0.745, 0.671, 0.12),
            border_subtle: Color::from_rgba(0.831, 0.745, 0.671, 0.06),
            accent_base: Color::from_rgb8(0xe0, 0x99, 0x5e),
            accent_hover: Color::from_rgb8(0xf0, 0xb2, 0x7a),
            accent_active: Color::from_rgb8(0xc6, 0x7c, 0x3a),
            on_accent_label: Color::from_rgb8(0x1c, 0x1c, 0x1e),
            success: Color::from_rgb8(0x00, 0xff, 0x80),
            warning: Color::from_rgb8(0xff, 0xcc, 0x00),
            error: Color::from_rgb8(0xff, 0x3b, 0x30),
            info: Color::from_rgb8(0x5a, 0xc8, 0xfa),
            terminal_bg: Color::BLACK,
            terminal_fg: Color::WHITE,
            scrollbar_hover: Color::from_rgba(0.831, 0.745, 0.671, 0.15),
            scrollbar_active: Color::from_rgba(0.831, 0.745, 0.671, 0.28),
            line_highlight: Color::from_rgba(0.831, 0.745, 0.671, 0.06),
            selection_bg: Color::from_rgba(0.878, 0.6, 0.37, 0.28),
        }
    }
}

/// ANSI 16 色（终端，见 `doc/组件设计规范.md` §3.4）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerminalAnsiTokens {
    pub black: Color,
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub blue: Color,
    pub magenta: Color,
    pub cyan: Color,
    pub white: Color,
}

impl TerminalAnsiTokens {
    pub const CANON: Self = Self {
        black: Color::BLACK,
        red: Color::from_rgb8(0xff, 0x3b, 0x30),
        green: Color::from_rgb8(0x00, 0xff, 0x80),
        yellow: Color::from_rgb8(0xff, 0xcc, 0x00),
        blue: Color::from_rgb8(0x00, 0x7a, 0xff),
        magenta: Color::from_rgb8(0xaf, 0x52, 0xde),
        cyan: Color::from_rgb8(0x5a, 0xc8, 0xfa),
        white: Color::WHITE,
    };
}
