use iced::Element;
use iced::alignment::Alignment;
use iced::widget::{Space, button, column, container, row, text};

use crate::app::components::helpers::{
    layered_scrim_style, tokens_for_state, top_bar_material_style,
};
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::widgets::chrome_button::{lg_icon_button, lg_primary_button, lg_secondary_button};
use crate::theme::icons::{IconId, IconOptions, icon_view_with};

/// Build the auto-probe consent modal.
pub(crate) fn auto_probe_consent_modal(state: &IcedState) -> Element<'_, Message> {
    let Some(_m) = state.auto_probe_consent_modal.as_ref() else {
        return Space::new().into();
    };

    let tokens = tokens_for_state(state);

    let scrim = container(
        Space::new()
            .width(iced::Length::Fill)
            .height(iced::Length::Fill),
    )
    .style(layered_scrim_style(tokens, 0));

    let body = column![
        row![
            text("首次自动探测提示").size(16),
            Space::new().width(iced::Length::Fill),
            icon_close_button(tokens),
        ]
        .align_y(Alignment::Center),
        text("将尝试使用系统 SSH Agent 或本机密钥进行认证。不会上传私钥，仅在本机使用。").size(12),
        row![
            button(text("允许（本次）").size(13))
                .on_press(Message::AutoProbeConsentAllowOnce)
                .style(lg_secondary_button(tokens)),
            button(text("始终允许").size(13))
                .on_press(Message::AutoProbeConsentAlwaysAllow)
                .style(lg_primary_button(tokens)),
            button(text("改用密码").size(13))
                .on_press(Message::AutoProbeConsentUsePassword)
                .style(lg_secondary_button(tokens)),
        ]
        .spacing(8),
    ]
    .spacing(10)
    .width(iced::Length::Fill);

    let card = container(body)
        .width(iced::Length::Fixed(560.0))
        .padding(16)
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
        Message::AutoProbeConsentUsePassword,
    );
    button(close_icon)
        .on_press(Message::AutoProbeConsentUsePassword)
        .width(iced::Length::Fixed(28.0))
        .height(iced::Length::Fixed(28.0))
        .style(lg_icon_button(tokens))
        .into()
}
