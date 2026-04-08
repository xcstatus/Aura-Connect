use iced::alignment::Alignment;
use iced::Element;
use iced::widget::{button, checkbox, column, container, mouse_area, pick_list, row, text, text_input, Space};

use secrecy::ExposeSecret;

use crate::iced_app::components::helpers::modal_scrim_style;
use crate::iced_app::message::Message;
use crate::iced_app::state::IcedState;
use crate::iced_app::widgets::chrome_button::{style_chrome_primary, style_chrome_secondary, style_top_icon};

/// Build the session editor modal for creating/editing SSH session profiles.
pub fn session_editor_modal(state: &IcedState) -> Element<'_, Message> {
    let Some(ed) = state.session_editor.as_ref() else {
        return Space::new().into()
    };

    let i18n = &state.model.i18n;

    let scrim = mouse_area(
        container(Space::new().width(iced::Length::Fill).height(iced::Length::Fill))
            .style(modal_scrim_style),
    )
    .on_press(Message::SessionEditorClose);

    let auth_options: Vec<crate::session::AuthMethod> = vec![
        crate::session::AuthMethod::Password,
        crate::session::AuthMethod::Agent,
        crate::session::AuthMethod::Interactive,
        crate::session::AuthMethod::Key { private_key_path: String::new() },
    ];

    let mut body = column![
        row![
            text(i18n.tr("iced.settings.conn.edit")).size(16),
            Space::new().width(iced::Length::Fill),
            button(text("×").size(14))
                .on_press(Message::SessionEditorClose)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(28.0))
                .style(style_top_icon(14.0)),
        ]
        .align_y(Alignment::Center),
        text_input(i18n.tr("iced.field.host"), &ed.host)
            .on_input(Message::SessionEditorHostChanged)
            .width(iced::Length::Fill),
        text_input(i18n.tr("iced.field.port"), &ed.port)
            .on_input(Message::SessionEditorPortChanged)
            .width(iced::Length::Fill),
        text_input(i18n.tr("iced.field.user"), &ed.user)
            .on_input(Message::SessionEditorUserChanged)
            .width(iced::Length::Fill),
        pick_list(auth_options, Some(ed.auth.clone()), Message::SessionEditorAuthChanged).placeholder("Auth"),
    ]
    .spacing(10)
    .width(iced::Length::Fill);

    if matches!(ed.auth, crate::session::AuthMethod::Password) {
        body = body.push(
            text_input(i18n.tr("iced.field.password"), ed.password.expose_secret())
                .secure(true)
                .on_input(Message::SessionEditorPasswordChanged)
                .width(iced::Length::Fill),
        );
    }

    if ed.existing_credential_id.is_some() {
        body = body.push(
            checkbox(ed.clear_saved_password)
                .label("清除已保存密码")
                .on_toggle(Message::SessionEditorClearPasswordToggled),
        );
    }

    if let Some(err) = ed.error.as_ref() {
        body = body.push(text(err).size(12));
    }

    let actions = row![
        button(text(i18n.tr("iced.btn.save_settings")).size(13))
            .on_press(Message::SessionEditorSave)
            .style(style_chrome_primary(13.0)),
        button(text(i18n.tr("iced.quick_connect.back")).size(13))
            .on_press(Message::SessionEditorClose)
            .style(style_chrome_secondary(13.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    body = body.push(actions);

    let card = container(body)
        .width(iced::Length::Fixed(520.0))
        .padding(16)
        .style(crate::iced_app::chrome::top_bar_material_style);
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
