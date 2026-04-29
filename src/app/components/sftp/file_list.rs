//! SFTP 文件列表组件
//!
//! 提供 Liquid Glass 风格的文件列表，包括：
//! - 冻结表头（不随滚动）
//! - 返回上级目录行（固定在表头下方，不受排序影响）
//! - 可排序的表头行
//! - 选中态高亮

use iced::alignment::Vertical;
use iced::widget::{Column, Space, button, container, row, scrollable, text};
use iced::{Background, Border, Color, Element, Length};

use crate::app::components::helpers::tokens_for_state;
use crate::app::message::Message;
use crate::app::state::{IcedState, SftpPanel};
use crate::sftp::{RemoteFileEntry, SftpSortBy, SortDirection};
use crate::theme::icons::{IconId, IconOptions, IconState, icon, icon_view_with};
use crate::theme::{DesignTokens, layout};

// ============================================================================
// Public API
// ============================================================================

/// 构建 SFTP 文件列表（主入口）
///
/// 布局结构：
/// - 表头行（冻结，不滚动）
/// - scrollable 内：
///   - 返回上级目录行（第一行，不受排序影响）
///   - 文件行列表
pub(crate) fn file_list<'a>(state: &'a IcedState, sftp: &'a SftpPanel) -> Element<'a, Message> {
    let tokens = tokens_for_state(state);
    let i18n = &state.model.i18n;

    // 如果有错误信息
    if let Some(error) = &sftp.error {
        return make_empty_state(error.as_str(), tokens.error);
    }

    // 正在加载时显示 loading 状态
    if sftp.is_loading {
        return make_empty_state(i18n.tr("iced.sftp.loading"), tokens.text_secondary);
    }

    // 按排序规则排序
    let mut entries = sort_entries(sftp.entries.clone(), sftp.sort_by, sftp.sort_direction);

    // 过滤隐藏文件
    if !sftp.show_hidden {
        entries.retain(|e| !e.is_hidden());
    }

    // 如果列表为空（但可能仍需显示返回上级）
    let show_parent = !sftp.current_path.is_empty() && sftp.current_path != "/";
    if entries.is_empty() && !show_parent {
        return make_empty_state(i18n.tr("iced.sftp.empty"), tokens.text_secondary);
    }

    // 构建滚动区域内的行
    let mut list_column = Column::new().spacing(1);

    // 返回上级目录行（固定在第一行，不受排序影响）
    if show_parent {
        list_column = list_column.push(build_parent_row(tokens, i18n.tr("iced.sftp.btn.parent")));
    }

    // 文件行
    for entry in entries {
        let row_elem = build_file_row(entry, tokens);
        list_column = list_column.push(row_elem);
    }

    // 滚动容器（仅文件行滚动）
    let scrollable_list = scrollable(list_column)
        .width(Length::Fill)
        .height(Length::Fill);

    // 表头冻结在顶部，scrollable 在下方
    iced::widget::column![
        header_row(state, sftp),
        container(scrollable_list)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([4, 0]),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

// ============================================================================
// Parent row (返回上级目录)
// ============================================================================

/// 构建返回上级目录行（内部函数，作为列表第一行）
fn build_parent_row<'a>(tokens: DesignTokens, label: &'a str) -> Element<'a, Message> {
    let back_icon: Element<'a, Message> = icon(IconId::NavArrowLeft, &tokens, IconState::Default)
        .map(|_| Message::SftpTab(crate::app::message::SftpTabMessage::SftpNavigateToParent));
    let back_text = text(label)
        .size(12)
        .color(tokens.text_secondary);

    let content: Element<'a, Message> = row![
        back_icon,
        text(" ").color(tokens.text_secondary),
        back_text,
    ]
    .spacing(6)
    .align_y(Vertical::Center)
    .into();

    button(content)
        .padding([layout::SFTP_ROW_PADDING_V as u16, layout::SFTP_ROW_PADDING_H as u16])
        .style(parent_row_style(tokens))
        .width(Length::Fill)
        .height(Length::Fixed(layout::SFTP_FILE_ROW_HEIGHT))
        .on_press(Message::SftpTab(
            crate::app::message::SftpTabMessage::SftpNavigateToParent,
        ))
        .into()
}

// ============================================================================
// Header row (冻结表头)
// ============================================================================

