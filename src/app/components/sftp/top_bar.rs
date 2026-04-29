//! SFTP 面板顶栏组件

use iced::alignment::Alignment;
use iced::widget::{Space, container, row, text};
use iced::{Element, Length};

use crate::app::components::helpers::tokens_for_state;
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::state::SftpPanel;
use crate::theme::DesignTokens;
use crate::theme::icons::{IconButtonSize, IconId, icon_button_with_tooltip};
use crate::theme::layout;

/// 构建 SFTP 面板顶栏
pub(crate) fn sftp_top_bar(state: &IcedState, sftp: &SftpPanel) -> Element<'static, Message> {
    let tokens = tokens_for_state(state);
    let i18n = &state.model.i18n;

    // 路径面包屑导航
    let path_display = build_path_breadcrumb(sftp, &tokens);

    // 右侧操作按钮（标准图标按钮）
    let layout_btn = icon_button_with_tooltip(
        IconId::FnLayout,
        IconButtonSize::STANDARD,
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpToggleLayout),
        i18n.tr("iced.sftp.btn.toggle_layout"),
        &tokens,
    );
    let refresh_btn = icon_button_with_tooltip(
        IconId::FnReload,
        IconButtonSize::STANDARD,
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpRefresh),
        i18n.tr("iced.sftp.btn.refresh"),
        &tokens,
    );
    let close_btn = icon_button_with_tooltip(
        IconId::FnClose,
        IconButtonSize::COMPACT,
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpToggle),
        i18n.tr("iced.sftp.btn.close"),
        &tokens,
    );
    let upload_btn = icon_button_with_tooltip(
        IconId::ActionUpload2Line,
        IconButtonSize::COMPACT,
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpUpload),
        i18n.tr("iced.sftp.btn.upload"),
        &tokens,
    );

    let actions = row![upload_btn, layout_btn, refresh_btn, close_btn]
        .spacing(4)
        .align_y(Alignment::Center);

    let content = row![path_display, Space::new().width(Length::Fill), actions]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([0, 8]);

    container(content)
        .height(Length::Fixed(layout::SFTP_TOP_BAR_HEIGHT))
        .width(Length::Fill)
        .align_y(Alignment::Center)
        .style(top_bar_style(tokens))
        .into()
}

/// 构建路径面包屑导航（每段可点击跳转到对应目录）
fn build_path_breadcrumb(sftp: &SftpPanel, tokens: &DesignTokens) -> Element<'static, Message> {
    let current_path = if sftp.current_path.is_empty() {
        "/".to_string()
    } else {
        sftp.current_path.clone()
    };

    // 根路径特殊处理
    if current_path == "/" {
        return text("/")
            .size(12)
            .color(tokens.text_secondary)
            .into();
    }

    // 按 "/" 分割路径段
    let segments: Vec<&str> = current_path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return text("/")
            .size(12)
            .color(tokens.text_secondary)
            .into();
    }

    let mut parts: Vec<Element<'static, Message>> = Vec::new();

    // 根目录 "/"
    parts.push(
        text("/")
            .size(12)
            .color(tokens.text_secondary)
            .into(),
    );

    for (i, segment) in segments.iter().enumerate() {
        let is_last = i == segments.len() - 1;
        let segment_path = format!("/{}", segments[..=i].join("/"));

        if is_last {
            // 最后一段：当前目录，不可点击
            parts.push(
                text(segment.to_string())
                    .size(12)
                    .color(tokens.text_primary)
                    .into(),
            );
        } else {
            // 中间段：可点击跳转
            let accent = tokens.accent_base;
            let segment_text = text(segment.to_string())
                .size(12)
                .color(accent);

            let btn = iced::widget::button(segment_text)
                .padding([2, 4])
                .style(move |_: &iced::Theme, status: iced::widget::button::Status| {
                    let mut style = iced::widget::button::Style::default();
                    if let iced::widget::button::Status::Hovered = status {
                        style.background = Some(iced::Background::Color(
                            iced::Color::from_rgba(accent.r, accent.g, accent.b, 0.10),
                        ));
                        style.border = iced::Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        };
                    }
                    style
                })
                .on_press(Message::SftpTab(
                    crate::app::message::SftpTabMessage::SftpNavigate(segment_path),
                ));
            parts.push(btn.into());

            // 分隔符
            parts.push(
                text("/")
                    .size(12)
                    .color(tokens.text_disabled)
                    .into(),
            );
        }
    }

    row(parts)
        .spacing(2)
        .align_y(Alignment::Center)
        .into()
}

/// 顶栏背景样式
fn top_bar_style(
    tokens: DesignTokens,
) -> impl Fn(&iced::Theme) -> iced::widget::container::Style + 'static {
    let bg = tokens.surface_1;
    move |_: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(bg)),
        border: iced::Border {
            width: 0.0,
            color: iced::Color::TRANSPARENT,
            radius: Default::default(),
        },
        ..Default::default()
    }
}
