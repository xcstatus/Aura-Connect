//! SFTP 文件列表组件

use iced::alignment::Vertical;
use iced::widget::{button, container, row, scrollable, text, Column, Space};
use iced::{Element, Length};

use crate::app::components::helpers::tokens_for_state;
use crate::app::message::Message;
use crate::app::state::{IcedState, SftpPanel};
use crate::sftp::{RemoteFileEntry, SftpSortBy};
use crate::theme::{layout, DesignTokens};
use crate::theme::icons::{icon, icon_view_with, IconId, IconOptions, IconState};

/// 构建 SFTP 文件列表
pub(crate) fn file_list<'a>(state: &'a IcedState, sftp: &'a SftpPanel) -> Element<'a, Message> {
    let tokens = tokens_for_state(state);
    let i18n = &state.model.i18n;

    // 按排序规则排序
    let entries = sort_entries(sftp.entries.clone(), sftp.sort_by, sftp.sort_direction);

    // 根据加载状态显示不同内容
    if sftp.is_loading {
        return make_empty_column(i18n.tr("iced.sftp.loading"), tokens.text_secondary);
    }

    // 如果有错误信息
    if let Some(error) = &sftp.error {
        return make_empty_column(error.as_str(), tokens.error);
    }

    // 如果列表为空
    if entries.is_empty() {
        return make_empty_column(i18n.tr("iced.sftp.empty"), tokens.text_secondary);
    }

    // 分离目录和文件
    let (dirs, files): (Vec<_>, Vec<_>) = entries.into_iter().partition(|e| e.is_dir);

    // 构建列
    let mut list_column = Column::new();

    // 添加目录
    for entry in dirs {
        let row_elem = build_file_row(state, entry);
        list_column = list_column.push(row_elem);
    }

    // 添加文件
    for entry in files {
        let row_elem = build_file_row(state, entry);
        list_column = list_column.push(row_elem);
    }

    // 使用滚动容器
    let scrollable_list: scrollable::Scrollable<'a, Message> = scrollable(list_column.spacing(0))
        .width(Length::Fill)
        .height(Length::Fill);

    container(scrollable_list)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(flat_container_style())
        .into()
}

/// 创建一个居中显示文本的列
fn make_empty_column<'a>(msg: &'a str, color: iced::Color) -> Element<'a, Message> {
    let col: Column<'a, Message> = Column::new()
        .push(Space::new().height(Length::Fill))
        .push(text(msg).size(12).color(color))
        .push(Space::new().height(Length::Fill))
        .align_x(iced::alignment::Horizontal::Center);

    container(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(flat_container_style())
        .into()
}

