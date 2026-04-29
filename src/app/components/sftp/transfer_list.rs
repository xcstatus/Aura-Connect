//! SFTP 传输列表组件

use iced::alignment::Vertical;
use iced::widget::{Space, column, container, progress_bar, row, text};
use iced::{Element, Length, Pixels};

use crate::app::components::helpers::{tokens_for_state, lg_panel_container_style};
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::sftp::{SftpTransfer, TransferStatus};
use crate::theme::icons::{IconButtonSize, IconId, icon_button_with_tooltip};
use crate::theme::{DesignTokens, layout};

/// 构建传输列表面板
pub(crate) fn transfer_list_panel<'a>(state: &'a IcedState) -> Element<'a, Message> {
    let tokens = tokens_for_state(state);
    let i18n = &state.model.i18n;

    let pane = match state.tab_panes.get(state.active_tab) {
        Some(p) => p,
        None => return Space::new().width(Length::Fill).height(Length::Fill).into(),
    };

    let sftp = match &pane.sftp_panel {
        Some(s) => s,
        None => return Space::new().width(Length::Fill).height(Length::Fill).into(),
    };

    if !sftp.show_transfer_list {
        return Space::new().width(Length::Fixed(0.0)).height(Length::Fixed(0.0)).into();
    }

    // 标题栏
    let title = text(i18n.tr("iced.sftp.transfer.title"))
        .size(13)
        .color(tokens.text_primary);

    let open_dir_btn = icon_button_with_tooltip(
        IconId::FileFolder,
        IconButtonSize::COMPACT,
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpOpenDownloadDir),
        i18n.tr("iced.sftp.transfer.open_dir"),
        &tokens,
    );

    let clear_btn = icon_button_with_tooltip(
        IconId::FnDelete,
        IconButtonSize::COMPACT,
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpClearCompletedTransfers),
        i18n.tr("iced.sftp.transfer.clear"),
        &tokens,
    );

    let close_btn = icon_button_with_tooltip(
        IconId::FnClose,
        IconButtonSize::COMPACT,
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpToggleTransferList),
        i18n.tr("iced.sftp.btn.close"),
        &tokens,
    );

    let header = row![
        title,
        Space::new().width(Length::Fill),
        open_dir_btn,
        clear_btn,
        close_btn,
    ]
    .spacing(4)
    .align_y(Vertical::Center);

    // 传输列表
    let mut list = column![].spacing(4);

    if sftp.transfers.is_empty() {
        list = list.push(
            container(
                text(i18n.tr("iced.sftp.transfer.empty"))
                    .size(12)
                    .color(tokens.text_secondary),
            )
            .width(Length::Fill)
            .padding(16)
            .center_x(Length::Fill),
        );
    } else {
        for transfer in &sftp.transfers {
            list = list.push(transfer_item(transfer, tokens));
        }
    }

    let content = column![
        container(header)
            .padding([8, 12])
            .width(Length::Fill),
        container(list)
            .padding(iced::Padding {
                top: 4.0,
                right: 12.0,
                bottom: 8.0,
                left: 12.0,
            })
            .width(Length::Fill)
            .height(Length::Fill),
    ]
    .spacing(0);

    container(content)
        .width(Length::Fill)
        .max_height(Pixels(300.0))
        .style(lg_panel_container_style(tokens))
        .into()
}

/// 构建单个传输项
fn transfer_item<'a>(transfer: &SftpTransfer, tokens: DesignTokens) -> Element<'a, Message> {
    let file_name = std::path::Path::new(&transfer.source)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| transfer.source.clone());

    // 状态图标
    let status_icon = match transfer.status {
        TransferStatus::Pending => "⏳",
        TransferStatus::Running => "⬇",
        TransferStatus::Completed => "✓",
        TransferStatus::Failed => "✗",
        TransferStatus::Cancelled => "⊘",
    };

    let status_color = match transfer.status {
        TransferStatus::Running => tokens.accent_base,
        TransferStatus::Completed => tokens.success,
        TransferStatus::Failed => tokens.error,
        _ => tokens.text_secondary,
    };

    // 文件名
    let name_text = text(format!("{} {}", status_icon, file_name))
        .size(12)
        .color(tokens.text_primary);

    // 进度信息
    let progress_text = match transfer.status {
        TransferStatus::Running => {
            let speed = transfer.speed_human();
            let remaining = transfer.remaining_time_human();
            let progress_pct = (transfer.progress() * 100.0) as u32;
            format!("{}% · {} · {} remaining", progress_pct, speed, remaining)
        }
        TransferStatus::Completed => {
            let size = transfer.total_size;
            format!(
                "{} - Done",
                crate::sftp::RemoteFileEntry {
                    name: String::new(),
                    path: std::path::PathBuf::new(),
                    is_dir: false,
                    size,
                    modified: 0,
                    permissions: String::new(),
                    owner: None,
                    group: None,
                    is_symlink: false,
                    symlink_target: None,
                }
                .size_human()
            )
        }
        TransferStatus::Failed => {
            transfer.error.clone().unwrap_or_else(|| "Failed".to_string())
        }
        _ => String::new(),
    };

    let detail_text = text(progress_text)
        .size(11)
        .color(status_color);

    // 进度条（仅在运行中显示）
    let progress_widget: Option<iced::widget::ProgressBar> = if transfer.status == TransferStatus::Running {
        Some(progress_bar(0.0..=1.0, transfer.progress() as f32))
    } else {
        None
    };

    let mut col = column![name_text, detail_text].spacing(2);
    if let Some(pb) = progress_widget {
        col = col.push(pb);
    }

    container(col)
        .padding(8)
        .width(Length::Fill)
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(
                iced::Color::from_rgba(0.0, 0.0, 0.0, 0.05),
            )),
            border: iced::Border {
                radius: layout::SFTP_ROW_RADIUS.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
