//! SFTP 返回上级目录行组件

use iced::widget::{button, container, row, text, Space};
use iced::{Element, Length};

use crate::app::components::helpers::tokens_for_state;
use crate::app::message::Message;
use crate::app::state::{IcedState, SftpPanel};
use crate::theme::icons::{icon, IconId, IconState};

/// 构建返回上级目录行
pub(crate) fn parent_row<'a>(state: &'a IcedState, sftp: &'a SftpPanel) -> Element<'a, Message> {
    let tokens = tokens_for_state(state);
    let i18n = &state.model.i18n;

    // 判断是否需要显示返回上级行（当不在根目录时）
    let show_parent = !sftp.current_path.is_empty() && sftp.current_path != "/";

    if !show_parent {
        return Space::new().width(Length::Fill).height(Length::Fixed(0.0)).into();
    }

    // 返回上级图标
    let back_icon: Element<'a, ()> = icon(IconId::ArrowLeft, &tokens, IconState::Default);

    // 返回上级文字
    let back_text = text(i18n.tr("iced.sftp.btn.parent"))
        .size(12)
        .color(tokens.text_secondary);

    // 可点击的行
    let content: Element<'a, Message> = row![
        back_icon.map(|_| Message::SftpTab(crate::app::message::SftpTabMessage::SftpNavigateToParent)),
        text(" ").color(tokens.text_secondary),
        back_text,
    ]
    .spacing(6)
    .align_y(iced::alignment::Vertical::Center)
    .into();

    container(
        button(content)
            .padding([4, 12])
            .style(parent_button_style(tokens))
            .on_press(Message::SftpTab(crate::app::message::SftpTabMessage::SftpNavigateToParent))
    )
    .width(Length::Fill)
    .into()
}

/// 返回上级按钮样式
fn parent_button_style(tokens: crate::theme::DesignTokens) -> impl Fn(&iced::Theme, button::Status) -> iced::widget::button::Style + 'static {
    let surface_2 = tokens.surface_2;
    let surface_3 = tokens.surface_3;
    move |_: &iced::Theme, status: button::Status| {
        let mut style = iced::widget::button::Style::default();

        match status {
            button::Status::Hovered => {
                style.background = Some(iced::Background::Color(surface_2));
            }
            button::Status::Pressed => {
                style.background = Some(iced::Background::Color(surface_3));
            }
            _ => {}
        }

        style
    }
}
