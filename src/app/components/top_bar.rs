use iced::alignment::Alignment;
use iced::widget::{button, column, container, mouse_area, row, text, tooltip, Space};
use iced::{Element, Theme};

use crate::app::chrome::{
    TOP_BAR_EDGE_PAD, TOP_BAR_H, TOP_CONTROL_GROUP_W, TOP_ICON_BTN,
    TRAFFIC_LIGHT_BAND_W, TRAFFIC_LIGHT_DIAMETER,    tab_strip_width,
};
use crate::app::components::helpers::{tokens_for_state};
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::widgets::chrome_button::{style_tab_strip, style_top_icon};
use crate::theme::icons::{icon_view_with, IconId, IconOptions};

/// Build the top bar (tab strip + action buttons + control buttons).
pub(crate) fn top_bar(state: &IcedState, tick_ms: f32) -> Element<'_, Message> {
    let tokens = tokens_for_state(state);
    let tabs_row = build_tab_strip(state, tick_ms);
    let action_group = build_action_group(state, tokens);
    let control_group = build_control_group(state, tokens);

    let mut top_bar_row = row![].spacing(0).align_y(Alignment::Center);

    #[cfg(target_os = "macos")]
    {
        top_bar_row = top_bar_row.push(
            container(Space::new().height(iced::Length::Fixed(TRAFFIC_LIGHT_DIAMETER)))
                .width(iced::Length::Fixed(TRAFFIC_LIGHT_BAND_W))
                .style(top_bar_ambient_style(tokens)),
        );
    }

    top_bar_row = top_bar_row
        .push(tabs_row)
        .push(action_group)
        .push(control_group);

    container(top_bar_row)
        .height(iced::Length::Fixed(TOP_BAR_H))
        .padding(iced::Padding::from([0.0, TOP_BAR_EDGE_PAD]))
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

fn build_tab_strip(state: &IcedState, tick_ms: f32) -> Element<'_, Message> {
    let tokens = tokens_for_state(state);
    let mut tabs_row = row![].spacing(0).align_y(Alignment::Center);

    let mut total_tabs_w = 0.0f32;

    for (i, tab) in state.tabs.iter().enumerate() {
        let tab_label = tab.title.clone();
        let tab_w = state.tab_animated_width(i, tick_ms);
        let is_active = i == state.active_tab;

        // 关闭按钮：仅当前标签显示（始终可见）
        let show_close = is_active;
        let close_w = if show_close { 30.0 } else { 0.0 };
        let label_w = (tab_w - close_w - 3.0).max(16.0);

        let select_btn = button(
            text(tab_label).size(11),
        )
        .on_press(Message::TabSelected(i))
        .width(iced::Length::Fixed(label_w))
        .height(iced::Length::Fill)
        .style(style_tab_strip(tokens));

        let close_btn: Element<'_, Message> = if show_close {
            icon_tab_close_button(tokens, i, 30.0)
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
        .padding([0.0, 4.0])
        .width(iced::Length::Fill)
        .height(iced::Length::Fixed(TOP_BAR_H))
        .style(chip_bg_style);

        let chip = mouse_area(inner)
            .on_enter(Message::TabChipHover(Some(i)))
            .on_exit(Message::TabChipHover(None));

        tabs_row = tabs_row.push(chip);
        total_tabs_w += tab_w + 8.0; // +8 补偿 padding
    }

    // 标签栏宽度：未达最大可用宽度时自适应，总宽度否则填满
    let available_w = tab_strip_width(state.window_size.width);
    let tabs_row_container = container(tabs_row)
        .height(iced::Length::Fixed(TOP_BAR_H));

    if total_tabs_w < available_w {
        tabs_row_container
            .width(iced::Length::Fixed(total_tabs_w))
            .into()
    } else {
        tabs_row_container
            .width(iced::Length::Fill)
            .into()
    }
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
        .spacing(6)
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

    container(
        row![settings_ctrl, win_controls]
            .spacing(4)
            .align_y(Alignment::Center),
    )
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
fn icon_tab_close_button(tokens: crate::theme::DesignTokens, tab_index: usize, size: f32) -> Element<'static, Message> {
    let close_icon = icon_view_with(
        IconOptions::new(IconId::Close)
            .with_size(12)
            .with_color(tokens.text_secondary),
        Message::TabClose(tab_index),
    );
    button(close_icon)
        .on_press(Message::TabClose(tab_index))
        .width(iced::Length::Fixed(size))
        .height(iced::Length::Fixed(size))
        .padding(iced::Padding { top: 0.0, right: 3.0, bottom: 0.0, left: 0.0 }) // 距右侧 3px
        .style(style_tab_strip(tokens))
        .into()
}
