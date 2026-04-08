use iced::alignment::Alignment;
use iced::Element;
use iced::widget::{container, row, text, Space};

use crate::app::components::helpers::top_bar_material_style;
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::theme::icons::{icon_view, IconId, IconOptions};

/// Terminal-inline overlay: shows animated connection progress when the quick-connect
/// modal is closed and a connection is in progress.
pub(crate) fn inline_connecting_overlay(state: &IcedState) -> Element<'_, Message> {
    let is_connecting = !state.quick_connect_open
        && matches!(state.quick_connect_flow, crate::app::state::QuickConnectFlow::Connecting);

    if !is_connecting {
        return Space::new().into();
    }

    let stage = state.connection_stage;
    let tick = state.tick_count;
    let dots = match tick % 3 {
        0 => "",
        1 => ".",
        _ => "..",
    };
    let stage_label = match stage {
        crate::app::state::ConnectionStage::VaultLoading => state.model.i18n.tr("iced.stage.vault_loading"),
        crate::app::state::ConnectionStage::SshConnecting => state.model.i18n.tr("iced.stage.ssh_connecting"),
        crate::app::state::ConnectionStage::Authenticating => state.model.i18n.tr("iced.stage.authenticating"),
        crate::app::state::ConnectionStage::SessionSetup => state.model.i18n.tr("iced.stage.session_setup"),
        _ => state.model.i18n.tr("iced.term.connecting"),
    };

    let tokens = crate::app::components::helpers::tokens_for_state(state);
    let reload_icon: Element<'static, Message> = icon_view(
        IconOptions::new(IconId::Reload)
            .with_size(14)
            .with_color(tokens.text_secondary)
    ).map(|_| unreachable!("icon has no message"));
    let content = container(
        row![reload_icon, text(format!("{stage_label}{dots}")).size(12)]
            .spacing(6)
            .align_y(Alignment::Center),
    )
    .padding(8)
    .style(top_bar_material_style(tokens));

    use iced::widget::Stack;
    Stack::with_children([Space::new().into(), content.into()])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}