/// 构建可排序的表头行
fn header_row<'a>(state: &'a IcedState, sftp: &'a SftpPanel) -> Element<'a, Message> {
    let tokens = tokens_for_state(state);
    let i18n = &state.model.i18n;

    let sort_col = sftp.sort_by;
    let sort_dir = sftp.sort_direction;

    let name_btn = header_col_button(
        i18n.tr("iced.sftp.header.name"),
        sort_col == SftpSortBy::Name,
        sort_dir,
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpSortBy(SftpSortBy::Name)),
        tokens,
    );
    let perm_label = text(i18n.tr("iced.sftp.header.permissions"))
        .size(11)
        .color(tokens.text_secondary);
    let size_btn = header_col_button(
        i18n.tr("iced.sftp.header.size"),
        sort_col == SftpSortBy::Size,
        sort_dir,
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpSortBy(SftpSortBy::Size)),
        tokens,
    );
    let modified_btn = header_col_button(
        i18n.tr("iced.sftp.header.modified"),
        sort_col == SftpSortBy::Modified,
        sort_dir,
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpSortBy(SftpSortBy::Modified)),
        tokens,
    );

    // 左侧留出图标宽度的空间
    let icon_space = Space::new().width(Length::Fixed(layout::SFTP_ICON_SIZE_FOLDER + layout::SFTP_COL_NAME_ICON_SPACING));

    let header_content: Element<'a, Message> = row![
        icon_space,
        name_btn,
        Space::new().width(Length::Fill),
        container(perm_label).width(Length::Fixed(layout::SFTP_COL_PERMISSIONS)),
        Space::new().width(Length::Fixed(layout::SFTP_COL_SPACING)),
        container(size_btn).width(Length::Fixed(layout::SFTP_COL_SIZE)),
        Space::new().width(Length::Fixed(layout::SFTP_COL_SPACING)),
        container(modified_btn).width(Length::Fixed(layout::SFTP_COL_MODIFIED)),
    ]
    .spacing(0)
    .align_y(Vertical::Center)
    .into();

    container(header_content)
        .padding([0, layout::SFTP_ROW_PADDING_H as u16])
        .width(Length::Fill)
        .height(Length::Fixed(layout::SFTP_HEADER_ROW_HEIGHT))
        .align_y(Vertical::Center)
        .style(header_row_style(tokens))
        .into()
}

/// 构建可排序的表头列按钮
fn header_col_button<'a>(
    label: &'a str,
    is_active: bool,
    direction: SortDirection,
    msg: Message,
    tokens: DesignTokens,
) -> Element<'a, Message> {
    let color = if is_active {
        tokens.accent_base
    } else {
        tokens.text_secondary
    };

    let label_elem = text(label)
        .size(11)
        .color(color);

    if is_active {
        // 激活状态：文字 + 排序方向图标
        let arrow_icon = match direction {
            SortDirection::Ascending => icon_view_with(
                IconOptions::new(IconId::NavArrowDown)
                    .with_size(10)
                    .with_color(tokens.accent_base),
                msg.clone(),
            ),
            SortDirection::Descending => icon_view_with(
                IconOptions::new(IconId::NavArrowDown)
                    .with_size(10)
                    .with_color(tokens.accent_base),
                msg.clone(),
            ),
        };

        let content: Element<'a, Message> = row![label_elem, arrow_icon]
            .spacing(2)
            .align_y(Vertical::Center)
            .into();

        button(content)
            .padding([3, 6])
            .style(move |_: &iced::Theme, status: iced::widget::button::Status| {
                let mut style = iced::widget::button::Style::default();
                match status {
                    iced::widget::button::Status::Hovered => {
                        style.background = Some(Background::Color(
                            Color::from_rgba(tokens.accent_base.r, tokens.accent_base.g, tokens.accent_base.b, 0.12),
                        ));
                        style.border = Border {
                            radius: layout::SFTP_ROW_RADIUS.into(),
                            ..Default::default()
                        };
                    }
                    _ => {
                        style.background = Some(Background::Color(
                            Color::from_rgba(tokens.accent_base.r, tokens.accent_base.g, tokens.accent_base.b, 0.06),
                        ));
                        style.border = Border {
                            radius: layout::SFTP_ROW_RADIUS.into(),
                            ..Default::default()
                        };
                    }
                }
                style
            })
            .on_press(msg)
            .into()
    } else {
        // 非激活状态：纯文字
        button(label_elem)
            .padding([3, 6])
            .style(move |_: &iced::Theme, status: iced::widget::button::Status| {
                let mut style = iced::widget::button::Style::default();
                if let iced::widget::button::Status::Hovered = status {
                    style.background = Some(Background::Color(
                        Color::from_rgba(tokens.text_secondary.r, tokens.text_secondary.g, tokens.text_secondary.b, 0.08),
                    ));
                    style.border = Border {
                        radius: layout::SFTP_ROW_RADIUS.into(),
                        ..Default::default()
                    };
                }
                style
            })
            .on_press(msg)
            .into()
    }
}

