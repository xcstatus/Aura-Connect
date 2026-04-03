use iced::Element;
use iced::Theme;
use iced::theme::Base;
use iced::widget::{Space, container, row};

use super::state::IcedState;

/// Extra insets when using a macOS unified (full-size content) title bar so the first row
/// clears the traffic lights and title/toolbar band.
#[cfg(target_os = "macos")]
pub(crate) fn unified_titlebar_padding() -> iced::padding::Padding {
    // 与窗口左右对齐，避免右侧留白在圆角窗口上形成「缺口」观感
    iced::padding::Padding::ZERO
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn unified_titlebar_padding() -> iced::padding::Padding {
    iced::padding::Padding::ZERO
}

pub(crate) use crate::theme::layout::TOP_BAR_HEIGHT as TOP_BAR_H;

/// 控制组（设置 + 非 Mac 窗口按钮）固定宽度；内容在组内右对齐。
#[cfg(target_os = "macos")]
pub(crate) const TOP_CONTROL_GROUP_W: f32 = 44.0;
#[cfg(not(target_os = "macos"))]
pub(crate) const TOP_CONTROL_GROUP_W: f32 = 132.0;

pub(crate) const TOP_ICON_BTN: f32 = 28.0;

/// `container(top_bar_row).padding([0, x])` 左右各 inset
pub(crate) const TOP_BAR_EDGE_PAD: f32 = 12.0;

/// 单个标签芯片总宽（与 view 中 `width(Fixed(134))` 一致）
pub(crate) const TAB_CHIP_WIDTH: f32 = 134.0;
pub(crate) const TAB_CHIP_SPACING: f32 = 4.0;
/// 顶栏「操作组」（⚡ / +）固定占位，不参与标签横向滚动
pub(crate) const TAB_ACTION_GROUP_W: f32 = 8.0 + TOP_ICON_BTN + 6.0 + TOP_ICON_BTN + 8.0;

/// 标签横向 [`scrollable`] 的 widget id（滚轮映射、程序化滚动）
pub(crate) const TAB_STRIP_SCROLLABLE_ID: &str = "rustssh-tab-strip-scroll";

#[cfg(target_os = "macos")]
pub(crate) const TRAFFIC_LIGHT_BAND_W: f32 = 76.0;
#[cfg(not(target_os = "macos"))]
pub(crate) const TRAFFIC_LIGHT_BAND_W: f32 = 0.0;

pub(crate) const SCROLL_TO_CONTROL_GUTTER_W: f32 = 6.0;

/// 横向滚动内容最小宽度（仅标签行 + 与操作组之间的竖线，操作组在滚动区外）
pub(crate) fn tab_scroll_content_min_width(tab_count: usize) -> f32 {
    let tabs_w = if tab_count == 0 {
        0.0
    } else {
        tab_count as f32 * TAB_CHIP_WIDTH + (tab_count.saturating_sub(1) as f32) * TAB_CHIP_SPACING
    };
    tabs_w + 1.0
}

/// 标签 `scrollable` 视口可用宽度（逻辑像素）
pub(crate) fn tab_scroll_viewport_width(window_width: f32) -> f32 {
    if !window_width.is_finite() || window_width <= 0.0 {
        return 0.0;
    }
    let p = unified_titlebar_padding();
    let chrome_w = window_width - p.left - p.right;
    let row_inner = chrome_w - TOP_BAR_EDGE_PAD * 2.0;
    let w = row_inner
        - TRAFFIC_LIGHT_BAND_W
        - TAB_ACTION_GROUP_W
        - SCROLL_TO_CONTROL_GUTTER_W
        - TOP_CONTROL_GROUP_W;
    w.max(0.0)
}

/// 标签未溢出时隐藏右侧渐变蒙层
pub(crate) fn tab_scroll_needs_fade(tab_count: usize, window_width: f32) -> bool {
    tab_scroll_content_min_width(tab_count) > tab_scroll_viewport_width(window_width) + 2.0
}

/// 标签栏滚动区右侧蒙层（与终端区 `base` 背景融色）。
pub(crate) fn tab_scroll_right_fade<Message: 'static>() -> Element<'static, Message> {
    let mut fade = row![].spacing(0).align_y(iced::alignment::Vertical::Center);
    for i in 0..10u8 {
        let t = (i + 1) as f32 / 10.0;
        let strip = container(
            Space::new()
                .width(4.0)
                .height(iced::Length::Fixed(TOP_BAR_H - 6.0)),
        )
        .style(move |theme: &Theme| {
            let base = theme.extended_palette().background.base.color;
            let a = t * 0.5;
            container::Style::default()
                .background(iced::Color::from_rgba(base.r, base.g, base.b, a))
        });
        fade = fade.push(strip);
    }
    container(
        container(fade)
            .width(40.0)
            .height(iced::Length::Fixed(TOP_BAR_H))
            .align_x(iced::alignment::Horizontal::Right),
    )
    .width(iced::Length::Fill)
    .height(iced::Length::Fixed(TOP_BAR_H))
    .into()
}

/// 标签栏与操作组之间的竖分割线（紧贴标签区）。
pub(crate) fn top_bar_vertical_rule<Message: 'static>() -> Element<'static, Message> {
    container(
        Space::new()
            .width(1.0)
            .height(iced::Length::Fixed(TOP_BAR_H)),
    )
    .style(|theme: &Theme| {
        let c = theme.extended_palette().background.weak.color;
        container::Style::default().background(c)
    })
    .into()
}

/// Title strip: translucent tint (macOS blur shows through); no border.
pub(crate) fn top_bar_material_style(theme: &Theme) -> container::Style {
    let base = theme.extended_palette().background.strongest.color;
    let tint = iced::Color::from_rgba(base.r, base.g, base.b, 0.42);
    let border = iced::Border {
        width: 0.0,
        ..Default::default()
    };
    container::Style::default().background(tint).border(border)
}

/// App chrome behind the title row: solid so only the top bar reads as “glass”.
pub(crate) fn main_chrome_style(theme: &Theme) -> container::Style {
    let bg = theme.extended_palette().background.base.color;
    container::Style::default().background(bg)
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
