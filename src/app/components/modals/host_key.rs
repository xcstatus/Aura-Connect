use iced::alignment::Alignment;
use iced::Element;
use iced::widget::{button, column, container, row, text, Space};

use crate::app::components::helpers::layered_scrim_style;
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::widgets::chrome_button::{style_chrome_primary, style_chrome_secondary, style_top_icon};

/// Build the host key confirmation prompt modal.
pub(crate) fn host_key_prompt_modal(state: &IcedState) -> Element<'_, Message> {
    let Some(p) = state.host_key_prompt.as_ref() else {
        return Space::new().into()
    };

    let i18n = &state.model.i18n;
    let info = &p.info;

    let scrim = container(Space::new().width(iced::Length::Fill).height(iced::Length::Fill))
        .style(|theme: &iced::Theme| layered_scrim_style(theme, 0));

    let mut body = column![
        row![
            text(i18n.tr("iced.host_key_prompt.title")).size(16),
            Space::new().width(iced::Length::Fill),
            button(text("×").size(14))
                .on_press(Message::HostKeyReject)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(28.0))
                .style(style_top_icon(14.0)),
        ]
        .align_y(Alignment::Center),
        text(i18n.tr_fmt(
            "iced.host_key_prompt.host_line",
            &[
                ("host", &info.host),
                ("port", &info.port.to_string()),
                ("algo", &info.algo),
            ],
        ))
        .size(12),
    ]
    .spacing(10)
    .width(iced::Length::Fill);

    if let Some(old) = info.old_fingerprint.as_ref() {
        body = body.push(
            text(i18n.tr_fmt("iced.host_key_prompt.old_fingerprint", &[("fp", old)])).size(12),
        );
    }

    body = body
        .push(
            text(i18n.tr_fmt(
                "iced.host_key_prompt.new_fingerprint",
                &[("fp", &info.fingerprint)],
            ))
            .size(12),
        )
        .push(
            text(match state.model.settings.security.host_key_policy {
                crate::settings::HostKeyPolicy::Strict => i18n.tr("settings.security.hosts.policy.strict"),
                crate::settings::HostKeyPolicy::Ask => i18n.tr("settings.security.hosts.policy.ask"),
                crate::settings::HostKeyPolicy::AcceptNew => i18n.tr("settings.security.hosts.policy.accept_new"),
            })
            .size(12),
        )
        .push(
            row![
                button(text(i18n.tr("iced.host_key_prompt.accept_once")).size(13))
                    .on_press(Message::HostKeyAcceptOnce)
                    .style(style_chrome_secondary(13.0)),
                button(text(i18n.tr("iced.host_key_prompt.always_trust")).size(13))
                    .on_press(Message::HostKeyAlwaysTrust)
                    .style(style_chrome_primary(13.0)),
                button(text(i18n.tr("iced.host_key_prompt.reject")).size(13))
                    .on_press(Message::HostKeyReject)
                    .style(style_chrome_secondary(13.0)),
            ]
            .spacing(8),
        );

    let card = container(body)
        .width(iced::Length::Fixed(560.0))
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
