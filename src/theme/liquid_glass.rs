//! Liquid Glass 效果模块 - Apple Liquid Glass 设计语言实现
//!
//! 提供玻璃容器、玻璃按钮等组件的样式函数，支持：
//! - 玻璃着色层级 (subtle, moderate, strong, intense)
//! - 边缘高光效果
//! - 悬停/按压动效
//! - 灯光等级系统
//!
//! 设计规范参考: `doc/设计规范/Liquid Glass 设计规范.md`

use iced::Background;
use iced::Border;
use iced::Color;
use iced::Theme;
use iced::widget::button;
use iced::widget::button::Status;
use iced::widget::container;

use crate::theme::DesignTokens;
use crate::theme::layout;

/// 玻璃容器样式强度枚举
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GlassIntensity {
    /// 弱 - 4% 透明度 (顶栏、次要容器)
    Subtle,
    /// 中等 - 8% 透明度 (卡片、面板)
    Moderate,
    /// 强 - 12% 透明度 (弹窗、浮层)
    Strong,
    /// 极强 - 16% 透明度 (模态框)
    Intense,
}

impl GlassIntensity {
    /// 获取对应的着色颜色
    pub fn color(&self, tokens: &DesignTokens) -> Color {
        match self {
            Self::Subtle => tokens.glass_subtle,
            Self::Moderate => tokens.glass_moderate,
            Self::Strong => tokens.glass_strong,
            Self::Intense => tokens.glass_intense,
        }
    }
}

/// 玻璃容器样式配置
#[derive(Debug, Clone, Copy)]
pub struct GlassContainerConfig {
    /// 玻璃着色强度
    pub intensity: GlassIntensity,
    /// 是否有边框
    pub bordered: bool,
    /// 边框透明度 (当 bordered 为 true 时)
    pub border_alpha: f32,
    /// 圆角半径
    pub radius: f32,
}

impl Default for GlassContainerConfig {
    fn default() -> Self {
        Self {
            intensity: GlassIntensity::Strong,
            bordered: true,
            border_alpha: 0.08,
            radius: layout::RADIUS_POPUP,
        }
    }
}

impl GlassContainerConfig {
    /// 创建模态框配置
    pub fn modal() -> Self {
        Self {
            intensity: GlassIntensity::Intense,
            bordered: true,
            border_alpha: 0.10,
            radius: layout::MODAL_RADIUS,
        }
    }

    /// 创建面板配置
    pub fn panel() -> Self {
        Self {
            intensity: GlassIntensity::Moderate,
            bordered: true,
            border_alpha: 0.08,
            radius: layout::RADIUS_PANEL,
        }
    }

    /// 创建顶栏配置
    pub fn top_bar() -> Self {
        Self {
            intensity: GlassIntensity::Subtle,
            bordered: false,
            border_alpha: 0.0,
            radius: 0.0,
        }
    }

    /// 创建卡片配置
    pub fn card() -> Self {
        Self {
            intensity: GlassIntensity::Strong,
            bordered: true,
            border_alpha: 0.08,
            radius: layout::RADIUS_TAB,
        }
    }
}

/// 生成玻璃容器样式闭包
pub fn glass_container_style(
    tokens: DesignTokens,
    config: GlassContainerConfig,
) -> impl Fn(&Theme) -> container::Style + 'static {
    let intensity_color = config.intensity.color(&tokens);
    let border_color = if config.bordered {
        Color::from_rgba(
            if tokens.is_dark() { 1.0 } else { 0.0 },
            if tokens.is_dark() { 1.0 } else { 0.0 },
            if tokens.is_dark() { 1.0 } else { 0.0 },
            config.border_alpha,
        )
    } else {
        Color::TRANSPARENT
    };

    move |_: &Theme| container::Style {
        background: Some(Background::Color(intensity_color)),
        border: Border {
            color: border_color,
            width: if config.bordered { 1.0 } else { 0.0 },
            radius: config.radius.into(),
        },
        ..Default::default()
    }
}

