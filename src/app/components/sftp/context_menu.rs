//! SFTP 右键菜单组件

use iced::widget::{button, column, container, text};
use iced::{Element, Length};

use crate::app::components::helpers::tokens_for_state;
use crate::app::message::{Message, SftpContextMenuTarget, SftpTabMessage};
use crate::app::state::{IcedState, SftpContextMenuState};

/// 右键菜单项数据
struct MenuItem {
    label: &'static str,
    icon: &'static str,
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
        SftpContextMenuTarget::File { name: _, path, is_dir } => {
            if *is_dir {
                vec![
                    MenuItem {
                        label: i18n.tr("iced.sftp.menu.download_folder"),
                        icon: "⬇",
                        action: SftpTabMessage::SftpDownloadFolder(path.clone()),
                    },
                    MenuItem {
                        label: i18n.tr("iced.sftp.menu.copy_path"),
                        icon: "📋",
                        action: SftpTabMessage::SftpCopyPath(path.clone()),
                    },
                    MenuItem {
                        label: i18n.tr("iced.sftp.menu.delete"),
                        icon: "🗑",
                        action: SftpTabMessage::SftpDelete(path.clone()),
                    },
                ]
            } else {
                vec![
                    MenuItem {
                        label: i18n.tr("iced.sftp.menu.download"),
                        icon: "⬇",
                        action: SftpTabMessage::SftpDownload(path.clone()),
                    },
                    MenuItem {
                        label: i18n.tr("iced.sftp.menu.copy_path"),
                        icon: "📋",
                        action: SftpTabMessage::SftpCopyPath(path.clone()),
                    },
                    MenuItem {
                        label: i18n.tr("iced.sftp.menu.delete"),
                        icon: "🗑",
                        action: SftpTabMessage::SftpDelete(path.clone()),
                    },
                ]
            }
        }
        SftpContextMenuTarget::EmptyArea => {
            vec![
                MenuItem {
                    label: i18n.tr("iced.sftp.menu.create_folder"),
                    icon: "📁",
                    action: SftpTabMessage::SftpCreateFolder,
                },
                MenuItem {
                    label: i18n.tr("iced.sftp.menu.refresh"),
                    icon: "🔄",
                    action: SftpTabMessage::SftpRefresh,
                },
            ]
        }
    };

    // 构建菜单列
    let menu_column = column(
        items.iter()
            .map(|item| {
                menu_item_row(
                    item.label,
                    item.icon,
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
        .style(context_menu_style(&tokens))
        .into()
}

/// 构建单个菜单项行
fn menu_item_row<'a>(
    label: &'a str,
    icon: &'a str,
    action: Message,
    tokens: &crate::theme::DesignTokens,
) -> Element<'a, Message> {
    let icon_elem = text(format!("{}  {}", icon, label))
        .size(12)
        .color(tokens.text_primary);

    button(icon_elem)
        .width(Length::Fill)
        .padding([6, 12])
        .style(context_menu_item_style(tokens))
        .on_press(action)
        .into()
}

/// 右键菜单样式
fn context_menu_style(
    tokens: &crate::theme::DesignTokens,
) -> impl Fn(&iced::Theme) -> iced::widget::container::Style + 'static {
    let bg = tokens.surface_1;
    let border_default = tokens.border_default;
    move |_: &iced::Theme| {
        iced::widget::container::Style {
            background: Some(iced::Background::Color(bg)),
            border: iced::Border {
                width: 1.0,
                color: border_default,
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    }
}

/// 右键菜单项按钮样式
fn context_menu_item_style(
    tokens: &crate::theme::DesignTokens,
) -> impl Fn(&iced::Theme, button::Status) -> iced::widget::button::Style + 'static {
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
