use iced::Element;
use iced::alignment::Alignment;
use iced::widget::{Space, button, column, container, row, text, text_input};

use crate::app::components::helpers::{
    layered_scrim_style, lg_modal_container_style, tokens_for_state,
};
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::widgets::chrome_button::{lg_icon_button, lg_primary_button, lg_secondary_button};
use crate::theme::icons::{IconId, IconOptions, icon_view_with};

/// Build the vault unlock modal.
pub(crate) fn vault_unlock_modal(state: &IcedState) -> Element<'_, Message> {
    let Some(unlock) = state.vault_unlock.as_ref() else {
        return Space::new().into();
    };

    let tokens = tokens_for_state(state);

    let scrim = container(
        Space::new()
            .width(iced::Length::Fill)
            .height(iced::Length::Fill),
    )
    .style(layered_scrim_style(tokens, 0));

    let title = if unlock.pending_save_credentials_profile_id.is_some() {
        state
            .model
            .i18n
            .tr("iced.vault_unlock.title_save_credentials")
    } else {
        state.model.i18n.tr("iced.vault_unlock.title")
    };

    let i18n = &state.model.i18n;
    use secrecy::ExposeSecret;

    let body = column![
        row![text(title).size(16),].align_y(Alignment::Center),
        text_input(
            state
                .model
                .i18n
                .tr("iced.vault_unlock.password_placeholder"),
            unlock.password.expose_secret()
        )
        .secure(true)
        .on_input(Message::VaultUnlockPasswordChanged),
    ]
    .spacing(10)
    .width(iced::Length::Fill);

    let mut body = column![body].spacing(10).width(iced::Length::Fill);

    if unlock.pending_save_credentials_profile_id.is_some() {
        body = body.push(
            text(
                state
                    .model
                    .i18n
                    .tr("iced.vault_unlock.hint_save_credentials"),
            )
            .size(12),
        );
    }

    if let Some(err) = unlock.error.as_ref() {
        body = body.push(text(err).size(12));
    }

    body = body.push(
        row![
            button(text(i18n.tr("iced.vault_unlock.btn.confirm")).size(13))
                .on_press(Message::VaultUnlockSubmit)
                .style(lg_primary_button(tokens)),
            button(text(i18n.tr("iced.vault_unlock.btn.cancel")).size(13))
                .on_press(Message::VaultUnlockClose)
                .style(lg_secondary_button(tokens)),
        ]
        .spacing(8),
    );

    use iced::widget::Stack;
    // 关闭按钮使用 Stack 绝对定位到弹窗右上角，与弹窗边缘无间隙
    let close_btn_overlay = container(
        container(icon_close_button(tokens))
            .width(iced::Length::Fill)
            .height(iced::Length::Fixed(32.0))
            .align_x(iced::alignment::Horizontal::Right)
            .padding(0),
    )
    .width(iced::Length::Fill)
    .height(iced::Length::Fixed(32.0));

    let card = container(Stack::with_children([
        container(body)
            .padding(iced::Padding {
                top: 16.0,
                right: 16.0,
                bottom: 16.0,
                left: 16.0,
            })
            .style(lg_modal_container_style(tokens))
            .into(),
        close_btn_overlay.into(),
    ]))
    .width(iced::Length::Fixed(520.0));

    let centered = container(card)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    Stack::with_children([scrim.into(), centered.into()])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 创建关闭图标按钮（尺寸与弹窗 header 高度对齐）
fn icon_close_button(tokens: crate::theme::DesignTokens) -> Element<'static, Message> {
    let close_icon = icon_view_with(
        IconOptions::new(IconId::FnClose)
            .with_size(14)
            .with_color(tokens.text_secondary),
        Message::VaultUnlockClose,
    );
    button(close_icon)
        .on_press(Message::VaultUnlockClose)
        .width(iced::Length::Fixed(32.0))
        .height(iced::Length::Fixed(32.0))
        .padding(0)
        .style(lg_icon_button(tokens))
        .into()
}
