//! SFTP 面板主视图组件

use iced::widget::{column, container, Space};
use iced::{Element, Length};

use crate::app::components::helpers::{terminal_area_bg_style, tokens_for_state};
use crate::app::components::sftp::file_list::file_list;
use crate::app::components::sftp::parent_row::parent_row;
use crate::app::components::sftp::top_bar::sftp_top_bar;
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::theme::layout;

/// 构建 SFTP 面板主视图
pub(crate) fn sftp_panel_view<'a>(state: &'a IcedState) -> Element<'a, Message> {
    let pane = match state.tab_panes.get(state.active_tab) {
        Some(p) => p,
        None => return Space::new().width(iced::Length::Fill).height(iced::Length::Fill).into(),
    };

    let sftp = match &pane.sftp_panel {
        Some(s) => s,
        None => return Space::new().width(iced::Length::Fill).height(iced::Length::Fill).into(),
    };

    let tokens = tokens_for_state(state);

    // 计算面板高度（基于布局比例）
    let height = calculate_sftp_height(state);

    container(
        column![
            sftp_top_bar(state, sftp),
            parent_row(state, sftp),
            file_list(state, sftp),
        ]
        .spacing(0)
        .height(Length::Fill),
    )
    .height(height)
    .width(iced::Length::Fill)
    .style(terminal_area_bg_style(tokens))
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
            // 从窗口高度中减去顶栏、面包屑、底栏后的可用高度
            let available = state.window_size.height
                - layout::TOP_BAR_HEIGHT
                - layout::BREADCRUMB_HEIGHT
                - layout::BOTTOM_BAR_HEIGHT;
            Length::Fixed(
                (available * sftp_ratio as f32).max(layout::SFTP_PANEL_MIN_HEIGHT),
            )
        }
        crate::app::state::PaneLayout::SftpBesideTerminal { .. } => Length::Fill,
    }
}
