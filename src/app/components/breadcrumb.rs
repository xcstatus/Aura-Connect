use iced::alignment::Alignment;
use iced::Element;
use iced::widget::{button, container, row, text};

use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::terminal_viewport;
use crate::app::widgets::chrome_button::style_chrome_secondary;

/// Build the breadcrumb navigation bar.
pub(crate) fn breadcrumb(state: &IcedState) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let term_vp = terminal_viewport::terminal_viewport_spec_for_settings(&state.model.settings.terminal);
    let is_connected = state.active_session_is_connected();
    let current_node = state
        .model
        .selected_session_id
        .as_deref()
        .unwrap_or(i18n.tr("iced.breadcrumb.not_connected"));
    let cwd = std::env::current_dir()
        .ok()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "-".to_string());
    let conn_label = if is_connected {
        i18n.tr("iced.breadcrumb.connected")
    } else {
        i18n.tr("iced.breadcrumb.disconnected")
    };

    container(
        row![
            text(conn_label),
            text("/"),
            text(current_node),
            text("|"),
            text(cwd),
            row![
                button(text(i18n.tr("iced.btn.reconnect")).size(12))
                    .on_press(Message::ConnectPressed)
                    .style(style_chrome_secondary(12.0)),
                button(text(i18n.tr("iced.btn.sftp")).size(12)).style(style_chrome_secondary(12.0)),
                button(text(i18n.tr("iced.btn.port_fwd")).size(12))
                    .style(style_chrome_secondary(12.0)),
            ]
            .spacing(8),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding(term_vp.breadcrumb_padding())
    .height(iced::Length::Fixed(term_vp.breadcrumb_block_h()))
    .align_y(Alignment::Center)
    .into()
}
