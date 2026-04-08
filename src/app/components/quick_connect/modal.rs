use iced::widget::{container, Stack};
use iced::Element;
use iced::Length;

use crate::app::components::helpers::layered_scrim_alpha;
use crate::app::message::Message;
use crate::app::state::IcedState;

use super::form::quick_connect_new_form;
use super::picker::quick_connect_picker;

/// Quick connect modal wrapper with animation.
pub(crate) fn quick_connect_modal_stack(state: &IcedState) -> Element<'_, Message> {
    let tick_ms = state.tick_ms();
    let scrim_alpha = state.quick_connect_anim_alpha(tick_ms);
    let scrim = container(iced::widget::Space::new().width(Length::Fill).height(Length::Fill))
        .style(layered_scrim_alpha(scrim_alpha, 0));

    let offset_y = state.quick_connect_anim_offset(tick_ms);
    let anchored = container(
        container(quick_connect_panel_content(state))
            .max_width(520.0)
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Top)
    .padding(iced::Padding {
        top: 6.0 - offset_y,
        right: 16.0,
        bottom: 16.0,
        left: 16.0,
    });

    Stack::with_children([scrim.into(), anchored.into()])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Routes to picker or form based on current panel state.
pub(crate) fn quick_connect_panel_content(state: &IcedState) -> Element<'_, Message> {
    use crate::app::state::QuickConnectPanel;

    match state.quick_connect_panel {
        QuickConnectPanel::Picker => quick_connect_picker(state),
        QuickConnectPanel::NewConnection => quick_connect_new_form(state),
    }
}
