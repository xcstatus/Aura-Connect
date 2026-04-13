//! SFTP 标签内容布局视图
//!
//! 根据 PaneLayout 渲染终端 + SFTP 面板的组合布局。

use iced::widget::{container, row, Column, Space};
use iced::{Element, Length};

use crate::app::components::sftp::panel::sftp_panel_view;
use crate::app::components::terminal_view;
use crate::app::message::Message;
use crate::app::state::{IcedState, PaneLayout};

/// 构建标签内容视图（终端 + 可选的 SFTP 面板）
pub(crate) fn tab_content(state: &IcedState) -> Element<'_, Message> {
    let pane = match state.tab_panes.get(state.active_tab) {
        Some(p) => p,
        None => {
            return container(Space::new().width(Length::Fill).height(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }
    };

    let layout = pane.pane_layout;

    match layout {
        PaneLayout::TerminalOnly => {
            // 仅显示终端
            terminal_view::terminal_panel(state)
        }
        PaneLayout::TerminalAboveSftp { sftp_ratio } => {
            // 终端在上，SFTP 在下
            let breadcrumb_visible = state.breadcrumb_pinned || state.breadcrumb_temp_visible;
            let term_portion = if breadcrumb_visible {
                ((1.0 - sftp_ratio) * 100.0) as u16
            } else {
                100u16
            };
            let sftp_portion = (sftp_ratio * 100.0) as u16;

            let term_view = terminal_view::terminal_panel(state);
            let sftp_view = sftp_panel_view(state);

            Column::new()
                .push(
                    container(term_view)
                        .height(Length::FillPortion(term_portion))
                        .width(Length::Fill)
                )
                .push(
                    container(sftp_view)
                        .height(Length::FillPortion(sftp_portion))
                        .width(Length::Fill)
                )
                .spacing(0)
                .height(Length::Fill)
                .into()
        }
        PaneLayout::SftpBesideTerminal { sftp_ratio } => {
            // SFTP 在左，终端在右
            let sftp_portion = (sftp_ratio * 100.0) as u16;
            let term_portion = ((1.0 - sftp_ratio) * 100.0) as u16;

            let sftp_view = sftp_panel_view(state);
            let term_view = terminal_view::terminal_panel(state);

            row![
                container(sftp_view)
                    .width(Length::FillPortion(sftp_portion))
                    .height(Length::Fill),
                container(term_view)
                    .width(Length::FillPortion(term_portion))
                    .height(Length::Fill),
            ]
            .spacing(0)
            .height(Length::Fill)
            .into()
        }
    }
}
