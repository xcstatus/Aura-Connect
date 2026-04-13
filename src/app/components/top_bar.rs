use iced::alignment::Alignment;
use iced::widget::{button, column, container, mouse_area, row, scrollable, text, tooltip, Space};
use iced::{Element, Theme};

use crate::app::chrome::{
    TOP_BAR_H, TOP_CONTROL_GROUP_W, TOP_ICON_BTN,
    TRAFFIC_LIGHT_BAND_W, TRAFFIC_LIGHT_DIAMETER, tab_strip_width,
};
use crate::app::components::helpers::tokens_for_state;
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::widgets::chrome_button::{style_tab_strip, style_top_icon};
use crate::theme::icons::{icon_view_with, IconId, IconOptions};
use crate::theme::layout::{TAB_CHIP_MIN_WIDTH, TAB_CHIP_WIDTH, TAB_CLOSE_HIT_W, TAB_CLOSE_ICON_W, TAB_LABEL_CLOSE_SPACING, TAB_CHIP_PAD_H};

/// Build the top bar (tab strip + action buttons + control buttons).
pub(crate) fn top_bar(state: &IcedState, _tick_ms: f32) -> Element<'_, Message> {
    let tokens = tokens_for_state(state);
    let tabs_row = build_tab_strip(state);
    let action_group = build_action_group(state, tokens);
    let control_group = build_control_group(state, tokens);

    // 左侧区域：traffic_light + 标签栏 + 操作按钮组
    let left_area: Element<'_, Message> = {
        let mut left_row = row![].spacing(0);

        #[cfg(target_os = "macos")]
        {
            left_row = left_row.push(
                container(Space::new().height(iced::Length::Fixed(TRAFFIC_LIGHT_DIAMETER)))
                    .width(iced::Length::Fixed(TRAFFIC_LIGHT_BAND_W))
                    .style(top_bar_ambient_style(tokens)),
            );
        }

        left_row = left_row
            .push(tabs_row);

        container(left_row)
            .width(iced::Length::Fill)
            .height(iced::Length::Fixed(TOP_BAR_H))
            .into()
    };

    // 顶栏：左侧区域 + 控制组，无 spacing，直接贴靠
    let top_bar_row = row![left_area, action_group, control_group].spacing(0).align_y(Alignment::Center);

    container(top_bar_row)
        .height(iced::Length::Fixed(TOP_BAR_H))
        .padding(0)
        .style(top_bar_ambient_style(tokens))
        .into()
}

/// 欢迎页专用的简化顶栏：只有 traffic lights + 操作按钮，无标签栏。
pub(crate) fn title_bar(state: &IcedState, _tick_ms: f32) -> Element<'_, Message> {
    let tokens = tokens_for_state(state);
    let action_group = build_action_group(state, tokens);
    let control_group = build_control_group(state, tokens);

    let left_area: Element<'_, Message> = {
        let mut left_row = row![].spacing(0);

        #[cfg(target_os = "macos")]
        {
            left_row = left_row.push(
                container(Space::new().height(iced::Length::Fixed(TRAFFIC_LIGHT_DIAMETER)))
                    .width(iced::Length::Fixed(TRAFFIC_LIGHT_BAND_W))
                    .style(top_bar_ambient_style(tokens)),
            );
        }

        container(left_row)
            .width(iced::Length::Fill)
            .height(iced::Length::Fixed(TOP_BAR_H))
            .into()
    };

    let top_bar_row = row![left_area, action_group, control_group]
        .spacing(0)
        .align_y(Alignment::Center);

    container(top_bar_row)
        .height(iced::Length::Fixed(TOP_BAR_H))
        .padding(0)
        .style(top_bar_ambient_style(tokens))
        .into()
}

/// 顶栏背景样式（用于容器）
fn top_bar_ambient_style(tokens: crate::theme::DesignTokens) -> impl Fn(&Theme) -> iced::widget::container::Style + 'static {
    let bg = tokens.bg_header;
    move |_: &Theme| {
        container::Style::default().background(bg)
    }
}

