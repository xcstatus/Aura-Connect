use iced::Element;
use iced::alignment::Alignment;
use iced::widget::{Space, button, column, container, row, text, text_input};

use secrecy::ExposeSecret;

use crate::app::components::helpers::{
    layered_scrim_style, tokens_for_state, top_bar_material_style,
};
use crate::app::message::Message;
use crate::app::state::{IcedState, VaultFlowMode};
use crate::app::widgets::chrome_button::{lg_icon_button, lg_primary_button, lg_secondary_button};
use crate::theme::icons::{IconId, IconOptions, icon_view_with};

/// Build the vault (password manager) initialization/change-password modal.
pub(crate) fn vault_modal(state: &IcedState) -> Element<'_, Message> {
    let Some(flow) = state.vault_flow.as_ref() else {
        return Space::new().into();
    };

    let tokens = tokens_for_state(state);

    let scrim = container(
        Space::new()
            .width(iced::Length::Fill)
            .height(iced::Length::Fill),
    )
    .style(layered_scrim_style(tokens, 0));

    let title = match flow.mode {
        VaultFlowMode::Initialize => state.model.i18n.tr("iced.vault.title.initialize"),
        VaultFlowMode::ChangePassword => state.model.i18n.tr("iced.vault.title.change_password"),
    };

    let i18n = &state.model.i18n;
    let mut body = column![
        row![
            text(title).size(16),
            Space::new().width(iced::Length::Fill),
            icon_close_button(tokens),
        ]
        .align_y(Alignment::Center),
    ]
    .spacing(10)
    .width(iced::Length::Fill);

    if matches!(flow.mode, VaultFlowMode::ChangePassword) {
        body = body.push(
            text_input(
                i18n.tr("iced.vault.label.old_password"),
                flow.old_password.expose_secret(),
            )
            .secure(true)
            .on_input(Message::VaultOldPasswordChanged),
        );
    }

    body = body
        .push(
            text_input(
                i18n.tr("iced.vault.label.new_password"),
                flow.new_password.expose_secret(),
            )
            .secure(true)
            .on_input(Message::VaultNewPasswordChanged),
        )
        .push(
            text_input(
                i18n.tr("iced.vault.label.confirm_password"),
                flow.confirm_password.expose_secret(),
            )
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
                .style(lg_primary_button(tokens)),
            button(text(i18n.tr("iced.btn.cancel")).size(13))
                .on_press(Message::VaultClose)
                .style(lg_secondary_button(tokens)),
        ]
        .spacing(8),
    );

    let card = container(body)
        .width(iced::Length::Fixed(520.0))
        .padding(16)
        .align_x(iced::alignment::Horizontal::Right)
        .style(top_bar_material_style(tokens));
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

// ============================================================================
// 辅助函数
// ============================================================================

/// 创建关闭图标按钮
fn icon_close_button(tokens: crate::theme::DesignTokens) -> Element<'static, Message> {
    let close_icon = icon_view_with(
        IconOptions::new(IconId::FnClose)
            .with_size(14)
            .with_color(tokens.text_secondary),
        Message::VaultClose,
    );
    button(close_icon)
        .on_press(Message::VaultClose)
        .width(iced::Length::Fixed(28.0))
        .height(iced::Length::Fixed(28.0))
        .style(lg_icon_button(tokens))
        .into()
}
