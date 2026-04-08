//! UI 组件辅助函数：遮罩层、样式等。
//!
//! 所有颜色使用 DesignTokens，确保与设计系统一致。

use iced::widget::container;
use iced::Theme;
use crate::app::state::IcedState;
use crate::theme::{DesignTokens, RustSshThemeId};

/// Helper: get DesignTokens for current state theme.
pub fn tokens_for_state(state: &IcedState) -> DesignTokens {
    let theme_id = match state.model.settings.general.theme.as_str() {
        "Light" => RustSshThemeId::Light,
        "Warm" => RustSshThemeId::Warm,
        "GitHub" => RustSshThemeId::GitHub,
        _ => RustSshThemeId::Dark,
    };
    DesignTokens::for_id(theme_id)
}

/// Modal scrim style (用于 container.style())。
/// 基于 tokens.scrim()，闭包捕获背景色。
pub fn modal_scrim_style(tokens: DesignTokens) -> impl Fn(&Theme) -> container::Style + 'static {
    let bg = tokens.scrim();
    move |_: &Theme| {
        container::Style::default().background(bg)
    }
}

/// Modal scrim style with dynamic alpha (for animations)。
pub fn modal_scrim_alpha_fn(alpha: f32) -> impl Fn(&Theme) -> container::Style + 'static {
    let bg = DesignTokens::for_id(RustSshThemeId::Dark).scrim_with_alpha(alpha);
    move |_: &Theme| {
        container::Style::default().background(bg)
    }
}

/// Layered scrim style - adjusts transparency based on modal stack depth.
/// level: 0 = first modal, 1 = second modal (nested), etc.
/// Each additional layer adds 0.15 to alpha, capped at 0.75.
pub fn layered_scrim_style(tokens: DesignTokens, level: usize) -> impl Fn(&Theme) -> container::Style + 'static {
    let bg = tokens.layered_scrim(level);
    move |_: &Theme| {
        container::Style::default().background(bg)
    }
}

/// Layered scrim style with dynamic alpha (for animations + layered effect).
pub fn layered_scrim_alpha(alpha: f32, level: usize) -> impl Fn(&Theme) -> container::Style + 'static {
    let bg = DesignTokens::for_id(RustSshThemeId::Dark).layered_scrim_with_alpha(alpha, level);
    move |_: &Theme| {
        container::Style::default().background(bg)
    }
}

/// Group header style for quick connect panels.
pub fn quick_connect_group_header_style(tokens: DesignTokens) -> impl Fn(&Theme) -> container::Style + 'static {
    let bg = tokens.bg_header;
    move |_: &Theme| {
        container::Style::default().background(bg)
    }
}

/// Top bar material style (用于模态框卡片背景)。
pub fn top_bar_material_style(tokens: DesignTokens) -> impl Fn(&Theme) -> container::Style + 'static {
    let tint = iced::Color::from_rgba(
        tokens.bg_header.r,
        tokens.bg_header.g,
        tokens.bg_header.b,
        0.42,
    );
    let border = iced::Border {
        width: 0.0,
        color: iced::Color::TRANSPARENT,
        radius: Default::default(),
    };
    move |_: &Theme| {
        container::Style {
            background: Some(iced::Background::Color(tint)),
            border,
            ..Default::default()
        }
    }
}
