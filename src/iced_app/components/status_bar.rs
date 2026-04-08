use iced::alignment::Alignment;
use iced::Element;
use iced::widget::{container, row, text};

use crate::theme::layout::BOTTOM_BAR_HEIGHT;

use crate::iced_app::chrome::main_chrome_style;
use crate::iced_app::message::Message;
use crate::iced_app::state::{IcedState, VaultStatus};

/// Build the status bar at the bottom of the terminal area.
pub fn status_bar(state: &IcedState) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let engine = crate::iced_app::engine_adapter::EngineAdapter::active(state);
    let term_scroll = engine.scroll();
    let term_in_scrollback = engine.is_in_scrollback();
    let is_connected = state.active_session_is_connected();
    let term_scroll_word = if term_in_scrollback {
        "回滚中"
    } else {
        "跟随底部"
    };

    let status_word = if is_connected {
        i18n.tr("iced.breadcrumb.connected")
    } else {
        i18n.tr("iced.breadcrumb.disconnected")
    };

    let vault_word = match state.vault_status {
        VaultStatus::Uninitialized => "未初始化",
        VaultStatus::Unlocked => "已解锁",
        VaultStatus::Locked => "已锁定",
        VaultStatus::Unavailable => "不可用",
    };

    container(
        row![
            text(format!("{}: {}", i18n.tr("iced.footer.status"), status_word)),
            text("|"),
            text(format!(
                "{}: {}",
                "终端",
                if term_in_scrollback {
                    format!(
                        "{} ({}/{})",
                        term_scroll_word,
                        term_scroll.offset_rows,
                        term_scroll.total_rows.saturating_sub(term_scroll.viewport_rows)
                    )
                } else {
                    term_scroll_word.to_string()
                }
            )),
            text("|"),
            text(format!("{}: {}", i18n.tr("iced.footer.hint"), state.model.status)),
            text("|"),
            text(format!("{}: {}", i18n.tr("iced.footer.vault"), vault_word)),
            text("|"),
            text(format!("{}: iced", i18n.tr("iced.footer.runtime"))),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .width(iced::Length::Fill)
    .height(iced::Length::Fixed(BOTTOM_BAR_HEIGHT))
    .padding([0, 12])
    .style(main_chrome_style)
    .into()
}
