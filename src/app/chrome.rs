use iced::Theme;
use iced::theme::Base;
use iced::widget::container;

use super::state::IcedState;
use crate::theme::DesignTokens;

/// Extra insets when using a macOS unified (full-size content) title bar so the first row
/// clears the traffic lights and title/toolbar band.
#[cfg(target_os = "macos")]
pub(crate) fn unified_titlebar_padding() -> iced::padding::Padding {
    iced::padding::Padding::ZERO
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn unified_titlebar_padding() -> iced::padding::Padding {
    iced::padding::Padding::ZERO
}

pub(crate) use crate::theme::layout::TOP_BAR_HEIGHT as TOP_BAR_H;
pub(crate) use crate::theme::layout::TAB_CHIP_WIDTH;

/// 操作按钮尺寸（可交互区域和图标尺寸）
pub(crate) const TOP_ICON_BTN: f32 = 36.0;

/// 控制组宽度
#[cfg(target_os = "macos")]
pub(crate) const TOP_CONTROL_GROUP_W: f32 = 36.0;
#[cfg(not(target_os = "macos"))]
pub(crate) const TOP_CONTROL_GROUP_W: f32 = 36.0 * 3.0 + 6.0;

/// 操作组宽度：2个按钮 36px + 间距
pub(crate) const TAB_ACTION_GROUP_W: f32 = 36.0 * 2.0 + 6.0;

/// `container(top_bar_row).padding([0, x])` 左右各 inset
pub(crate) const TOP_BAR_EDGE_PAD: f32 = 12.0;

#[cfg(target_os = "macos")]
pub(crate) const TRAFFIC_LIGHT_BAND_W: f32 = 76.0;
#[cfg(not(target_os = "macos"))]
pub(crate) const TRAFFIC_LIGHT_BAND_W: f32 = 0.0;

/// macOS 红绿灯按钮直径（约 12-13px），用于红绿灯垂直居中计算
pub(crate) const TRAFFIC_LIGHT_DIAMETER: f32 = 12.0;

/// 计算标签栏可用宽度
pub(crate) fn tab_strip_width(window_width: f32) -> f32 {
    if !window_width.is_finite() || window_width <= 0.0 {
        return 0.0;
    }
    let chrome_w = window_width;
    let row_inner = chrome_w - TOP_BAR_EDGE_PAD * 2.0;
    let w = row_inner
        - TRAFFIC_LIGHT_BAND_W
        - TAB_ACTION_GROUP_W
        - TOP_CONTROL_GROUP_W;
    w.max(0.0)
}

/// App chrome behind the title row: solid so only the top bar reads as "glass".
/// 使用 DesignTokens 获取颜色，支持主题切换。
pub(crate) fn main_chrome_style(tokens: DesignTokens) -> impl Fn(&Theme) -> container::Style + 'static {
    let bg = tokens.bg_primary;
    move |_: &Theme| {
        container::Style::default().background(bg)
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn app_chrome_style(_state: &IcedState, theme: &Theme) -> iced::theme::Style {
    // 根视图铺满不透明底，避免圆角窗口 + 透明 client 在左上/右上露出桌面
    let mut base = theme.base();
    base.background_color = theme.extended_palette().background.base.color;
    base
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn app_chrome_style(_state: &IcedState, theme: &Theme) -> iced::theme::Style {
    theme.base()
}
