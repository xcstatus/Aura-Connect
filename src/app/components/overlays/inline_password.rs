use iced::alignment::Alignment;
use iced::Element;
use iced::widget::{button, column, container, row, text, stack,text_input, Space};
use iced::padding::Padding;
use secrecy::ExposeSecret;

use crate::app::components::helpers::{layered_scrim_style, tokens_for_state, top_bar_material_style};
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::widgets::chrome_button::style_chrome_primary;
use crate::app::widgets::chrome_button::style_top_icon;
use crate::theme::icons::{icon_view_with, IconId, IconOptions};
use crate::theme::layout;

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

    // Password visibility toggle button
    let visibility_toggle = {
        let icon = icon_view_with(
            if state.show_inline_password {
                IconOptions::new(IconId::EyeOff)
                    .with_width(12)
                    .with_height(8)
            } else {
                IconOptions::new(IconId::Eye)
                    .with_width(12)
                    .with_height(8)
            }
            .with_size(14)
            .with_color(tokens.text_secondary),
            Message::QuickConnectInlinePasswordToggleVisibility,
        );
        button(icon)
            .on_press(Message::QuickConnectInlinePasswordToggleVisibility)
            .padding(0)
            .width(iced::Length::Fixed(28.0))
            .height(iced::Length::Fixed(28.0))
            .style(style_top_icon(tokens))
    };

    // 输入框本身（右侧留出按钮空间）
    let password_input = text_input(label, state.inline_password_input.expose_secret())
        .on_input(Message::QuickConnectInlinePasswordChanged)
        .on_submit(Message::QuickConnectInlinePasswordSubmit(
            state.inline_password_input.expose_secret().to_string()
        ))
        .secure(state.show_inline_password)
        .width(iced::Length::Fill)
        .padding(Padding {
            left: 8.0,      // 左侧内边距保持视觉舒适
            right: 36.0,    // 右侧留出空间给按钮（按钮宽度28 + 间隙8）
            top: 8.0,
            bottom: 8.0,
        });

    // 使用 Stack 将按钮叠加在输入框的右侧内部
    let input_with_icon = stack![
        password_input,
        container(visibility_toggle)
            .width(iced::Length::Fill)
            .height(iced::Length::Fixed(36.0))
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Center)
            .style(container::transparent)
            .padding(Padding {
                right: 8.0,
                ..Default::default()
            })
    ]
    .width(iced::Length::Fill); 
    // Close button for the overlay
    let close_button = {
        let icon = icon_view_with(
            IconOptions::new(IconId::Close)
                .with_size(12)
                .with_color(tokens.text_secondary),
            Message::QuickConnectInlinePasswordClose,
        );
        button(icon)
            .on_press(Message::QuickConnectInlinePasswordClose)
            .padding(0)
            .width(iced::Length::Fixed(layout::ICON_BUTTON_SIZE_COMPACT))
            .height(iced::Length::Fixed(layout::ICON_BUTTON_SIZE_COMPACT))
            .style(style_top_icon(tokens))
    };

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
                close_button,
            ]
            .align_y(Alignment::Center),
            input_with_icon,
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

    let centered = container(input_form)
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
