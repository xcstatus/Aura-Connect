//! Welcome page view — shown at app start and when all tabs are closed.

use iced::alignment::Alignment;
use iced::widget::{button, column, container, text};
use iced::Element;
use iced::Theme;

use crate::app::components::helpers::tokens_for_state;
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::widgets::chrome_button::{style_chrome_primary, style_chrome_secondary};

/// Welcome page: quick connect + recent connections + settings.
/// Shown when `state.show_welcome` is true (app start or all tabs closed).
pub(crate) fn welcome_view(state: &IcedState) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let tokens = tokens_for_state(state);

    let quick_connect_btn = button(
        text(i18n.tr("iced.welcome.quick_connect"))
            .size(14),
    )
    .on_press(Message::TopQuickConnect)
    .width(iced::Length::Fill)
    .padding([12, 16])
    .style(style_chrome_primary(tokens));

    let recent_label = container(text(i18n.tr("iced.welcome.recent")).size(11))
        .padding(iced::Padding {
            top: 0.0,
            right: 0.0,
            bottom: 8.0,
            left: 4.0,
        })
        .width(iced::Length::Fill);

    // Recent connections section
    let recent = state.model.recent_connections();
    let recent_entries: Element<'_, Message> = if recent.is_empty() {
        text(i18n.tr("iced.quick_connect.empty_recent"))
            .size(12)
            .style(|theme: &Theme| text::Style {
                color: Some(
                    theme.extended_palette().background.base.text.scale_alpha(0.45),
                ),
            })
            .into()
    } else {
        let mut col = column![].spacing(6).align_x(Alignment::Start);
        for r in recent.iter().take(5) {
            let rec = r.clone();
            let subtitle = format!("{} · {}", r.user, r.host);
            col = col.push(
                button(
                    column![
                        text(r.label.clone()).size(13),
                        text(subtitle).size(11).style(|theme: &Theme| text::Style {
                            color: Some(
                                theme.extended_palette().background.base.text.scale_alpha(0.6),
                            ),
                        }),
                    ]
                    .spacing(2)
                    .align_x(Alignment::Start),
                )
                .on_press(Message::QuickConnectPickRecent(rec))
                .width(iced::Length::Fill)
                .padding([8, 12])
                .style(style_chrome_secondary(tokens)),
            );
        }
        col.into()
    };

    let settings_btn = button(text(i18n.tr("iced.welcome.settings")).size(13))
        .on_press(Message::TopOpenSettings)
        .width(iced::Length::Fill)
        .padding([8, 12])
        .style(style_chrome_secondary(tokens));

    let body = column![
        iced::widget::Space::new().height(iced::Length::FillPortion(2)),
        text("RustSSH").size(28),
        iced::widget::Space::new().height(iced::Length::Fixed(24.0)),
        quick_connect_btn,
        iced::widget::Space::new().height(iced::Length::Fixed(20.0)),
        recent_label,
        recent_entries,
        iced::widget::Space::new().height(iced::Length::Fixed(16.0)),
        // Subtle divider line between recent connections and settings
        container(iced::widget::Space::new().height(iced::Length::Fixed(1.0)))
            .padding(iced::Padding {
                left: 40.0,
                right: 40.0,
                top: 0.0,
                bottom: 0.0,
            }),
        iced::widget::Space::new().height(iced::Length::Fixed(16.0)),
        settings_btn,
        iced::widget::Space::new().height(iced::Length::FillPortion(3)),
    ]
    .spacing(0)
    .width(iced::Length::Fixed(320.0))
    .align_x(Alignment::Center);

    container(body)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .center_x(iced::Length::Fill)
        .center_y(iced::Length::Fill)
        .into()
}