// ============================================================================
// File row
// ============================================================================

/// 根据文件扩展名返回对应的图标 ID
fn file_type_icon(name: &str) -> IconId {
    let ext = name
        .rsplit('.')
        .next()
        .filter(|e| e.len() < name.len())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        // 代码文件
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "java" | "c" | "cpp" | "h" | "hpp"
        | "cs" | "rb" | "php" | "swift" | "kt" | "scala" | "lua" | "r" | "m" | "mm" => {
            IconId::FileCode
        }
        // 标记/配置文件
        "html" | "htm" | "css" | "scss" | "less" | "json" | "yaml" | "yml" | "toml" | "xml"
        | "svg" | "md" | "rst" | "tex" => IconId::FileCode,
        // Shell/脚本文件
        "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" => IconId::FileCode,
        // 文本文件
        "txt" | "log" | "csv" | "tsv" | "ini" | "cfg" | "conf" | "env" => {
            IconId::FileDocumentFill
        }
        // 图片文件
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" | "tiff" | "tif" | "heic"
        | "heif" => IconId::FileImage,
        // 压缩文件
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "tgz" | "tbz2" | "zst" => {
            IconId::FileArchive
        }
        // 媒体文件
        "mp3" | "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "wav" | "flac" | "ogg"
        | "aac" | "wma" | "m4a" | "m4v" | "webm" => IconId::FileMedia,
        // 其他
        _ => IconId::File,
    }
}

/// 构建单个文件行
fn build_file_row<'a>(
    entry: RemoteFileEntry,
    tokens: DesignTokens,
) -> Element<'a, Message> {
    let name = entry.name.clone();
    let path_str = entry.path.to_string_lossy().into_owned();
    let size_human = entry.size_human();
    let modified_human = entry.modified_human();
    let is_dir = entry.is_dir;
    let is_hidden = entry.is_hidden();
    let is_symlink = entry.is_symlink;
    let permissions = entry.permissions.clone();

    // 文件名颜色
    let name_color = if is_hidden {
        tokens.text_disabled
    } else if is_symlink {
        tokens.accent_hover
    } else {
        tokens.text_primary
    };

    // 大小文本
    let size_text = if is_dir { "-".to_string() } else { size_human };

    // 创建消息
    let msg: Message = if is_dir {
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpNavigate(
            path_str.clone(),
        ))
    } else {
        Message::SftpTab(crate::app::message::SftpTabMessage::SftpDownload(
            path_str.clone(),
        ))
    };

    // 图标
    let icon_size = if is_dir {
        layout::SFTP_ICON_SIZE_FOLDER
    } else {
        layout::SFTP_ICON_SIZE_FILE
    };
    let icon_color = if is_dir {
        tokens.accent_base
    } else if is_symlink {
        tokens.accent_hover
    } else {
        tokens.text_secondary
    };

    let icon_id = if is_dir {
        IconId::FileFolder
    } else {
        file_type_icon(&name)
    };

    let icon_elem = icon_view_with(
        IconOptions::new(icon_id)
            .with_size(icon_size as u32)
            .with_color(icon_color),
        msg.clone(),
    );

    // 文件名
    let name_elem: Element<'a, Message> = text(name)
        .size(12)
        .color(name_color)
        .into();

    // 权限文本
    let perm_text = if permissions.is_empty() {
        String::new()
    } else {
        permissions
    };
    let perm_elem: Element<'a, Message> =
        text(perm_text).size(11).color(tokens.text_disabled).into();

    // 大小
    let size_elem: Element<'a, Message> =
        text(size_text).size(11).color(tokens.text_secondary).into();

    // 修改时间
    let modified_elem: Element<'a, Message> = text(modified_human)
        .size(11)
        .color(tokens.text_secondary)
        .into();

    // 行布局
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
        .padding([layout::SFTP_ROW_PADDING_V, layout::SFTP_ROW_PADDING_H])
        .width(Length::Fill)
        .height(Length::Fixed(layout::SFTP_FILE_ROW_HEIGHT))
        .on_press(msg)
        .style(file_row_button_style(tokens, false))
        .into()
}

