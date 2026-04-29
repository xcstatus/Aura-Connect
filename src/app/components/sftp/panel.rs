//! SFTP 面板主视图组件

use iced::alignment::{Alignment, Vertical};
use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Element, Length};

use crate::app::components::helpers::{tokens_for_state, lg_panel_container_style, terminal_area_bg_style};
use crate::app::components::sftp::file_list::file_list;
use crate::app::components::sftp::top_bar::sftp_top_bar;
use crate::app::components::sftp::transfer_list::transfer_list_panel;
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::theme::layout;

/// 构建 SFTP 面板主视图
pub(crate) fn sftp_panel_view<'a>(state: &'a IcedState) -> Element<'a, Message> {
    let pane = match state.tab_panes.get(state.active_tab) {
        Some(p) => p,
        None => {
            return Space::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }
    };

    let sftp = match &pane.sftp_panel {
        Some(s) => s,
        None => {
            return Space::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }
    };

    let tokens = tokens_for_state(state);
    let height = calculate_sftp_height(state);

    // 主内容
    let main_content = column![
        sftp_top_bar(state, sftp),
        file_list(state, sftp),
        transfer_list_panel(state),
    ]
    .spacing(0)
    .height(Length::Fill);

    // 如果有待确认操作，叠加覆盖层
    if sftp.pending_delete.is_some() {
        let overlay = delete_confirm_overlay(state, sftp);
        container(
            iced::widget::Stack::new()
                .push(main_content)
                .push(overlay),
        )
        .height(height)
        .width(Length::Fill)
        .style(terminal_area_bg_style(tokens))
        .into()
    } else if sftp.creating_folder {
        let overlay = create_folder_overlay(state, sftp);
        container(
            iced::widget::Stack::new()
                .push(main_content)
                .push(overlay),
        )
        .height(height)
        .width(Length::Fill)
        .style(terminal_area_bg_style(tokens))
        .into()
    } else {
        container(main_content)
            .height(height)
            .width(Length::Fill)
            .style(terminal_area_bg_style(tokens))
            .into()
    }
}

/// 删除确认覆盖层
fn delete_confirm_overlay<'a>(state: &'a IcedState, sftp: &'a crate::app::state::SftpPanel) -> Element<'a, Message> {
    let tokens = tokens_for_state(state);
    let i18n = &state.model.i18n;

    let file_name = sftp
        .pending_delete
        .as_ref()
        .and_then(|p| std::path::Path::new(p).file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "...".to_string());

    let title = text(i18n.tr("iced.sftp.confirm.delete"))
        .size(14)
        .color(tokens.text_primary);

    let file_name_text = text(file_name.clone())
        .size(12)
        .color(tokens.text_secondary);

    let confirm_btn = button(
        text(i18n.tr("iced.btn.confirm")).size(12).color(tokens.on_accent_label),
    )
    .padding([6, 16])
    .style(crate::theme::liquid_glass::glass_primary_button_style(tokens))
    .on_press(Message::SftpTab(crate::app::message::SftpTabMessage::SftpDeleteConfirm));

    let cancel_btn = button(
        text(i18n.tr("iced.btn.cancel")).size(12),
    )
    .padding([6, 16])
    .style(crate::theme::liquid_glass::glass_secondary_button_style(tokens))
    .on_press(Message::SftpTab(crate::app::message::SftpTabMessage::SftpDeleteCancel));

    let content = iced::widget::column![
        title,
        Space::new().height(Length::Fixed(8.0)),
        file_name_text,
        Space::new().height(Length::Fixed(12.0)),
        row![cancel_btn, confirm_btn].spacing(8).align_y(Vertical::Center),
    ]
    .align_x(Alignment::Center)
    .padding(20)
    .spacing(4);

    // 半透明遮罩 + 居中卡片
    let scrim = container(Space::new().width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.4))),
            ..Default::default()
        });

    let card = container(content)
        .style(lg_panel_container_style(tokens))
        .center_x(Length::Fixed(280.0))
        .center_y(Length::Fixed(140.0));

    container(
        iced::widget::Stack::new()
            .push(scrim)
            .push(card),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// 创建文件夹输入覆盖层
fn create_folder_overlay<'a>(state: &'a IcedState, sftp: &'a crate::app::state::SftpPanel) -> Element<'a, Message> {
    let tokens = tokens_for_state(state);
    let i18n = &state.model.i18n;

    let title = text(i18n.tr("iced.sftp.menu.create_folder"))
        .size(14)
        .color(tokens.text_primary);

    let input = text_input(
        i18n.tr("iced.sftp.placeholder.folder_name"),
        &sftp.new_folder_name,
    )
    .on_input(|s| Message::SftpTab(crate::app::message::SftpTabMessage::SftpCreateFolderNameChanged(s)))
    .on_submit(Message::SftpTab(crate::app::message::SftpTabMessage::SftpCreateFolderConfirm))
    .padding(8)
    .width(Length::Fixed(240.0));

    let confirm_btn = button(
        text(i18n.tr("iced.btn.confirm")).size(12).color(tokens.on_accent_label),
    )
    .padding([6, 16])
    .style(crate::theme::liquid_glass::glass_primary_button_style(tokens))
    .on_press(Message::SftpTab(crate::app::message::SftpTabMessage::SftpCreateFolderConfirm));

    let cancel_btn = button(
        text(i18n.tr("iced.btn.cancel")).size(12),
    )
    .padding([6, 16])
    .style(crate::theme::liquid_glass::glass_secondary_button_style(tokens))
    .on_press(Message::SftpTab(crate::app::message::SftpTabMessage::SftpCreateFolderCancel));

    let content = iced::widget::column![
        title,
        Space::new().height(Length::Fixed(8.0)),
        input,
        Space::new().height(Length::Fixed(12.0)),
        row![cancel_btn, confirm_btn].spacing(8).align_y(Vertical::Center),
    ]
    .align_x(Alignment::Center)
    .padding(20)
    .spacing(4);

    let scrim = container(Space::new().width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.4))),
            ..Default::default()
        });

    let card = container(content)
        .style(lg_panel_container_style(tokens))
        .center_x(Length::Fixed(300.0))
        .center_y(Length::Fixed(160.0));

    container(
        iced::widget::Stack::new()
            .push(scrim)
            .push(card),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// 根据当前布局计算 SFTP 面板高度
fn calculate_sftp_height(state: &IcedState) -> Length {
    let pane = match state.tab_panes.get(state.active_tab) {
        Some(p) => p,
        None => return Length::Fill,
    };

    match pane.pane_layout {
        crate::app::state::PaneLayout::TerminalOnly => Length::Fixed(0.0),
        crate::app::state::PaneLayout::TerminalAboveSftp { sftp_ratio } => {
            let available = state.window_size.height
                - layout::TOP_BAR_HEIGHT
                - layout::BREADCRUMB_HEIGHT
                - layout::BOTTOM_BAR_HEIGHT;
            Length::Fixed((available * sftp_ratio as f32).max(layout::SFTP_PANEL_MIN_HEIGHT))
        }
        crate::app::state::PaneLayout::SftpBesideTerminal { .. } => Length::Fill,
    }
}
