//! 设计系统 v3 色卡：Foundation → Surface → Semantic（见 `doc/设计系统_色卡.md`）。

use iced::Color;

/// 应用 UI 主题变体（Iced 窗口级）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RustSshThemeId {
    #[default]
    Dark,
    Light,
    Warm,
    /// GitHub 风格主题
    GitHub,
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
            RustSshThemeId::GitHub => Self::github(),
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

    /// 🐙 GitHub 风格主题
    ///
    /// 基于 GitHub Dark 主题色彩体系，为开发者提供熟悉的界面。
    /// 参考: https://github.com/primer/github-syntax-dark
    pub const fn github() -> Self {
        Self {
            // 背景色 - GitHub Dark
            bg_primary: Color::from_rgb8(0x16, 0x1B, 0x22),     // #161B22
            bg_secondary: Color::from_rgb8(0x0D, 0x11, 0x17),    // #0D1117
            bg_header: Color::from_rgb8(0x21, 0x26, 0x2D),       // #21262D
            bg_popup: Color::from_rgb8(0x21, 0x26, 0x2D),       // #21262D
            surface_1: Color::from_rgb8(0x21, 0x26, 0x2D),      // #21262D
            surface_2: Color::from_rgb8(0x30, 0x36, 0x3D),       // #30363D
            surface_3: Color::from_rgb8(0x48, 0x4F, 0x58),       // #484F58
            tab_active_bg: Color::from_rgb8(0x0D, 0x11, 0x17),   // #0D1117
            // 文本色
            text_primary: Color::from_rgb8(0xE6, 0xED, 0xF3),   // #E6EDF3
            text_secondary: Color::from_rgb8(0x8B, 0x94, 0x9E), // #8B949E
            text_disabled: Color::from_rgb8(0x6E, 0x76, 0x81),  // #6E7681
            // 边框色 - 基于 #484F58 的透明度
            border_default: Color::from_rgba(0.545, 0.580, 0.620, 0.13),
            border_subtle: Color::from_rgba(0.545, 0.580, 0.620, 0.08),
            // Accent 色 - GitHub 绿
            accent_base: Color::from_rgb8(0x23, 0x86, 0x36),     // #238636
            accent_hover: Color::from_rgb8(0x2E, 0xA0, 0x43),   // #2EA043
            accent_active: Color::from_rgb8(0x19, 0x6C, 0x2E),  // #196C2E
            on_accent_label: Color::WHITE,
            // 状态色 - GitHub 风格
            success: Color::from_rgb8(0x3F, 0xB9, 0x50),         // #3FB950
            warning: Color::from_rgb8(0xD2, 0x99, 0x22),         // #D29922
            error: Color::from_rgb8(0xF8, 0x51, 0x49),           // #F85149
            info: Color::from_rgb8(0x58, 0xA6, 0xFF),            // #58A6FF
            // 终端色 - GitHub 深色
            terminal_bg: Color::from_rgb8(0x0D, 0x11, 0x17),    // #0D1117
            terminal_fg: Color::from_rgb8(0xE6, 0xED, 0xF3),    // #E6EDF3
            scrollbar_hover: Color::from_rgba(0.545, 0.580, 0.620, 0.15),
            scrollbar_active: Color::from_rgba(0.545, 0.580, 0.620, 0.25),
            line_highlight: Color::from_rgba(0.545, 0.580, 0.620, 0.08),
            selection_bg: Color::from_rgba(0.220, 0.545, 0.992, 0.25), // #58A6FF @ 25%
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
