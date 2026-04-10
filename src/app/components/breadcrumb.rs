//! Breadcrumb 导航栏组件。

use iced::alignment::Alignment;
use iced::widget::{button, container, row, text, Space};
use iced::{Element, Length};

use crate::app::components::helpers::{terminal_area_bg_style, tokens_for_state};
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::terminal_viewport;
use crate::app::widgets::chrome_button::style_top_icon;
use crate::theme::icons::{icon_view_with, IconId, IconOptions};
use crate::theme::layout::{
    BREADCRUMB_BTN_GAP, BREADCRUMB_FLOAT_ICON_MARGIN, BREADCRUMB_FLOAT_ICON_SIZE,
    BREADCRUMB_SECTIONS_GAP,
};

/// 构建会话标识文本。
/// 优先使用会话名称，如果没有则使用 username@host[:port]。
fn build_session_label(state: &IcedState) -> String {
    // 1. 优先使用会话名称
    if let Some(tab) = state.tabs.get(state.active_tab) {
        if !tab.title.is_empty() && tab.title != state.model.i18n.tr("iced.tab.new") {
            return tab.title.clone();
        }
    }

    // 2. 回退到 user@host[:port]
    let draft = &state.model.draft;
    let mut label = format!("{}@{}", draft.user, draft.host);
    if draft.port != "22" {
        label.push_str(&format!(":{}", draft.port));
    }
    label
}

/// 构建 breadcrumb 导航栏。
/// 分为左右两部分：
/// - 左侧：会话标识 + 当前目录
/// - 右侧：重新连接、SFTP、端口转发、固定按钮
pub(crate) fn breadcrumb(state: &IcedState) -> Element<'_, Message> {
    let tokens = tokens_for_state(state);
    let term_vp = terminal_viewport::terminal_viewport_spec_for_settings(&state.model.settings.terminal);
    let i18n = &state.model.i18n;

    // 会话标识
    let session_label = build_session_label(state);
    let session_text = text(session_label)
        .size(13);

    // 当前目录
    let cwd_text = text(state.remote_cwd.clone())
        .size(13);

    // 左侧区域：会话标识 + 目录
    let left_area = row![session_text, text(" · ").size(13), cwd_text]
        .spacing(4)
        .align_y(Alignment::Center);

    // 右侧按钮
    let right_buttons = build_action_buttons(state, tokens, i18n);

    // 完整布局：左侧区域 + 弹性空间 + 右侧按钮（自动右对齐）
    container(
        row![left_area, Space::new().width(Length::Fill), right_buttons]
            .spacing(BREADCRUMB_SECTIONS_GAP)
            .align_y(Alignment::Center),
    )
    .padding(term_vp.breadcrumb_padding())
    .height(iced::Length::Fixed(term_vp.breadcrumb_block_h()))
    .align_y(Alignment::Center)
    .style(terminal_area_bg_style(tokens))
    .into()
}

/// 构建右侧操作按钮行。
fn build_action_buttons(
    state: &IcedState,
    tokens: crate::theme::DesignTokens,
    _i18n: &crate::i18n::I18n,
) -> Element<'static, Message> {
    // 重新连接按钮
    let reconnect_icon = icon_view_with(
        IconOptions::new(IconId::Reload)
            .with_size(15)
            .with_color(tokens.text_secondary),
        Message::ConnectPressed,
    );
    let reconnect_btn = button(reconnect_icon)
        .on_press(Message::ConnectPressed)
        .style(style_top_icon(tokens));

    // SFTP 按钮
    let sftp_icon = icon_view_with(
        IconOptions::new(IconId::Sftp)
            .with_size(15)
            .with_color(tokens.text_secondary),
        Message::BreadcrumbSftp,
    );
    let sftp_btn = button(sftp_icon)
        .on_press(Message::BreadcrumbSftp)
        .style(style_top_icon(tokens));

    // 端口转发按钮
    let port_icon = icon_view_with(
        IconOptions::new(IconId::QuickConnect)
            .with_size(15)
            .with_color(tokens.text_secondary),
        Message::BreadcrumbPortForward,
    );
    let port_btn = button(port_icon)
        .on_press(Message::BreadcrumbPortForward)
        .style(style_top_icon(tokens));

    // 固定按钮：点击后隐藏 breadcrumb
    let pin_icon = icon_view_with(
        IconOptions::new(IconId::Pin)
            .with_size(15)
            .with_color(tokens.text_secondary),
        Message::BreadcrumbTogglePin,
    );
    let pin_btn = button(pin_icon)
        .on_press(Message::BreadcrumbTogglePin)
        .style(style_top_icon(tokens));

    row![reconnect_btn, sftp_btn, port_btn, pin_btn]
        .spacing(BREADCRUMB_BTN_GAP)
        .align_y(Alignment::Center)
        .into()
}

/// 构建 breadcrumb 浮动图标。
/// 固定在终端区域右上角，用于在 breadcrumb 隐藏时快速呼出。
pub(crate) fn breadcrumb_float_icon(state: &IcedState) -> Element<'static, Message> {
    use iced::widget::Space;

    let tokens = tokens_for_state(state);

    // 使用 pin 图标，点击后显示 breadcrumb（临时模式）
    let icon = icon_view_with(
        IconOptions::new(IconId::Unpin)
            .with_size(15)
            .with_color(tokens.text_secondary),
        Message::BreadcrumbShowTemp,
    );

    let btn = button(icon)
        .on_press(Message::BreadcrumbShowTemp)
        .width(Length::Fixed(BREADCRUMB_FLOAT_ICON_SIZE))
        .height(Length::Fixed(BREADCRUMB_FLOAT_ICON_SIZE))
        .style(style_top_icon(tokens));

    // 使用 Space 将图标推到右上角
    container(
        row![
            Space::new().width(Length::Fill),
            row![btn]
                .spacing(0)
                .align_y(Alignment::Center),
        ]
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(BREADCRUMB_FLOAT_ICON_SIZE + BREADCRUMB_FLOAT_ICON_MARGIN * 2.0))
    .padding(BREADCRUMB_FLOAT_ICON_MARGIN)
    .into()
}