// ============================================================================
// Sorting
// ============================================================================

/// 根据排序规则排序文件列表
fn sort_entries(
    entries: Vec<RemoteFileEntry>,
    sort_by: SftpSortBy,
    direction: SortDirection,
) -> Vec<RemoteFileEntry> {
    let desc = direction.is_descending();

    let mut entries = entries;
    entries.sort_by(|a, b| {
        // 目录优先排序
        if a.is_dir != b.is_dir {
            return if a.is_dir {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }

        let cmp = match sort_by {
            SftpSortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SftpSortBy::Size => a.size.cmp(&b.size),
            SftpSortBy::Modified => a.modified.cmp(&b.modified),
        };

        if desc { cmp.reverse() } else { cmp }
    });

    entries
}

// ============================================================================
// Styles
// ============================================================================

/// 空状态容器
fn make_empty_state<'a>(msg: &'a str, color: Color) -> Element<'a, Message> {
    let col: Column<'a, Message> = Column::new()
        .push(Space::new().height(Length::Fill))
        .push(
            row![
                Space::new().width(Length::Fill),
                text(msg).size(12).color(color),
                Space::new().width(Length::Fill),
            ]
            .align_y(Vertical::Center),
        )
        .push(Space::new().height(Length::Fill))
        .align_x(iced::alignment::Horizontal::Center);

    container(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// 表头行样式
fn header_row_style(
    tokens: DesignTokens,
) -> impl Fn(&iced::Theme) -> iced::widget::container::Style + 'static {
    let bg = tokens.surface_2;
    move |_: &iced::Theme| iced::widget::container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            width: 0.0,
            color: Color::TRANSPARENT,
            radius: 0.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// 文件行按钮样式（Liquid Glass 风格）
fn file_row_button_style(
    tokens: DesignTokens,
    is_selected: bool,
) -> impl Fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style + 'static {
    let accent = tokens.accent_base;
    let glass_subtle = tokens.glass_subtle;
    let glass_moderate = tokens.glass_moderate;
    let border_subtle = tokens.border_subtle;

    move |_: &iced::Theme, status: iced::widget::button::Status| {
        let mut style = iced::widget::button::Style::default();

        if is_selected {
            style.background = Some(Background::Color(Color::from_rgba(
                accent.r, accent.g, accent.b, 0.10,
            )));
            style.border = Border {
                width: 1.0,
                color: Color::from_rgba(accent.r, accent.g, accent.b, 0.25),
                radius: layout::SFTP_ROW_RADIUS.into(),
            };
        } else {
            match status {
                iced::widget::button::Status::Hovered => {
                    style.background = Some(Background::Color(glass_subtle));
                    style.border = Border {
                        width: 1.0,
                        color: border_subtle,
                        radius: layout::SFTP_ROW_RADIUS.into(),
                    };
                }
                iced::widget::button::Status::Pressed => {
                    style.background = Some(Background::Color(glass_moderate));
                    style.border = Border {
                        width: 1.0,
                        color: border_subtle,
                        radius: layout::SFTP_ROW_RADIUS.into(),
                    };
                }
                _ => {
                    style.border = Border {
                        width: 1.0,
                        color: Color::TRANSPARENT,
                        radius: layout::SFTP_ROW_RADIUS.into(),
                    };
                }
            }
        }

        style
    }
}

/// 返回上级目录行样式
fn parent_row_style(
    tokens: DesignTokens,
) -> impl Fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style + 'static {
    let glass_subtle = tokens.glass_subtle;
    let glass_moderate = tokens.glass_moderate;
    let border_subtle = tokens.border_subtle;

    move |_: &iced::Theme, status: iced::widget::button::Status| {
        let mut style = iced::widget::button::Style::default();
        match status {
            iced::widget::button::Status::Hovered => {
                style.background = Some(Background::Color(glass_subtle));
                style.border = Border {
                    width: 1.0,
                    color: border_subtle,
                    radius: layout::SFTP_ROW_RADIUS.into(),
                };
            }
            iced::widget::button::Status::Pressed => {
                style.background = Some(Background::Color(glass_moderate));
                style.border = Border {
                    width: 1.0,
                    color: border_subtle,
                    radius: layout::SFTP_ROW_RADIUS.into(),
                };
            }
            _ => {
                style.border = Border {
                    width: 1.0,
                    color: Color::TRANSPARENT,
                    radius: layout::SFTP_ROW_RADIUS.into(),
                };
            }
        }
        style
    }
}