fn build_tab_strip(state: &IcedState) -> Element<'_, Message> {
    let tokens = tokens_for_state(state);
    let available_w = tab_strip_width(state.window_size.width);
    let tabs_count = state.tabs.len();

    // 计算每个 chip 的目标宽度
    let avg_chip_w = if tabs_count > 0 {
        (available_w / tabs_count as f32).max(TAB_CHIP_MIN_WIDTH)
    } else {
        TAB_CHIP_WIDTH
    };
    let target_chip_w = TAB_CHIP_WIDTH.min(avg_chip_w);

    // 构建标签行
    let mut tabs_row = row![].spacing(0).align_y(Alignment::Center);

    for (i, tab) in state.tabs.iter().enumerate() {
        let tab_label = tab.title.clone();
        let is_active = i == state.active_tab;

        // 关闭按钮：悬停时显示，固定 24px 可交互区域
        let is_hovered = Some(i) == state.tab_hover_index;
        let show_close = is_hovered;
        let close_w = if show_close { TAB_CLOSE_HIT_W } else { 0.0 };
        let label_w = (target_chip_w - TAB_CHIP_PAD_H * 2.0 - close_w - TAB_LABEL_CLOSE_SPACING).max(0.0);

        let select_btn = container(
            button(
                text(tab_label).size(11),
            )
            .on_press(Message::TabScrollTo(i))
            .width(iced::Length::Fixed(label_w))
            .height(iced::Length::Fill)
            .style(style_tab_strip(tokens)),
        )
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_y(Alignment::Center);

        let close_btn: Element<'_, Message> = if show_close {
            // 欢迎页模式下不显示关闭按钮（欢迎页不是标签，不能被关闭）
            if state.show_welcome {
                Space::new()
                    .width(iced::Length::Fixed(0.0))
                    .height(iced::Length::Fill)
                    .into()
            } else {
                icon_tab_close_button(tokens, i, TAB_CLOSE_HIT_W)
            }
        } else {
            Space::new()
                .width(iced::Length::Fixed(close_w))
                .height(iced::Length::Fill)
                .into()
        };

        let body_h = if is_active { TOP_BAR_H - 2.0 } else { TOP_BAR_H };

        let top_line = container(
            Space::new()
                .width(iced::Length::Fill)
                .height(iced::Length::Fixed(if is_active { 2.0 } else { 0.0 })),
        )
        .style(move |_theme: &Theme| {
            if is_active {
                container::Style::default().background(tokens.accent_base)
            } else {
                container::Style::default()
            }
        });

        // 每个 chip 根据 is_active 独立设置背景色
        let chip_bg = if is_active { tokens.bg_primary } else { tokens.bg_header };
        let chip_bg_style = move |_: &Theme| container::Style {
            background: Some(iced::Background::Color(chip_bg)),
            ..Default::default()
        };

        // chip 内层容器：无背景，仅负责内容布局与垂直居中
        let inner = container(
            column![
                top_line,
                container(
                    row![select_btn, close_btn]
                        .spacing(0)
                        .align_y(Alignment::Center)
                )
                .width(iced::Length::Fill)
                .height(iced::Length::Fixed(body_h))
                .align_y(Alignment::Center),
            ]
            .spacing(0),
        )
        .padding(0)
        .width(iced::Length::Fixed(target_chip_w))
        .height(iced::Length::Fixed(TOP_BAR_H))
        .style(chip_bg_style);

        let chip = mouse_area(inner)
            .on_enter(Message::TabChipHover(Some(i)))
            .on_exit(Message::TabChipHover(None));

        tabs_row = tabs_row.push(chip);
    }

    // 计算溢出数量
    let min_chips = (available_w / TAB_CHIP_MIN_WIDTH).floor() as usize;
    let overflow_count = tabs_count.saturating_sub(min_chips);

    // 标签滚动区
    let scrollable_tabs = scrollable(tabs_row)
        .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::hidden()))
        .width(iced::Length::Fill)
        .height(iced::Length::Fixed(TOP_BAR_H))
        .on_scroll(|viewport| Message::TabScrollTick(viewport.absolute_offset().x));

    // 动态构建行：无徽章时不添加徽章元素
    let mut final_row = row![scrollable_tabs].spacing(0).align_y(Alignment::Center);
    if overflow_count > 0 {
        let badge = build_overflow_badge(overflow_count, tokens);
        final_row = final_row.push(badge);
    }

    final_row.into()
}

fn build_overflow_badge(overflow_count: usize, tokens: crate::theme::DesignTokens) -> Element<'static, Message> {
    let badge = button(
        text(format!("+{}", overflow_count))
            .size(11)
            .color(tokens.text_secondary)
            .align_y(Alignment::Center),
    )
    .on_press(Message::TabOverflowToggle)
    .width(iced::Length::Fixed(TOP_ICON_BTN))
    .height(iced::Length::Fixed(TOP_ICON_BTN))
    .style(style_top_icon(tokens));

    container(badge)
        .width(iced::Length::Shrink)
        .height(iced::Length::Fixed(TOP_BAR_H))
        .align_y(Alignment::Center)
        .into()
}

