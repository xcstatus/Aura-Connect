use iced::alignment::Alignment;
use iced::Element;
use iced::widget::{button, column, container, row, text, Space};

use crate::app::components::helpers::layered_scrim_style;
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::widgets::chrome_button::{style_chrome_secondary, style_chrome_primary, style_top_icon};

/// Build the auto-probe consent modal.
pub(crate) fn auto_probe_consent_modal(state: &IcedState) -> Element<'_, Message> {
    let Some(_m) = state.auto_probe_consent_modal.as_ref() else {
        return Space::new().into()
    };

    let scrim = container(Space::new().width(iced::Length::Fill).height(iced::Length::Fill))
        .style(|theme: &iced::Theme| layered_scrim_style(theme, 0));

    let body = column![
        row![
            text("首次自动探测提示").size(16),
            Space::new().width(iced::Length::Fill),
            button(text("×").size(14))
                .on_press(Message::AutoProbeConsentUsePassword)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(28.0))
                .style(style_top_icon(14.0)),
        ]
        .align_y(Alignment::Center),
        text("将尝试使用系统 SSH Agent 或本机密钥进行认证。不会上传私钥，仅在本机使用。").size(12),
        row![
            button(text("允许（本次）").size(13))
                .on_press(Message::AutoProbeConsentAllowOnce)
                .style(style_chrome_secondary(13.0)),
            button(text("始终允许").size(13))
                .on_press(Message::AutoProbeConsentAlwaysAllow)
                .style(style_chrome_primary(13.0)),
            button(text("改用密码").size(13))
                .on_press(Message::AutoProbeConsentUsePassword)
                .style(style_chrome_secondary(13.0)),
        ]
        .spacing(8),
    ]
    .spacing(10)
    .width(iced::Length::Fill);

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