/// 玻璃边缘高光容器样式
pub fn glass_edge_container_style(
    tokens: DesignTokens,
    config: GlassContainerConfig,
) -> impl Fn(&Theme) -> container::Style + 'static {
    let glass_color = config.intensity.color(&tokens);
    let border_color = if config.bordered {
        Color::from_rgba(
            if tokens.is_dark() { 1.0 } else { 0.0 },
            if tokens.is_dark() { 1.0 } else { 0.0 },
            if tokens.is_dark() { 1.0 } else { 0.0 },
            config.border_alpha,
        )
    } else {
        Color::TRANSPARENT
    };

    move |_: &Theme| container::Style {
        background: Some(Background::Color(glass_color)),
        border: Border {
            color: border_color,
            width: if config.bordered { 1.0 } else { 0.0 },
            radius: config.radius.into(),
        },
        ..Default::default()
    }
}

// ============================================================================
// 玻璃按钮样式
// ============================================================================

/// 玻璃按钮样式配置
#[derive(Debug, Clone, Copy)]
pub struct GlassButtonConfig {
    /// 按钮类型
    pub variant: GlassButtonVariant,
    /// 圆角半径
    pub radius: f32,
    /// 是否显示边框
    pub bordered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GlassButtonVariant {
    /// 主要操作按钮 (实心 accent 填充)
    Primary,
    /// 次要操作按钮 (玻璃背景)
    Secondary,
    /// 图标按钮 (透明背景)
    Icon,
    /// 标签栏按钮 (无背景)
    Tab,
    /// 危险操作按钮 (红色)
    Danger,
}

impl GlassButtonConfig {
    /// 主要操作按钮配置
    pub fn primary() -> Self {
        Self {
            variant: GlassButtonVariant::Primary,
            radius: layout::RADIUS_BUTTON,
            bordered: false,
        }
    }

    /// 次要操作按钮配置
    pub fn secondary() -> Self {
        Self {
            variant: GlassButtonVariant::Secondary,
            radius: layout::RADIUS_BUTTON,
            bordered: true,
        }
    }

    /// 图标按钮配置
    pub fn icon() -> Self {
        Self {
            variant: GlassButtonVariant::Icon,
            radius: layout::RADIUS_BUTTON,
            bordered: false,
        }
    }

    /// 标签栏按钮配置
    pub fn tab() -> Self {
        Self {
            variant: GlassButtonVariant::Tab,
            radius: 0.0,
            bordered: false,
        }
    }

    /// 危险操作按钮配置
    pub fn danger() -> Self {
        Self {
            variant: GlassButtonVariant::Danger,
            radius: layout::RADIUS_BUTTON,
            bordered: false,
        }
    }
}

/// 生成玻璃主要按钮样式
pub fn glass_primary_button_style(
    tokens: DesignTokens,
) -> impl Fn(&Theme, Status) -> button::Style + 'static {
    move |_theme: &Theme, status: Status| {
        let (bg, text, border) = match status {
            Status::Disabled => (
                None,
                with_alpha(tokens.on_accent_label, 0.45),
                Color::TRANSPARENT,
            ),
            Status::Active => (
                Some(Background::Color(tokens.accent_base)),
                tokens.on_accent_label,
                tokens.accent_base,
            ),
            Status::Hovered => (
                Some(Background::Color(tokens.accent_hover)),
                tokens.on_accent_label,
                tokens.accent_hover,
            ),
            Status::Pressed => (
                Some(Background::Color(pressed_from(tokens.accent_hover))),
                tokens.on_accent_label,
                tokens.accent_active,
            ),
        };

        button::Style {
            background: bg,
            text_color: text,
            border: Border {
                color: border,
                width: 1.0,
                radius: layout::RADIUS_BUTTON.into(),
            },
            ..Default::default()
        }
    }
}

