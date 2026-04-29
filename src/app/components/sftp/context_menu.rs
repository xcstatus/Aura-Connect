//! SFTP 右键菜单组件

use iced::alignment::Vertical;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Length};

use crate::app::components::helpers::tokens_for_state;
use crate::app::message::{Message, SftpContextMenuTarget, SftpTabMessage};
use crate::app::state::{IcedState, SftpContextMenuState};
use crate::app::widgets::chrome_button::lg_secondary_button;
use crate::theme::icons::{IconId, IconOptions, icon_view_with};

/// 右键菜单项数据
struct MenuItem {
    label: &'static str,
    icon_id: IconId,
    action: SftpTabMessage,
}

/// 构建右键菜单
pub(crate) fn context_menu<'a>(
    state: &'a IcedState,
    ctx_menu: &'a SftpContextMenuState,
) -> Element<'a, Message> {
    let tokens = tokens_for_state(state);
    let i18n = &state.model.i18n;

    // 根据目标类型生成菜单项
    let items = match &ctx_menu.target {
        SftpContextMenuTarget::File {
            name: _,
            path,
            is_dir,
        } => {
            if *is_dir {
                vec![
                    MenuItem {
                        label: i18n.tr("iced.sftp.menu.download_folder"),
                        icon_id: IconId::ActionDownload,
                        action: SftpTabMessage::SftpDownloadFolder(path.clone()),
                    },
                    MenuItem {
                        label: i18n.tr("iced.sftp.menu.copy_path"),
                        icon_id: IconId::FnQuickConnect, // Using as copy icon
                        action: SftpTabMessage::SftpCopyPath(path.clone()),
                    },
                    MenuItem {
                        label: i18n.tr("iced.sftp.menu.delete"),
                        icon_id: IconId::FnDelete,
                        action: SftpTabMessage::SftpDelete(path.clone()),
                    },
                ]
            } else {
                vec![
                    MenuItem {
                        label: i18n.tr("iced.sftp.menu.download"),
                        icon_id: IconId::ActionDownload,
                        action: SftpTabMessage::SftpDownload(path.clone()),
                    },
                    MenuItem {
                        label: i18n.tr("iced.sftp.menu.copy_path"),
                        icon_id: IconId::FnQuickConnect,
                        action: SftpTabMessage::SftpCopyPath(path.clone()),
                    },
                    MenuItem {
                        label: i18n.tr("iced.sftp.menu.delete"),
                        icon_id: IconId::FnDelete,
                        action: SftpTabMessage::SftpDelete(path.clone()),
                    },
                ]
            }
        }
        SftpContextMenuTarget::EmptyArea => {
            vec![
                MenuItem {
                    label: i18n.tr("iced.sftp.menu.create_folder"),
                    icon_id: IconId::FileFolderPlus,
                    action: SftpTabMessage::SftpCreateFolder,
                },
                MenuItem {
                    label: i18n.tr("iced.sftp.menu.refresh"),
                    icon_id: IconId::FnReload,
                    action: SftpTabMessage::SftpRefresh,
                },
            ]
        }
    };

    // 构建菜单列
    let menu_column = column(
        items
            .iter()
            .map(|item| {
                menu_item_row(
                    item.label,
                    item.icon_id,
                    Message::SftpTab(item.action.clone()),
                    &tokens,
                )
            })
            .collect::<Vec<_>>(),
    )
    .spacing(0)
    .padding(4);

    // 菜单容器
    container(menu_column)
        .style(context_menu_container_style(&tokens))
        .into()
}

/// 构建单个菜单项行
fn menu_item_row<'a>(
    label: &'a str,
    icon_id: IconId,
    action: Message,
    tokens: &crate::theme::DesignTokens,
) -> Element<'a, Message> {
    let icon_elem = icon_view_with(
        IconOptions::new(icon_id)
            .with_size(12)
            .with_color(tokens.text_secondary),
        action.clone(),
    );

    let label_elem = text(label).size(12).color(tokens.text_primary);

    let content = row![
        icon_elem,
        Space::new().width(iced::Length::Fixed(8.0)),
        label_elem,
    ]
    .spacing(0)
    .align_y(Vertical::Center);

    button(content)
        .width(Length::Fill)
        .padding([6, 12])
        .style(lg_secondary_button(*tokens))
        .on_press(action)
        .into()
}

/// 右键菜单容器样式
fn context_menu_container_style(
    tokens: &crate::theme::DesignTokens,
) -> impl Fn(&iced::Theme) -> iced::widget::container::Style + 'static {
    let bg = tokens.surface_1;
    let border_default = tokens.border_default;
    move |_: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(bg)),
        border: iced::Border {
            width: 1.0,
            color: border_default,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}
