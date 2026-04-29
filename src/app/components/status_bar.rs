use iced::Background;
use iced::Element;
use iced::alignment::Alignment;
use iced::widget::{container, row, text};

use crate::theme::layout::BOTTOM_BAR_HEIGHT;

use crate::app::components::helpers::tokens_for_state;
use crate::app::message::Message;
use crate::app::state::{IcedState, VaultStatus};
use crate::theme::DesignTokens;

/// 状态栏 Liquid Glass 样式
fn status_bar_style(
    tokens: DesignTokens,
) -> impl Fn(&iced::Theme) -> iced::widget::container::Style + 'static {
    move |_: &iced::Theme| iced::widget::container::Style {
        background: Some(Background::Color(tokens.glass_subtle)),
        border: iced::Border {
            color: tokens.border_subtle,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

/// Build the status bar at the bottom of the terminal area.
pub(crate) fn status_bar(state: &IcedState) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let engine = crate::app::engine_adapter::EngineAdapter::active(state);
    let term_scroll = engine.scroll();
    let term_in_scrollback = engine.is_in_scrollback();
    let is_connected = state.active_session_is_connected();
    let tokens = tokens_for_state(state);
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

    // 根据连接状态选择状态颜色
    let status_color = if is_connected {
        tokens.success // Connected: #00FF80
    } else {
        tokens.text_secondary // Disconnected
    };

    let vault_word = match state.vault_status {
        VaultStatus::Uninitialized => "未初始化",
        VaultStatus::Unlocked => "已解锁",
        VaultStatus::Locked => "已锁定",
        VaultStatus::Unavailable => "不可用",
    };

    // Vault 状态颜色
    let vault_color = match state.vault_status {
        VaultStatus::Unlocked => tokens.success,
        VaultStatus::Locked | VaultStatus::Uninitialized => tokens.text_secondary,
        VaultStatus::Unavailable => tokens.error,
    };

    container(
        row![
            text(format!("{}: ", i18n.tr("iced.footer.status"))).color(status_color),
            text(status_word).color(status_color),
            text(" | "),
            text(format!("终端: ")),
            text(term_scroll_word),
            text(" | "),
            text(format!("{}: ", i18n.tr("iced.footer.vault"))).color(vault_color),
            text(vault_word).color(vault_color),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .width(iced::Length::Fill)
    .height(iced::Length::Fixed(BOTTOM_BAR_HEIGHT))
    .padding([0, 12])
    .style(status_bar_style(tokens))
    .into()
}