fn build_action_group(state: &IcedState, tokens: crate::theme::DesignTokens) -> Element<'static, Message> {
    let i18n = &state.model.i18n;

    // 快速连接图标 (QuickConnect)
    let quick_icon = icon_view_with(
        IconOptions::new(IconId::QuickConnect)
            .with_size(15)
            .with_color(tokens.text_secondary),
        Message::TopQuickConnect,
    );
    let btn_quick = button(quick_icon)
        .on_press(Message::TopQuickConnect)
        .width(iced::Length::Fixed(TOP_ICON_BTN))
        .height(iced::Length::Fixed(TOP_ICON_BTN))
        .style(style_top_icon(tokens));

    // 新建标签页图标 (Plus)
    let plus_icon = icon_view_with(
        IconOptions::new(IconId::Plus)
            .with_size(15)
            .with_color(tokens.text_secondary),
        Message::TopAddTab,
    );
    let btn_new = button(plus_icon)
        .on_press(Message::TopAddTab)
        .width(iced::Length::Fixed(TOP_ICON_BTN))
        .height(iced::Length::Fixed(TOP_ICON_BTN))
        .style(style_top_icon(tokens));

    let quick_tip = text(i18n.tr("iced.topbar.quick_connect")).size(12);
    let new_tip = text(i18n.tr("iced.topbar.new_tab")).size(12);
    container(
        row![
            tooltip(btn_quick, quick_tip, iced::widget::tooltip::Position::Bottom),
            tooltip(btn_new, new_tip, iced::widget::tooltip::Position::Bottom),
        ]
        .align_y(Alignment::Center),
    )
    .height(iced::Length::Fixed(TOP_BAR_H))
    .padding([0, 0])
    .into()
}

fn build_control_group(state: &IcedState, tokens: crate::theme::DesignTokens) -> Element<'static, Message> {
    let i18n = &state.model.i18n;

    // 设置图标 (Gear)
    let gear_icon = icon_view_with(
        IconOptions::new(IconId::Gear)
            .with_size(15)
            .with_color(tokens.text_secondary),
        Message::TopOpenSettings,
    );
    let btn_settings = button(gear_icon)
        .on_press(Message::TopOpenSettings)
        .width(iced::Length::Fixed(TOP_ICON_BTN))
        .height(iced::Length::Fixed(TOP_ICON_BTN))
        .style(style_top_icon(tokens));
    let settings_tip = text(i18n.tr("iced.topbar.settings_center")).size(12);
    let settings_ctrl = tooltip(btn_settings, settings_tip, iced::widget::tooltip::Position::Bottom);

    // 窗口控制按钮
    let win_controls: Element<'static, Message> = {
        #[cfg(not(target_os = "macos"))]
        {
            // 最小化图标 (-)
            let minus_icon = icon_view_with(
                IconOptions::new(IconId::Close)
                    .with_size(12)
                    .with_color(tokens.text_secondary),
                Message::WinMinimize,
            );
            let btn_min = button(minus_icon)
                .on_press(Message::WinMinimize)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(26.0))
                .style(style_top_icon(tokens));

            // 最大化/还原图标 (reload 用作占位)
            let max_icon = icon_view_with(
                IconOptions::new(IconId::Reload)
                    .with_size(12)
                    .with_color(tokens.text_secondary),
                Message::WinToggleMaximize,
            );
            let btn_max = button(max_icon)
                .on_press(Message::WinToggleMaximize)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(26.0))
                .style(style_top_icon(tokens));

            // 关闭图标 (×)
            let close_icon = icon_view_with(
                IconOptions::new(IconId::Close)
                    .with_size(12)
                    .with_color(tokens.text_secondary),
                Message::WinClose,
            );
            let btn_close = button(close_icon)
                .on_press(Message::WinClose)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(26.0))
                .style(style_top_icon(tokens));

            row![
                btn_min,
                btn_max,
                btn_close,
            ]
            .spacing(2)
            .align_y(Alignment::Center)
            .into()
        }
        #[cfg(target_os = "macos")]
        {
            Space::new().into()
        }
    };

    let control_row = row![settings_ctrl, win_controls]
        .align_y(Alignment::Center);

    #[cfg(not(target_os = "macos"))]
    {
        control_row.spacing(4)
    }

    container(control_row)
    .width(iced::Length::Fixed(TOP_CONTROL_GROUP_W))
    .height(iced::Length::Fixed(TOP_BAR_H))
    .padding([0, 0])
    .align_x(Alignment::End)
    .into()
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 创建标签页关闭图标按钮
fn icon_tab_close_button(tokens: crate::theme::DesignTokens, tab_index: usize, hit_w: f32) -> Element<'static, Message> {
    let close_icon = icon_view_with(
        IconOptions::new(IconId::Close)
            .with_size(TAB_CLOSE_ICON_W as u32)
            .with_color(tokens.text_secondary),
        Message::TabClose(tab_index),
    );
    button(close_icon)
        .on_press(Message::TabClose(tab_index))
        .width(iced::Length::Fixed(hit_w))
        .height(iced::Length::Fixed(hit_w))
        .padding(if hit_w > TAB_CLOSE_ICON_W { (hit_w-TAB_CLOSE_ICON_W)/2.0 } else { 0.0 }  )
        .style(style_top_icon(tokens))
        .into()
}