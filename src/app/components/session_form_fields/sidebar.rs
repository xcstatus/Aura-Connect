//! 会话表单侧边栏组件
//! 提供左侧导航栏，支持 General/Advanced/Port Forwarding/Encryption 四个页面切换

use iced::alignment::{Alignment, Horizontal};
use iced::widget::{column, container, text};
use iced::{Element, Length, Theme};

use crate::app::message::Message;
use crate::i18n::I18n;
use crate::theme::DesignTokens;

/// 侧边栏页面枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarPage {
    General,
    Advanced,
    PortForward,
    Encryption,
}

/// 创建侧边栏组件
pub fn sidebar<'a>(
    current_page: &SidebarPage,
    tokens: DesignTokens,
    i18n: &'a I18n,
) -> Element<'a, Message> {
    let pages = vec![
        (SidebarPage::General, i18n.tr("session_form.sidebar.general")),
        (
            SidebarPage::Advanced,
            i18n.tr("session_form.sidebar.advanced"),
        ),
        (
            SidebarPage::PortForward,
            i18n.tr("session_form.sidebar.port_forward"),
        ),
        (
            SidebarPage::Encryption,
            i18n.tr("session_form.sidebar.encryption"),
        ),
    ];

    let mut items = column![];
    for (page, label) in pages {
        let is_active = *current_page == page;
        let item = sidebar_item(label, is_active, &tokens);
        items = items.push(item);
    }

    let bg_color = tokens.bg_secondary;
    container(items)
        .width(Length::Fixed(200.0))
        .height(Length::Fill)
        .padding(32.0)
        .style(move |_: &Theme| container::Style {
            background: Some(bg_color.into()),
            ..Default::default()
        })
        .into()
}

/// 创建侧边栏选项
fn sidebar_item<'a>(label: &'a str, is_active: bool, tokens: &DesignTokens) -> Element<'a, Message> {
    let bg = if is_active { tokens.surface_2 } else { tokens.surface_1 };

    let text_color = if is_active {
        tokens.text_primary
    } else {
        tokens.text_secondary
    };

    let border = if is_active {
        iced::Border {
            width: 2.0,
            color: tokens.accent_base,
            radius: 6.0.into(),
        }
    } else {
        iced::Border::default()
    };

    container(
        text(label).size(14).color(text_color),
    )
    .width(Length::Fill)
    .height(Length::Fixed(40.0))
    .padding(iced::Padding::from([0, 12]))
    .align_x(Horizontal::Left)
    .align_y(Alignment::Center)
    .style(move |_: &Theme| container::Style {
        background: Some(bg.into()),
        border,
        ..Default::default()
    })
    .into()
}
