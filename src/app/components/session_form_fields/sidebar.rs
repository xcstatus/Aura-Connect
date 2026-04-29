//! 会话表单侧边栏组件
//! 提供左侧导航栏，支持 General/Advanced/Port Forwarding/Encryption 四个页面切换

use iced::alignment::{Alignment, Horizontal};
use iced::widget::{Space, column, container, row, text};
use iced::{Background, Color, Element, Length, Theme};

use crate::app::message::Message;
use crate::i18n::I18n;
use crate::theme::DesignTokens;
use crate::theme::layout;

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
        (
            SidebarPage::General,
            i18n.tr("session_form.sidebar.general"),
        ),
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

/// 创建侧边栏选项 - Liquid Glass 风格
fn sidebar_item<'a>(
    label: &'a str,
    is_active: bool,
    tokens: &DesignTokens,
) -> Element<'a, Message> {
    // Liquid Glass: 选中项使用 glass_strong + accent 边框
    let bg = if is_active {
        tokens.glass_strong
    } else {
        tokens.glass_subtle
    };

    let text_color = if is_active {
        tokens.text_primary
    } else {
        tokens.text_secondary
    };

    // Liquid Glass: 选中项左侧显示 accent 强调线
    let accent_color = tokens.accent_base;

    let border = if is_active {
        iced::Border {
            width: 1.0,
            color: if tokens.is_dark() {
                Color::from_rgba(1.0, 1.0, 1.0, 0.08)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.08)
            },
            radius: layout::RADIUS_TAB.into(),
        }
    } else {
        iced::Border::default()
    };

    // 选中项添加左侧 accent 强调线
    let content: Element<'a, Message> = if is_active {
        container(
            row![
                container(Space::new().width(Length::Fixed(3.0))).style(move |_: &Theme| {
                    container::Style {
                        background: Some(Background::Color(accent_color)),
                        border: iced::Border {
                            width: 0.0,
                            color: Color::TRANSPARENT,
                            radius: 0.0.into(),
                        },
                        ..Default::default()
                    }
                }),
                container(text(label).size(14).color(text_color),)
                    .width(Length::Fill)
                    .height(Length::Fixed(44.0))
                    .padding(iced::Padding::from([0, 9]))
                    .align_x(Horizontal::Left)
                    .align_y(Alignment::Center),
            ]
            .spacing(0),
        )
        .width(Length::Fill)
        .height(Length::Fixed(44.0))
        .into()
    } else {
        container(text(label).size(14).color(text_color))
            .width(Length::Fill)
            .height(Length::Fixed(44.0))
            .padding(iced::Padding::from([0, 12]))
            .align_x(Horizontal::Left)
            .align_y(Alignment::Center)
            .into()
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fixed(44.0))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(bg)),
            border,
            ..Default::default()
        })
        .into()
}
