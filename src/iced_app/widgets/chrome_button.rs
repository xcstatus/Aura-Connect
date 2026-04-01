//! 统一顶栏 / 主界面按钮：悬停、按下、禁用下的背景与描边；标签文字字号与颜色。
//!
//! Iced 0.14 的 [`button::Status`] 不包含键盘焦点态，焦点环需在可聚焦封装上另行接入。

use iced::widget::button::{self, Status};
use iced::Background;
use iced::Border;
use iced::Color;
use iced::Theme;

use crate::theme::layout;

/// 与主题一致的文字颜色（字号在 `text(..).size(px)` 上与对应 `style_*(px)` 保持一致即可）。
#[derive(Debug, Clone, Copy)]
pub struct ButtonLabel {
    pub color: Color,
}

impl ButtonLabel {
    pub const fn new(color: Color) -> Self {
        Self { color }
    }
}

/// 交互用色调：用于悬停 / 按下时背景与描边混色（可与主题主色叠加）。
#[derive(Debug, Clone, Copy)]
pub struct ButtonTint {
    /// 常态下背景（含透明度）；`None` 表示透明。
    pub idle: Option<Color>,
    /// 悬停背景；未设置时用 `hover_blend` 与 [`mix_surface`] 从 `idle` / 表面色推导。
    pub hover_bg: Option<Color>,
    /// 按下背景；未设置时用 `pressed_blend` 推导。
    pub pressed_bg: Option<Color>,
    /// 无 `hover_bg` 时，与主题弱表面混色的强度（顶栏图标 ~0.12）。
    pub hover_blend: f32,
    /// 无 `pressed_bg` 时的混色强度（图标钮 ~0.22）。
    pub pressed_blend: f32,
    /// 弱描边：`Active` 态若 width>0 则绘淡色环（键盘焦点需另行接入可聚焦控件）。
    pub focus_ring: Border,
}

impl Default for ButtonTint {
    fn default() -> Self {
        Self {
            idle: None,
            hover_bg: None,
            pressed_bg: None,
            hover_blend: 0.14,
            pressed_blend: 0.28,
            focus_ring: Border::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ButtonStyleSpec {
    pub label: ButtonLabel,
    pub tint: ButtonTint,
    /// 圆角；顶栏图标钮常用 4~6。
    pub border_radius: f32,
    /// 极简标签内按钮：悬停/按下不画描边。
    pub borderless: bool,
}

impl ButtonStyleSpec {
    /// 顶栏图标按钮：透明底、弱悬停、按下略深。
    pub fn top_icon(theme: &Theme) -> Self {
        let t = theme.extended_palette();
        let text_color = t.background.base.text;
        Self {
            label: ButtonLabel::new(text_color),
            tint: ButtonTint {
                idle: None,
                hover_bg: None,
                pressed_bg: None,
                hover_blend: 0.12,
                pressed_blend: 0.22,
                focus_ring: Border {
                    color: t.primary.weak.color,
                    width: 1.0,
                    ..Default::default()
                },
            },
            border_radius: layout::RADIUS_BUTTON,
            borderless: true,
        }
    }

    /// 主内容区次要操作（描边弱、浅底）。
    pub fn chrome_secondary(theme: &Theme) -> Self {
        let t = theme.extended_palette();
        let text_color = t.background.base.text;
        Self {
            label: ButtonLabel::new(text_color),
            tint: ButtonTint {
                idle: Some(t.background.weak.color),
                hover_bg: None,
                pressed_bg: None,
                hover_blend: 0.5,
                pressed_blend: 0.72,
                focus_ring: Border {
                    color: t.primary.base.color,
                    width: 1.0,
                    ..Default::default()
                },
            },
            border_radius: layout::RADIUS_BUTTON,
            borderless: false,
        }
    }

    /// 主强调色（连接等）。
    pub fn chrome_primary(theme: &Theme) -> Self {
        let t = theme.extended_palette();
        let pair = t.primary.base;
        let hover = t.primary.strong.color;
        Self {
            label: ButtonLabel::new(pair.text),
            tint: ButtonTint {
                idle: Some(pair.color),
                hover_bg: Some(hover),
                pressed_bg: Some(pressed_from(hover)),
                hover_blend: 0.0,
                pressed_blend: 0.0,
                focus_ring: Border {
                    color: pair.color,
                    width: 1.0,
                    ..Default::default()
                },
            },
            border_radius: layout::RADIUS_BUTTON,
            borderless: false,
        }
    }

    /// 标签条内的文字按钮（透明底、极弱悬停，避免抢了顶部分隔线视觉）。
    pub fn tab_strip(theme: &Theme) -> Self {
        let t = theme.extended_palette();
        Self {
            label: ButtonLabel::new(t.background.base.text),
            tint: ButtonTint {
                idle: None,
                hover_bg: None,
                pressed_bg: None,
                hover_blend: 0.08,
                pressed_blend: 0.16,
                focus_ring: Border::default(),
            },
            border_radius: 0.0,
            borderless: true,
        }
    }
}

/// 顶栏 / 工具条图标钮（`⚡` `+` `⚙` 等）的 `.style(...)`。  
/// `font_size` 仅作文档约定，须与 `text(..).size(..)` 一致。
pub fn style_top_icon(_font_size: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let spec = ButtonStyleSpec::top_icon(theme);
        unified_button_style(theme, status, &spec)
    }
}

/// 标签栏内 `.style(...)`。
pub fn style_tab_strip(_font_size: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let spec = ButtonStyleSpec::tab_strip(theme);
        unified_button_style(theme, status, &spec)
    }
}

/// 主区次要按钮 `.style(...)`（断开、保存、面包屑等）。
pub fn style_chrome_secondary(_font_size: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let spec = ButtonStyleSpec::chrome_secondary(theme);
        unified_button_style(theme, status, &spec)
    }
}