/// 构建单个文件行
fn build_file_row<'a>(state: &'a IcedState, entry: RemoteFileEntry) -> Element<'a, Message> {
    // 克隆所有需要的数据为 owned types
    let name = entry.name.clone();
    let path_str = entry.path.to_string_lossy().into_owned();
    let size_human = entry.size_human();
    let modified_human = entry.modified_human();
    let is_dir = entry.is_dir;
    let is_hidden = entry.is_hidden();
    let permissions = entry.permissions.clone();

    // 获取 tokens
    let tokens = tokens_for_state(state);

    // 判断是否为上一级目录
    let is_parent_dir = name == "..";

    // 文件名颜色
    let name_color = if is_hidden { tokens.text_secondary } else { tokens.text_primary };

    // 大小文本
    let size_text = if is_dir { "-".to_string() } else { size_human };

    // 创建消息
    let msg: Message = if is_dir {
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpNavigate(path_str.clone()))
    } else {
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpDownload(path_str.clone()))
    };

    // 文件图标（上一级显示左箭头，目录显示文件夹，文件显示文档）
    let icon_elem: Element<'a, Message> = if is_parent_dir {
        icon_view_with(IconOptions::new(IconId::ArrowLeft).with_size(12), msg.clone())
    } else if is_dir {
        icon_view_with(IconOptions::new(IconId::Folder).with_size(12), msg.clone())
    } else {
        icon_view_with(IconOptions::new(IconId::File).with_size(12), msg.clone())
    };

    // 文件名
    let name_elem: Element<'a, Message> = text(name)
        .size(12)
        .color(name_color)
        .into();

    // 权限文本（隐藏文件显示空字符串）
    let perm_text = if is_hidden || permissions.is_empty() {
        String::new()
    } else {
        permissions
    };

    // 权限
    let perm_elem: Element<'a, Message> = text(perm_text)
        .size(11)
        .color(tokens.text_secondary)
        .into();

    // 大小
    let size_elem: Element<'a, Message> = text(size_text)
        .size(11)
        .color(tokens.text_secondary)
        .into();

    // 修改时间
    let modified_elem: Element<'a, Message> = text(modified_human)
        .size(11)
        .color(tokens.text_secondary)
        .into();

    // 整体布局（行）
    let row_elem: Element<'a, Message> = row![
        icon_elem,
        Space::new().width(Length::Fixed(layout::SFTP_COL_NAME_ICON_SPACING)),
        name_elem,
        Space::new().width(Length::Fill),
        container(perm_elem).width(Length::Fixed(layout::SFTP_COL_PERMISSIONS)),
        Space::new().width(Length::Fixed(layout::SFTP_COL_SPACING)),
        container(size_elem).width(Length::Fixed(layout::SFTP_COL_SIZE)),
        Space::new().width(Length::Fixed(layout::SFTP_COL_SPACING)),
        container(modified_elem).width(Length::Fixed(layout::SFTP_COL_MODIFIED)),
    ]
    .spacing(0)
    .align_y(Vertical::Center)
    .into();

    button(row_elem)
            .padding([4, 12])
            .width(Length::Fill)
            .height(Length::Fixed(layout::SFTP_FILE_ROW_HEIGHT))
            .on_press(msg)
            .style(flat_button_style(tokens))
            .into()
}

/// 根据排序规则排序文件列表
fn sort_entries(
    entries: Vec<RemoteFileEntry>,
    sort_by: SftpSortBy,
    direction: crate::sftp::SortDirection,
) -> Vec<RemoteFileEntry> {
    let desc = direction.is_descending();

    let mut entries = entries;
    entries.sort_by(|a, b| {
        // 目录优先排序
        if a.is_dir != b.is_dir {
            return if a.is_dir { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater };
        }

        let cmp = match sort_by {
            SftpSortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SftpSortBy::Size => a.size.cmp(&b.size),
            SftpSortBy::Modified => a.modified.cmp(&b.modified),
        };

        if desc {
            cmp.reverse()
        } else {
            cmp
        }
    });

    entries
}

/// 扁平容器样式
fn flat_container_style() -> impl Fn(&iced::Theme) -> iced::widget::container::Style + 'static {
    move |_: &iced::Theme| {
        iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
            ..Default::default()
        }
    }
}

/// 扁平按钮样式
fn flat_button_style(tokens: DesignTokens) -> impl Fn(&iced::Theme, button::Status) -> iced::widget::button::Style + 'static {
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

// ============== 返回上级目录行 ==============

/// 构建返回上级目录行
pub(crate) fn parent_row<'a>(state: &'a IcedState, sftp: &'a SftpPanel) -> Element<'a, Message> {
    let tokens = tokens_for_state(state);
    let i18n = &state.model.i18n;

    let show_parent = !sftp.current_path.is_empty() && sftp.current_path != "/";
    if !show_parent {
        return Space::new().width(Length::Fill).height(Length::Fixed(0.0)).into();
    }

    let back_icon: Element<'a, ()> = icon(IconId::ArrowLeft, &tokens, IconState::Default);
    let back_text = text(i18n.tr("iced.sftp.btn.parent"))
        .size(12)
        .color(tokens.text_secondary);

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
            .style(flat_button_style(tokens))
            .width(Length::Fill)
            .height(Length::Fixed(layout::SFTP_FILE_ROW_HEIGHT))
            .on_press(Message::SftpTab(crate::app::message::SftpTabMessage::SftpNavigateToParent)),
    )
    .width(Length::Fill)
    .into()
}