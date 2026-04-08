use iced::alignment::Alignment;
use iced::Element;
use iced::widget::{button, column, container, row, text, text_input, Space};

use secrecy::ExposeSecret;

use crate::app::components::helpers::{layered_scrim_style, tokens_for_state, top_bar_material_style};
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::widgets::chrome_button::style_chrome_primary;

/// Terminal-inline overlay: shows a compact password/passphrase input form.
pub(crate) fn inline_password_overlay(state: &IcedState) -> Element<'_, Message> {
    let needs_inline_input = !state.quick_connect_open
        && matches!(state.quick_connect_flow, crate::app::state::QuickConnectFlow::NeedAuthPassword);

    if !needs_inline_input {
        return Space::new().into();
    }

    let tokens = tokens_for_state(state);
    let is_key = matches!(state.model.draft.auth, crate::session::AuthMethod::Key { .. });
    let label = if is_key {
        state.model.i18n.tr("iced.term.passphrase_placeholder")
    } else {
        state.model.i18n.tr("iced.term.password_placeholder")
    };

    let scrim = container(Space::new().width(iced::Length::Fill).height(iced::Length::Fill))
        .style(layered_scrim_style(tokens, 0));

    let input_form = container(
        column![
            row![
                text(if is_key {
                    state.model.i18n.tr("iced.quick_connect.need_passphrase")
                } else {
                    state.model.i18n.tr("iced.quick_connect.need_password")
                })
                .size(13),
                Space::new().width(iced::Length::Fill),
            ]
            .align_y(Alignment::Center),
            text_input(label, state.inline_password_input.expose_secret())
                .on_input(Message::QuickConnectInlinePasswordChanged)
                .on_submit(Message::QuickConnectInlinePasswordSubmit(
                    state.inline_password_input.expose_secret().to_string()
                ))
                .secure(true)
                .width(iced::Length::Fixed(280.0)),
            row![
                button(text(state.model.i18n.tr("iced.btn.connect")).size(13))
                    .on_press(Message::QuickConnectInlinePasswordSubmit(
                        state.inline_password_input.expose_secret().to_string()
                    ))
                    .style(style_chrome_primary(tokens)),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(10)
        .width(iced::Length::Fixed(320.0)),
    )
    .padding(16)
    .style(top_bar_material_style(tokens));

    use iced::widget::Stack;
    Stack::with_children([scrim.into(), input_form.into()])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}
