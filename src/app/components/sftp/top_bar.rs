//! SFTP 面板顶栏组件

use iced::alignment::Alignment;
use iced::widget::{button, container, row, text, Space};
use iced::{Element, Length};

use crate::app::components::helpers::tokens_for_state;
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::state::SftpPanel;
use crate::theme::icons::{icon, IconId, IconState};
use crate::theme::layout;
use crate::theme::DesignTokens;

/// 构建 SFTP 面板顶栏
pub(crate) fn sftp_top_bar(state: &IcedState, sftp: &SftpPanel) -> Element<'static, Message> {
    let tokens = tokens_for_state(state);

    // 路径导航
    let path_display = build_path_display(state, sftp);

    // 右侧操作按钮
    let actions = row![
        // 切换布局按钮
        layout_toggle_button(state),
        // 刷新按钮
        refresh_button(state),
        // 关闭按钮
        close_button(state),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    // 整体布局：左侧路径 + 右侧操作
    let content = row![path_display, Space::new().width(Length::Fill), actions]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([0, 8]);

    container(content)
        .height(Length::Fixed(layout::SFTP_TOP_BAR_HEIGHT))
        .width(Length::Fill)
        .style(top_bar_style(tokens))
        .into()
}

/// 构建路径显示区域（显示当前路径）
fn build_path_display(state: &IcedState, sftp: &SftpPanel) -> Element<'static, Message> {
    let tokens = tokens_for_state(state);

    // 当前路径（默认显示 "/"）
    let display_path = if sftp.current_path.is_empty() {
        "/".to_string()
    } else {
        sftp.current_path.clone()
    };

    // 直接显示路径文本
    text(display_path)
        .size(12)
        .color(tokens.text_primary)
        .into()
}

/// 布局切换按钮
fn layout_toggle_button(state: &IcedState) -> Element<'static, Message> {
    let tokens = tokens_for_state(state);

    let icon_elem: Element<'static, ()> = icon(IconId::Layout, &tokens, IconState::Default);
    let icon_msg: Element<'static, Message> = icon_elem.map(move |_| {
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpToggleLayout)
    });

    button(icon_msg)
        .width(Length::Fixed(layout::ICON_BUTTON_SIZE))
        .height(Length::Fixed(layout::ICON_BUTTON_SIZE))
        .padding(0)
        .style(icon_button_style(tokens))
        .on_press(Message::SftpTab(crate::app::message::SftpTabMessage::SftpToggleLayout))
        .into()
}

/// 刷新按钮
fn refresh_button(state: &IcedState) -> Element<'static, Message> {
    let tokens = tokens_for_state(state);

    let icon_elem: Element<'static, ()> = icon(IconId::Reload, &tokens, IconState::Default);
    let icon_msg: Element<'static, Message> = icon_elem.map(move |_| {
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpRefresh)
    });

    button(icon_msg)
        .width(Length::Fixed(layout::ICON_BUTTON_SIZE))
        .height(Length::Fixed(layout::ICON_BUTTON_SIZE))
        .padding(0)
        .style(icon_button_style(tokens))
        .on_press(Message::SftpTab(crate::app::message::SftpTabMessage::SftpRefresh))
        .into()
}

/// 关闭按钮
fn close_button(state: &IcedState) -> Element<'static, Message> {
    let tokens = tokens_for_state(state);

    let icon_elem: Element<'static, ()> = icon(IconId::Close, &tokens, IconState::Default);
    let icon_msg: Element<'static, Message> = icon_elem.map(move |_| {
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpToggle)
    });

    button(icon_msg)
        .width(Length::Fixed(layout::ICON_BUTTON_SIZE))
        .height(Length::Fixed(layout::ICON_BUTTON_SIZE))
        .padding(0)
        .style(icon_button_style(tokens))
        .on_press(Message::SftpTab(crate::app::message::SftpTabMessage::SftpToggle))
        .into()
}

/// 顶栏背景样式
fn top_bar_style(tokens: DesignTokens) -> impl Fn(&iced::Theme) -> iced::widget::container::Style + 'static {
    let bg = tokens.surface_1;
    move |_: &iced::Theme| {
        iced::widget::container::Style {
            background: Some(iced::Background::Color(bg)),
            border: iced::Border {
                width: 0.0,
                color: iced::Color::TRANSPARENT,
                radius: Default::default(),
            },
            ..Default::default()
        }
    }
}

/// 图标按钮样式
fn icon_button_style(tokens: DesignTokens) -> impl Fn(&iced::Theme, button::Status) -> iced::widget::button::Style + 'static {
    let surface_2 = tokens.surface_2;
    let surface_3 = tokens.surface_3;
    move |_: &iced::Theme, status: button::Status| {
        let mut style = iced::widget::button::Style::default();

        match status {
            button::Status::Hovered => {
                style.background = Some(iced::Background::Color(surface_2));
            }
            button::Status::Pressed => {
                style.background = Some(iced::Background::Color(surface_3));
            }
            _ => {}
        }

        style
    }
}

/// 扁平按钮样式
#[allow(dead_code)]
fn flat_button_style(tokens: DesignTokens) -> impl Fn(&iced::Theme, button::Status) -> iced::widget::button::Style + 'static {
    let surface_2 = tokens.surface_2;
    let surface_3 = tokens.surface_3;
    move |_: &iced::Theme, status: button::Status| {
        let mut style = iced::widget::button::Style::default();

        match status {
            button::Status::Hovered => {
                style.background = Some(iced::Background::Color(surface_2));
            }
            button::Status::Pressed => {
                style.background = Some(iced::Background::Color(surface_3));
            }
            _ => {}
        }

        style
    }
}
