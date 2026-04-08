use iced::alignment::Alignment;
use iced::widget::{
    button, column, container, row, scrollable, text, text_input,
};
use iced::Element;
use iced::Theme;

use crate::app::components::helpers::{top_bar_material_style, quick_connect_group_header_style, tokens_for_state};
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::widgets::chrome_button::{style_chrome_primary, style_chrome_secondary};

use super::grouped_ssh_profiles;
use crate::session::{SessionProfile, TransportConfig};

/// Quick connect picker panel: shows recent connections, saved profiles, and search.
pub(crate) fn quick_connect_picker(state: &IcedState) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let tokens = tokens_for_state(state);

    let query = state.quick_connect_query.trim();
    let direct_parts = crate::app::connection::parse_direct_input(query);
    let is_direct = crate::app::connection::is_direct_candidate(query) && direct_parts.is_some();
    let direct_label = direct_parts.as_ref().map(|p| {
        let user = p.user.as_deref().unwrap_or("<user>");
        let port = p.port.unwrap_or(22);
        format!("{user}@{}:{port}", p.host)
    });
    let direct_cta: Element<'_, Message> = if is_direct && direct_label.is_some() {
        button(
            text(i18n.tr_fmt(
                "iced.quick_connect.direct_cta",
                &[("target", direct_label.as_deref().unwrap_or(""))],
            ))
            .size(13),
        )
        .on_press(Message::QuickConnectDirectSubmit)
        .width(iced::Length::Fill)
        .padding([8, 12])
        .style(style_chrome_primary(tokens))
        .into()
    } else {
        iced::widget::Space::new().into()
    };

    // Recent connections section
    let mut recent_block = column![text(i18n.tr("iced.quick_connect.recent")).size(13)]
        .spacing(6)
        .align_x(Alignment::Start);
    let recent = state.model.recent_connections();
    if recent.is_empty() {
        recent_block = recent_block.push(
            text(i18n.tr("iced.quick_connect.empty_recent"))
                .size(12)
                .style(|theme: &Theme| text::Style {
                    color: Some(
                        theme
                            .extended_palette()
                            .background
                            .base
                            .text
                            .scale_alpha(0.55),
                    ),
                }),
        );
    } else {
        for r in recent.iter() {
            let rec = r.clone();
            let subtitle = format!("{} · {}", r.user, r.host);
            recent_block = recent_block.push(
                button(
                    column![
                        text(r.label.clone()).size(13),
                        text(subtitle).size(11).style(|theme: &Theme| text::Style {
                            color: Some(
                                theme
                                    .extended_palette()
                                    .background
                                    .base
                                    .text
                                    .scale_alpha(0.6)
                            ),
                        }),
                    ]
                    .spacing(2)
                    .align_x(Alignment::Start),
                )
                .on_press(Message::QuickConnectPickRecent(rec))
                .width(iced::Length::Fill)
                .padding([6, 10])
                .style(style_chrome_secondary(tokens)),
            );
        }
    }

    // Saved profiles section
    let mut saved_block = column![text(i18n.tr("iced.quick_connect.saved")).size(13)]
        .spacing(8)
        .align_x(Alignment::Start);
    saved_block = saved_block.push(
        button(text(i18n.tr("iced.quick_connect.new_connection")).size(13))
            .on_press(Message::QuickConnectNewConnection)
            .width(iced::Length::Fill)
            .padding([8, 12])
            .style(style_chrome_primary(tokens)),
    );

    let q_lower = query.to_lowercase();
    let saved_profiles: Vec<SessionProfile> = if !q_lower.is_empty() && !is_direct {
        state
            .model
            .profiles()
            .iter()
            .filter(|p| matches!(p.transport, TransportConfig::Ssh(_)))
            .filter(|p| {
                let mut hay = p.name.to_lowercase();
                if let TransportConfig::Ssh(ssh) = &p.transport {
                    hay.push(' ');
                    hay.push_str(&ssh.host.to_lowercase());
                    hay.push(' ');
                    hay.push_str(&ssh.user.to_lowercase());
                }
                hay.contains(&q_lower)
            })
            .cloned()
            .collect()
    } else {
        state
            .model
            .profiles()
            .iter()
            .filter(|p| matches!(p.transport, TransportConfig::Ssh(_)))
            .cloned()
            .collect()
    };

    let default_label = i18n.tr("iced.quick_connect.group_default").to_string();
    for (group_key, sessions) in grouped_ssh_profiles(&saved_profiles) {
        let display = if group_key == "__default__" {
            default_label.clone()
        } else {
            group_key.clone()
        };
        let group_display = display.clone();
        saved_block = saved_block.push(
            container(text(group_display).size(12))
                .padding(iced::Padding {
                    top: 6.0,
                    right: 0.0,
                    bottom: 4.0,
                    left: 4.0,
                })
                .style(quick_connect_group_header_style(tokens))
                .width(iced::Length::Fill),
        );
        for p in sessions {
            let entry_title = p.name.clone();
            let subtitle = if let TransportConfig::Ssh(ssh) = &p.transport {
                format!("{} · {}", ssh.user, ssh.host)
            } else {
                String::new()
            };
            let prof = p.clone();
            saved_block = saved_block.push(
                button(
                    column![
                        text(entry_title).size(13),
                        text(subtitle).size(11).style(|theme: &Theme| text::Style {
                            color: Some(
                                theme
                                    .extended_palette()
                                    .background
                                    .base
                                    .text
                                    .scale_alpha(0.6)
                            ),
                        }),
                    ]
                    .spacing(2)
                    .align_x(Alignment::Start),
                )
                .on_press(Message::ProfileConnect(prof))
                .width(iced::Length::Fill)
                .padding([6, 10])
                .style(style_chrome_secondary(tokens)),
            );
        }
    }

    let body = column![
        row![
            text(i18n.tr("iced.topbar.quick_connect")).size(16),
            iced::widget::Space::new().width(iced::Length::Fill),
            button(text("×").size(14))
                .on_press(Message::QuickConnectDismiss)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(28.0))
                .style(crate::app::widgets::chrome_button::style_top_icon(tokens)),
        ]
        .align_y(Alignment::Center),
        text_input(
            i18n.tr("iced.quick_connect.search_or_direct"),
            &state.quick_connect_query
        )
        .on_input(Message::QuickConnectQueryChanged)
        .on_submit(Message::QuickConnectDirectSubmit)
        .padding([8, 10]),
        direct_cta,
        scrollable(
            column![recent_block, saved_block]
                .spacing(16)
                .width(iced::Length::Fill),
        )
        .height(iced::Length::Fixed(440.0))
        .width(iced::Length::Fill),
    ]
    .spacing(10)
    .width(iced::Length::Fill);

    container(body)
        .width(iced::Length::Fill)
        .padding(16)
        .style(top_bar_material_style(tokens))
        .into()
}