/// 生成玻璃次要按钮样式
pub fn glass_secondary_button_style(
    tokens: DesignTokens,
) -> impl Fn(&Theme, Status) -> button::Style + 'static {
    move |_theme: &Theme, status: Status| {
        let bg = match status {
            Status::Disabled => None,
            Status::Active => None,
            Status::Hovered => Some(Background::Color(tokens.glass_strong)),
            Status::Pressed => Some(Background::Color(tokens.glass_moderate)),
        };

        let text = match status {
            Status::Disabled => with_alpha(tokens.text_primary, 0.45),
            _ => tokens.text_primary,
        };

        let border = match status {
            Status::Disabled => Color::TRANSPARENT,
            Status::Active => Color::TRANSPARENT,
            Status::Hovered => Color::TRANSPARENT,
            Status::Pressed => Color::TRANSPARENT,
        };

        button::Style {
            background: bg,
            text_color: text,
            border: Border {
                color: border,
                width: 0.0,
                radius: layout::RADIUS_BUTTON.into(),
            },
            ..Default::default()
        }
    }
}

/// 生成玻璃图标按钮样式
pub fn glass_icon_button_style(
    tokens: DesignTokens,
) -> impl Fn(&Theme, Status) -> button::Style + 'static {
    move |_theme: &Theme, status: Status| {
        let (bg, text) = match status {
            Status::Disabled => (None, with_alpha(tokens.text_secondary, 0.45)),
            Status::Active => (None, tokens.text_primary),
            Status::Hovered => (
                Some(Background::Color(tokens.glass_moderate)),
                tokens.text_primary,
            ),
            Status::Pressed => (
                Some(Background::Color(tokens.glass_subtle)),
                tokens.text_secondary,
            ),
        };

        button::Style {
            background: bg,
            text_color: text,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: layout::RADIUS_BUTTON.into(),
            },
            ..Default::default()
        }
    }
}

/// 生成玻璃标签按钮样式
pub fn glass_tab_button_style(
    tokens: DesignTokens,
) -> impl Fn(&Theme, Status) -> button::Style + 'static {
    move |_theme: &Theme, status: Status| {
        let bg = match status {
            Status::Disabled => None,
            Status::Active => None,
            Status::Hovered => Some(Background::Color(tokens.glass_moderate)),
            Status::Pressed => Some(Background::Color(tokens.glass_subtle)),
        };

        let text = match status {
            Status::Disabled => tokens.text_disabled,
            _ => tokens.text_primary,
        };

        button::Style {
            background: bg,
            text_color: text,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    }
}

/// 生成危险按钮样式
pub fn glass_danger_button_style(
    tokens: DesignTokens,
) -> impl Fn(&Theme, Status) -> button::Style + 'static {
    move |_theme: &Theme, status: Status| {
        let (bg, text) = match status {
            Status::Disabled => (None, with_alpha(Color::WHITE, 0.45)),
            Status::Active => (None, Color::WHITE),
            Status::Hovered => (Some(Background::Color(tokens.error)), Color::WHITE),
            Status::Pressed => (
                Some(Background::Color(pressed_from(tokens.error))),
                Color::WHITE,
            ),
        };

        button::Style {
            background: bg,
            text_color: text,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: layout::RADIUS_BUTTON.into(),
            },
            ..Default::default()
        }
    }
}

// ============================================================================
// 工具函数
// ============================================================================

fn with_alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

fn pressed_from(c: Color) -> Color {
    Color {
        r: (c.r * 0.88).clamp(0.0, 1.0),
        g: (c.g * 0.88).clamp(0.0, 1.0),
        b: (c.b * 0.88).clamp(0.0, 1.0),
        a: (c.a + 0.04).min(1.0),
    }
}

/// 混合两个颜色
pub fn mix_colors(base: Color, overlay: Color, intensity: f32) -> Color {
    let i = intensity.clamp(0.0, 1.0);
    Color {
        r: base.r + (overlay.r - base.r) * i,
        g: base.g + (overlay.g - base.g) * i,
        b: base.b + (overlay.b - base.b) * i,
        a: base.a.max(overlay.a) + (1.0 - base.a) * i * 0.5,
    }
}
