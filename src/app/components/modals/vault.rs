use iced::alignment::Alignment;
use iced::Element;
use iced::widget::{button, column, container, mouse_area, row, text, text_input, Space};

use secrecy::ExposeSecret;

use crate::app::components::helpers::modal_scrim_style;
use crate::app::message::Message;
use crate::app::state::{IcedState, VaultFlowMode};
use crate::app::widgets::chrome_button::{style_chrome_primary, style_chrome_secondary, style_top_icon};

/// Build the vault (password manager) initialization/change-password modal.
pub fn vault_modal(state: &IcedState) -> Element<'_, Message> {
    let Some(flow) = state.vault_flow.as_ref() else {
        return Space::new().into()
    };

    let scrim = mouse_area(
        container(Space::new().width(iced::Length::Fill).height(iced::Length::Fill))
            .style(modal_scrim_style),
    )
    .on_press(Message::VaultClose);

    let title = match flow.mode {
        VaultFlowMode::Initialize => state.model.i18n.tr("iced.vault.title.initialize"),
        VaultFlowMode::ChangePassword => state.model.i18n.tr("iced.vault.title.change_password"),
    };

    let i18n = &state.model.i18n;
    let mut body = column![
        row![
            text(title).size(16),
            Space::new().width(iced::Length::Fill),
            button(text("×").size(14))
                .on_press(Message::VaultClose)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(28.0))
                .style(style_top_icon(14.0)),
        ]
        .align_y(Alignment::Center),
    ]
    .spacing(10)
    .width(iced::Length::Fill);

    if matches!(flow.mode, VaultFlowMode::ChangePassword) {
        body = body.push(
            text_input(i18n.tr("iced.vault.label.old_password"), flow.old_password.expose_secret())
                .secure(true)
                .on_input(Message::VaultOldPasswordChanged),
        );
    }

    body = body
        .push(
            text_input(i18n.tr("iced.vault.label.new_password"), flow.new_password.expose_secret())
                .secure(true)
                .on_input(Message::VaultNewPasswordChanged),
        )
        .push(
            text_input(i18n.tr("iced.vault.label.confirm_password"), flow.confirm_password.expose_secret())
                .secure(true)
                .on_input(Message::VaultConfirmPasswordChanged),
        );

    if let Some(err) = flow.error.as_ref() {
        body = body.push(text(err).size(12));
    }

    body = body.push(
        row![
            button(text(i18n.tr("iced.btn.confirm")).size(13))
                .on_press(Message::VaultSubmit)
                .style(style_chrome_primary(13.0)),
            button(text(i18n.tr("iced.btn.cancel")).size(13))
                .on_press(Message::VaultClose)
                .style(style_chrome_secondary(13.0)),
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
