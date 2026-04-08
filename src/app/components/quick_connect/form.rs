use iced::alignment::Alignment;
use iced::widget::{
    button, column, container, pick_list, row, text, text_input,
};
use iced::Element;
use iced::Theme;
use secrecy::ExposeSecret;

use crate::app::chrome::top_bar_material_style;
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::widgets::chrome_button::style_chrome_primary;
use crate::app::widgets::chrome_button::style_chrome_secondary;

/// Quick connect new connection form: host/port, user, password, auth method, etc.
pub fn quick_connect_new_form(state: &IcedState) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let is_connected = state.active_session_is_connected();
    let flow = state.quick_connect_flow;
    let err_kind = state.quick_connect_error_kind;
    let stage = state.connection_stage;
    let is_connecting = matches!(flow, crate::app::state::QuickConnectFlow::Connecting);

    // Form inputs
    let host_row = row![
        text_input(i18n.tr("iced.field.host"), &state.model.draft.host)
            .on_input(Message::HostChanged)
            .width(iced::Length::FillPortion(3)),
        text_input(i18n.tr("iced.field.port"), &state.model.draft.port)
            .on_input(Message::PortChanged)
            .width(iced::Length::FillPortion(1)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let user_row = row![
        text_input(i18n.tr("iced.field.user"), &state.model.draft.user)
            .on_input(Message::UserChanged)
            .width(iced::Length::FillPortion(1)),
        text_input(
            i18n.tr("iced.field.password"),
            state.model.draft.password.expose_secret(),
        )
        .secure(true)
        .on_input(Message::PasswordChanged)
        .width(iced::Length::FillPortion(1)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let auth_options: Vec<crate::session::AuthMethod> = vec![
        crate::session::AuthMethod::Password,
        crate::session::AuthMethod::Agent,
        crate::session::AuthMethod::Interactive,
        crate::session::AuthMethod::Key {
            private_key_path: String::new(),
        },
    ];
    let auth_row = row![pick_list(
        auth_options,
        Some(state.model.draft.auth.clone()),
        Message::QuickConnectAuthChanged
    )
    .width(iced::Length::Fill),]
    .spacing(10)
    .align_y(Alignment::Center);

    // Error/info banners
    let flow_banner: Option<Element<'_, Message>> = match flow {
        crate::app::state::QuickConnectFlow::NeedUser => Some(
            container(text(
                err_kind
                    .unwrap_or(crate::app::model::ConnectErrorKind::MissingHostOrUser)
                    .user_message(),
            ))
            .padding(10)
            .style(top_bar_material_style)
            .into(),
        ),
        crate::app::state::QuickConnectFlow::NeedAuthPassword => {
            let remaining = 3usize.saturating_sub(state.model.draft.password_error_count as usize);
            let msg = if err_kind == Some(crate::app::model::ConnectErrorKind::AuthFailed) {
                if remaining > 0 {
                    format!("SSH  密码错误，还剩 {} 次机会。", remaining)
                } else {
                    "SSH  密码错误次数已达上限。".to_string()
                }
            } else {
                "SSH  需要密码认证。".to_string()
            };
            Some(container(text(msg)).padding(10).style(top_bar_material_style).into())
        }
        crate::app::state::QuickConnectFlow::AuthLocked => Some(
            container(text("SSH  密码多次错误，已中断本次连接。请编辑后重试或切换认证方式。"))
                .padding(10)
                .style(top_bar_material_style)
                .into(),
        ),
        crate::app::state::QuickConnectFlow::Failed => Some(
            container(text(
                err_kind
                    .unwrap_or(crate::app::model::ConnectErrorKind::Unknown)
                    .user_message(),
            ))
            .padding(10)
            .style(top_bar_material_style)
            .into(),
        ),
        _ => None,
    };

    // Connecting progress
    let connecting_progress: Option<Element<'_, Message>> = if is_connecting {
        let dots = match state.tick_count % 3 {
            0 => "",
            1 => ".",
            _ => "..",
        };
        let stage_label = match stage {
            crate::app::state::ConnectionStage::VaultLoading => {
                state.model.i18n.tr("iced.stage.vault_loading")
            }
            crate::app::state::ConnectionStage::SshConnecting => {
                state.model.i18n.tr("iced.stage.ssh_connecting")
            }
            crate::app::state::ConnectionStage::Authenticating => {
                state.model.i18n.tr("iced.stage.authenticating")
            }
            crate::app::state::ConnectionStage::SessionSetup => {
                state.model.i18n.tr("iced.stage.session_setup")
            }
            _ => state.model.i18n.tr("iced.term.connecting"),
        };
        Some(
            container(
                row![text("⟳").size(14), text(format!("{stage_label}{dots}")).size(12)]
                    .spacing(6)
                    .align_y(Alignment::Center),
            )
            .padding(10)
            .style(top_bar_material_style)
            .into(),
        )
    } else {
        None
    };

    // Key auth fields
    let key_row: Option<Element<'_, Message>> = if matches!(
        state.model.draft.auth,
        crate::session::AuthMethod::Key { .. }
    ) {
        Some(
            column![
                text_input("Private key path", &state.model.draft.private_key_path)
                    .on_input(Message::QuickConnectKeyPathChanged)
                    .width(iced::Length::Fill),
                text_input(
                    "Passphrase (optional)",
                    state.model.draft.passphrase.expose_secret(),
                )
                .secure(true)
                .on_input(Message::QuickConnectPassphraseChanged)
                .width(iced::Length::Fill),
            ]
            .spacing(8)
            .into(),
        )
    } else {
        None
    };

    // Interactive auth fields
    let interactive_row: Option<Element<'_, Message>> = if matches!(
        state.quick_connect_flow,
        crate::app::state::QuickConnectFlow::NeedAuthInteractive
    ) {
        state.quick_connect_interactive.as_ref().map(|flow| {
            let mut col = column![
                text(flow.ui.name.clone()).size(13),
                text(flow.ui.instructions.clone()).size(12),
            ]
            .spacing(6)
            .width(iced::Length::Fill);
            for (i, p) in flow.ui.prompts.iter().enumerate() {
                let ans = flow.ui.answers.get(i).cloned().unwrap_or_default();
                col = col.push(
                    column![
                        text(p.prompt.clone()).size(12),
                        text_input("", &ans)
                            .secure(!p.echo)
                            .on_input(move |v| Message::QuickConnectInteractiveAnswerChanged(i, v))
                            .width(iced::Length::Fill),
                    ]
                    .spacing(4),
                );
            }
            if let Some(err) = flow.ui.error.as_ref() {
                col = col.push(text(err).size(12));
            }
            col = col.push(
                row![button(text("提交").size(13))
                    .on_press(Message::QuickConnectInteractiveSubmit)
                    .style(style_chrome_primary(13.0)),]
                .spacing(8),
            );
            container(col)
                .padding(12)
                .style(top_bar_material_style)
                .width(iced::Length::Fill)
                .into()
        })
    } else {
        None
    };

    // Saved session hint
    let saved_session_hint: Option<Element<'_, Message>> =
        state.model.selected_session_id.as_deref().and_then(|pid| {
            state
                .model
                .profiles()
                .iter()
                .find(|s| s.id == pid)
                .map(|p| {
                    text(format!(
                        "Saved session: {} — confirm below, then click Connect.",
                        p.name
                    ))
                    .size(12)
                    .style(|theme: &Theme| text::Style {
                        color: Some(
                            theme
                                .extended_palette()
                                .background
                                .base
                                .text
                                .scale_alpha(0.75),
                        ),
                    })
                    .into()
                })
        });

    // Action buttons
    let spinner = if state.tick_count % 2 == 0 { "◐" } else { "◓" };
    let connecting_label = i18n.tr("iced.btn.connecting");
    let connect_btn_text = if is_connecting {
        format!("{spinner} {connecting_label}")
    } else {
        i18n.tr("iced.btn.connect").to_string()
    };
    let actions = row![
        button(text(connect_btn_text).size(13))
            .on_press_maybe(
                (!matches!(
                    flow,
                    crate::app::state::QuickConnectFlow::Connecting
                        | crate::app::state::QuickConnectFlow::AuthLocked
                        | crate::app::state::QuickConnectFlow::NeedAuthInteractive
                ))
                .then_some(Message::ConnectPressed),
            )
            .style(style_chrome_primary(13.0)),
        button(text(i18n.tr("iced.btn.disconnect")).size(13))
            .on_press_maybe(is_connected.then_some(Message::DisconnectPressed))
            .style(style_chrome_secondary(13.0)),
        button(text(i18n.tr("iced.btn.save_session")).size(13))
            .on_press(Message::QuickConnectSaveSession)
            .style(style_chrome_secondary(13.0)),
        button(text(i18n.tr("iced.btn.save_settings")).size(13))
            .on_press(Message::SaveSettings)
            .style(style_chrome_secondary(13.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // Header row (with connecting spinner or back button)
    let header_row: Element<'_, Message> = if is_connecting {
        row![
            iced::widget::Space::new().width(iced::Length::Fixed(22.0)),
            text(format!("{spinner} {connecting_label}")).size(16),
            iced::widget::Space::new().width(iced::Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    } else {
        row![
            button(text(i18n.tr("iced.quick_connect.back")).size(12))
                .on_press(Message::QuickConnectBackToList)
                .style(style_chrome_secondary(12.0)),
            text(i18n.tr("iced.quick_connect.new_title")).size(16),
            iced::widget::Space::new().width(iced::Length::Fill),
            button(text("×").size(14))
                .on_press(Message::QuickConnectDismiss)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(28.0))
                .style(crate::app::widgets::chrome_button::style_top_icon(14.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    };

    // Build form
    let mut form_cols = column![header_row, text(i18n.tr("iced.title.subtitle")).size(12)]
        .spacing(14)
        .width(iced::Length::Fill);
    if let Some(b) = flow_banner {
        form_cols = form_cols.push(b);
    }
    if let Some(p) = connecting_progress {
        form_cols = form_cols.push(p);
    }
    if let Some(h) = saved_session_hint {
        form_cols = form_cols.push(h);
    }
    let mut form_cols = form_cols.push(host_row).push(auth_row).push(user_row);
    if let Some(k) = key_row {
        form_cols = form_cols.push(k);
    }
    if let Some(ir) = interactive_row {
        form_cols = form_cols.push(ir);
    }

    let switch_auth_btn: Option<Element<'_, Message>> = if matches!(
        flow,
        crate::app::state::QuickConnectFlow::Failed
            | crate::app::state::QuickConnectFlow::AuthLocked
    ) {
        Some(
            button(text(i18n.tr("iced.btn.switch_auth")).size(13))
                .on_press(Message::QuickConnectSwitchAuth)
                .style(style_chrome_secondary(13.0))
                .into(),
        )
    } else {
        None
    };

    let body = if let Some(btn) = switch_auth_btn {
        form_cols.push(row![btn].spacing(8).align_y(Alignment::Center)).push(actions).spacing(14)
    } else {
        form_cols.push(actions).spacing(14)
    };

    container(body)
        .width(iced::Length::Fill)
        .padding(16)
        .style(top_bar_material_style)
        .into()
}
