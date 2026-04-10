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
                // #RGB -> #RRGGBB
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

/// 终端配色方案（包含 ANSI 调色板 + 默认前景/背景）
///
/// 用于：
/// - 预设方案选择（Terminal Dark、Terminal Light、Nord 等）
/// - 快速调整（用户自定义背景色、前景色、光标色）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerminalPalette {
    /// 配色方案 ID
    pub id: &'static str,
    /// 配色方案显示名称
    pub name: &'static str,
    /// 默认前景色（终端文字）
    pub fg: Color,
    /// 默认背景色（终端内容区）
    pub bg: Color,
    /// ANSI 16 色调色板
    pub ansi: [Color; 16],
}

impl TerminalPalette {
    /// Terminal Dark - 经典深色终端
    pub const fn dark() -> Self {
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
            id: "TerminalDark",
            name: "Terminal Dark",
            fg: Color::from_rgb8(0xE6, 0xE6, 0xE6),
            bg: Color::from_rgb8(0x0A, 0x0A, 0x0F),
            ansi,
        }
    }

    /// Terminal Light - 浅色终端
    pub const fn light() -> Self {
        let ansi = [
            Color::from_rgb8(0xFF, 0xFF, 0xFF), // 0: Black (在浅色主题中白色作为背景)
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
            id: "TerminalLight",
            name: "Terminal Light",
            fg: Color::from_rgb8(0x1C, 0x1C, 0x1E),
            bg: Color::from_rgb8(0xFF, 0xFF, 0xFF),
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
            id: "Nord",
            name: "Nord",
            fg: Color::from_rgb8(0xD8, 0xDE, 0xE9),
            bg: Color::from_rgb8(0x2E, 0x34, 0x40),
            ansi,
        }
    }

    /// Solarized Dark - 护眼暗色
    pub const fn solarized_dark() -> Self {
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
            id: "Solarized",
            name: "Solarized",
            fg: Color::from_rgb8(0x83, 0x94, 0x96),
            bg: Color::from_rgb8(0x00, 0x2B, 0x36),
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
            id: "Monokai",
            name: "Monokai",
            fg: Color::from_rgb8(0xF8, 0xF8, 0xF2),
            bg: Color::from_rgb8(0x27, 0x28, 0x22),
            ansi,
        }
    }

    /// Gruvbox Dark - 暖色复古风格
    pub const fn gruvbox_dark() -> Self {
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
            id: "Gruvbox",
            name: "Gruvbox",
            fg: Color::from_rgb8(0xEB, 0xDB, 0xB2),
            bg: Color::from_rgb8(0x28, 0x28, 0x28),
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
            id: "Dracula",
            name: "Dracula",
            fg: Color::from_rgb8(0xF8, 0xF8, 0xF2),
            bg: Color::from_rgb8(0x28, 0x2A, 0x36),
            ansi,
        }
    }

    /// 根据 ID 获取配色方案
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "TerminalDark" => Some(Self::dark()),
            "TerminalLight" => Some(Self::light()),
            "Nord" => Some(Self::nord()),
            "Solarized" => Some(Self::solarized_dark()),
            "Monokai" => Some(Self::monokai()),
            "Gruvbox" => Some(Self::gruvbox_dark()),
            "Dracula" => Some(Self::dracula()),
            _ => None,
        }
    }

    /// 获取所有预设方案
    pub fn all_presets() -> [Self; 7] {
        [
            Self::dark(),
            Self::light(),
            Self::nord(),
            Self::solarized_dark(),
            Self::monokai(),
            Self::gruvbox_dark(),
            Self::dracula(),
        ]
    }
}
