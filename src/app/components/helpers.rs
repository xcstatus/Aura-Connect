//! UI 组件辅助函数：遮罩层、样式等。
//!
//! 所有颜色使用 DesignTokens，确保与设计系统一致。

use crate::app::state::IcedState;
use crate::theme::DesignTokens;
use crate::theme::layout;
use iced::Background;
use iced::Border;
use iced::Color;
use iced::Theme;
use iced::widget::container;

/// Helper: get DesignTokens for current state theme.
pub fn tokens_for_state(state: &IcedState) -> DesignTokens {
    DesignTokens::for_color_scheme(&state.model.settings.color_scheme)
}

/// Layered scrim style - adjusts transparency based on modal stack depth.
/// level: 0 = first modal, 1 = second modal (nested), etc.
/// Each additional layer adds 0.15 to alpha, capped at 0.75.
pub fn layered_scrim_style(
    tokens: DesignTokens,
    level: usize,
) -> impl Fn(&Theme) -> container::Style + 'static {
    let bg = tokens.layered_scrim(level);
    move |_: &Theme| container::Style::default().background(bg)
}

/// Layered scrim style with dynamic alpha (for animations + layered effect).
pub fn layered_scrim_alpha(
    alpha: f32,
    level: usize,
) -> impl Fn(&Theme) -> container::Style + 'static {
    let bg = DesignTokens::terminal_dark().layered_scrim_with_alpha(alpha, level);
    move |_: &Theme| container::Style::default().background(bg)
}

/// Top bar material style (用于模态框卡片背景)。
pub fn top_bar_material_style(
    tokens: DesignTokens,
) -> impl Fn(&Theme) -> container::Style + 'static {
    let tint = tokens.scrim();
    let border = iced::Border {
        width: 0.0,
        color: iced::Color::TRANSPARENT,
        radius: Default::default(),
    };
    move |_: &Theme| container::Style {
        background: Some(iced::Background::Color(tint)),
        border,
        ..Default::default()
    }
}

/// 终端区域背景样式（使用 tokens.terminal_bg，确保终端配色独立于 UI 主题）。
pub fn terminal_area_bg_style(
    tokens: DesignTokens,
) -> impl Fn(&Theme) -> container::Style + 'static {
    let bg = tokens.terminal_bg;
    move |_: &Theme| container::Style::default().background(bg)
}

// ============================================================================
// Liquid Glass 容器样式
// ============================================================================

/// Liquid Glass 玻璃容器样式 - 模态框
pub fn lg_modal_container_style(
    tokens: DesignTokens,
) -> impl Fn(&Theme) -> container::Style + 'static {
    move |_: &Theme| container::Style {
        background: Some(Background::Color(tokens.glass_intense)),
        border: Border {
            color: if tokens.is_dark() {
                Color::from_rgba(1.0, 1.0, 1.0, 0.10)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.10)
            },
            width: 1.0,
            radius: layout::MODAL_RADIUS.into(),
        },
        ..Default::default()
    }
}

/// Liquid Glass 玻璃容器样式 - 面板
pub fn lg_panel_container_style(
    tokens: DesignTokens,
) -> impl Fn(&Theme) -> container::Style + 'static {
    move |_: &Theme| container::Style {
        background: Some(Background::Color(tokens.glass_moderate)),
        border: Border {
            color: if tokens.is_dark() {
                Color::from_rgba(1.0, 1.0, 1.0, 0.08)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.08)
            },
            width: 1.0,
            radius: layout::RADIUS_PANEL.into(),
        },
        ..Default::default()
    }
}

/// Liquid Glass 玻璃容器样式 - 卡片
pub fn lg_card_container_style(
    tokens: DesignTokens,
) -> impl Fn(&Theme) -> container::Style + 'static {
    move |_: &Theme| container::Style {
        background: Some(Background::Color(tokens.glass_strong)),
        border: Border {
            color: if tokens.is_dark() {
                Color::from_rgba(1.0, 1.0, 1.0, 0.08)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.08)
            },
            width: 1.0,
            radius: layout::RADIUS_TAB.into(),
        },
        ..Default::default()
    }
}

/// Liquid Glass 玻璃容器样式 - 顶栏
pub fn lg_topbar_container_style(
    tokens: DesignTokens,
) -> impl Fn(&Theme) -> container::Style + 'static {
    move |_: &Theme| container::Style {
        background: Some(Background::Color(tokens.glass_subtle)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

/// Liquid Glass 选中项背景样式 (用于列表选中状态)
pub fn lg_selected_item_style(
    tokens: DesignTokens,
) -> impl Fn(&Theme) -> container::Style + 'static {
    let accent = tokens.accent_base;
    let bg = Color::from_rgba(accent.r, accent.g, accent.b, 0.10);
    move |_: &Theme| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: if tokens.is_dark() {
                Color::from_rgba(1.0, 1.0, 1.0, 0.12)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.12)
            },
            width: 1.0,
            radius: layout::RADIUS_TAB.into(),
        },
        ..Default::default()
    }
}