/// 主区主按钮 `.style(...)`（连接等）。
pub fn style_chrome_primary(_font_size: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let spec = ButtonStyleSpec::chrome_primary(theme);
        unified_button_style(theme, status, &spec)
    }
}

/// 统一按钮 [`button::Style`]：悬停 / 按下 / 禁用。
pub fn unified_button_style(
    theme: &Theme,
    status: Status,
    spec: &ButtonStyleSpec,
) -> button::Style {
    let t = theme.extended_palette();
    let base_text = spec.label.color;

    let border_none = || Border {
        color: Color::TRANSPARENT,
        width: 0.0,
        radius: spec.border_radius.into(),
    };

    match status {
        Status::Disabled => button::Style {
            background: spec.tint.idle.map(|c| Background::Color(with_alpha(c, 0.35))),
            text_color: with_alpha(base_text, 0.45),
            border: if spec.borderless {
                border_none()
            } else {
                rounded_border(spec.border_radius, with_alpha(t.background.strong.color, 0.25))
            },
            ..Default::default()
        },
        Status::Active => {
            let bg = spec
                .tint
                .idle
                .map(|c| Background::Color(c))
                .filter(|_| spec.tint.idle.is_some());
            let mut border = rounded_border(spec.border_radius, Color::TRANSPARENT);
            if !spec.borderless
                && spec.tint.focus_ring.width > 0.0
                && spec.tint.focus_ring.color.a > 0.0
            {
                border.color = with_alpha(spec.tint.focus_ring.color, 0.45);
                border.width = spec.tint.focus_ring.width;
            } else if spec.borderless {
                border = border_none();
            }
            button::Style {
                background: bg,
                text_color: base_text,
                border,
                ..Default::default()
            }
        }
        Status::Hovered => {
            let base = spec.tint.idle.unwrap_or(Color::TRANSPARENT);
            let hover = spec
                .tint
                .hover_bg
                .unwrap_or_else(|| mix_surface(theme, base, spec.tint.hover_blend));
            button::Style {
                background: Some(Background::Color(hover)),
                text_color: base_text,
                border: if spec.borderless {
                    border_none()
                } else {
                    rounded_border(spec.border_radius, with_alpha(t.primary.weak.color, 0.35))
                },
                ..Default::default()
            }
        }
        Status::Pressed => {
            let base = spec.tint.idle.unwrap_or(Color::TRANSPARENT);
            let pressed = spec
                .tint
                .pressed_bg
                .unwrap_or_else(|| mix_surface(theme, base, spec.tint.pressed_blend));
            button::Style {
                background: Some(Background::Color(pressed)),
                text_color: base_text,
                border: if spec.borderless {
                    border_none()
                } else {
                    rounded_border(spec.border_radius, with_alpha(t.primary.strong.color, 0.55))
                },
                ..Default::default()
            }
        }
    }
}

fn rounded_border(radius: f32, color: Color) -> Border {
    Border {
        color,
        width: if color.a > 0.0 { 1.0 } else { 0.0 },
        radius: radius.into(),
    }
}

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

fn mix_surface(theme: &Theme, base: Color, intensity: f32) -> Color {
    let t = theme.extended_palette();
    let surface = t.background.weak.color;
    if base.a < 0.02 {
        return with_alpha(surface, intensity.clamp(0.0, 1.0));
    }
    let i = intensity.clamp(0.0, 1.0);
    Color {
        r: base.r + (surface.r - base.r) * i * 0.55,
        g: base.g + (surface.g - base.g) * i * 0.55,
        b: base.b + (surface.b - base.b) * i * 0.55,
        a: base.a.max(0.15) + (1.0 - base.a) * i * 0.9,
    }
}
