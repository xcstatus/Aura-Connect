use iced::alignment::Alignment;
use iced::Element;
use iced::widget::{button, column, container, row, text, text_input, Space};

use crate::app::components::helpers::layered_scrim_style;
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::widgets::chrome_button::{style_chrome_primary, style_top_icon};

/// Build the vault unlock modal.
pub(crate) fn vault_unlock_modal(state: &IcedState) -> Element<'_, Message> {
    let Some(unlock) = state.vault_unlock.as_ref() else {
        return Space::new().into()
    };

    let scrim = container(Space::new().width(iced::Length::Fill).height(iced::Length::Fill))
        .style(|theme: &iced::Theme| layered_scrim_style(theme, 0));

    let title = if unlock.pending_save_credentials_profile_id.is_some() {
        state.model.i18n.tr("iced.vault_unlock.title_save_credentials")
    } else {
        state.model.i18n.tr("iced.vault_unlock.title")
    };

    let i18n = &state.model.i18n;
    use secrecy::ExposeSecret;
    let mut body = column![
        row![
            text(title).size(16),
            Space::new().width(iced::Length::Fill),
            button(text("×").size(14))
                .on_press(Message::VaultUnlockClose)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(28.0))
                .style(style_top_icon(14.0)),
        ]
        .align_y(Alignment::Center),
        text_input(state.model.i18n.tr("iced.vault_unlock.password_placeholder"), unlock.password.expose_secret())
            .secure(true)
            .on_input(Message::VaultUnlockPasswordChanged),
    ]
    .spacing(10)
    .width(iced::Length::Fill);

    if unlock.pending_save_credentials_profile_id.is_some() {
        body = body.push(text(state.model.i18n.tr("iced.vault_unlock.hint_save_credentials")).size(12));
    }

    if let Some(err) = unlock.error.as_ref() {
        body = body.push(text(err).size(12));
    }

    body = body.push(
        row![
            button(text(i18n.tr("iced.vault_unlock.btn.confirm")).size(13))
                .on_press(Message::VaultUnlockSubmit)
                .style(style_chrome_primary(13.0)),
            button(text(i18n.tr("iced.vault_unlock.btn.cancel")).size(13))
                .on_press(Message::VaultUnlockClose)
                .style(crate::app::widgets::chrome_button::style_chrome_secondary(13.0)),
        ]
        .spacing(8),
    );

    let card = container(body)
        .width(iced::Length::Fixed(520.0))
        .padding(16)
        .style(crate::app::chrome::top_bar_material_style);
    let centered = container(card)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    use iced::widget::Stack;
    Stack::with_children([scrim.into(), centered.into()])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}
