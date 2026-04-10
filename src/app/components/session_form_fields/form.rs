//! 会话表单组件入口
//! 统一的新建/编辑会话表单组件，支持快速连接和设置中心两种入口模式

use iced::alignment::{Alignment, Horizontal};
use iced::widget::{
    button, column, container, pick_list, row, text, text_input, Space,
};
use iced::{Element, Length};
use secrecy::ExposeSecret;

use crate::app::components::helpers::{layered_scrim_style, top_bar_material_style};
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::widgets::chrome_button::{style_chrome_primary, style_chrome_secondary, style_top_icon};
use crate::i18n::I18n;
use crate::session::AuthMethod;
use crate::theme::DesignTokens;
use crate::theme::icons::{icon_view_with, IconId, IconOptions};

use super::sidebar::{sidebar, SidebarPage};

/// 会话表单入口模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionFormMode {
    /// 快速连接入口（简化表单）
    QuickConnect,
    /// 设置中心入口（完整表单+侧边栏）
    SettingsCenter,
}

/// 辅助函数：获取当前状态对应的 DesignTokens
pub fn session_form_tokens(state: &IcedState) -> DesignTokens {
    DesignTokens::for_color_scheme(&state.model.settings.color_scheme)
}

/// 会话编辑器模态框（统一版本）
pub fn session_form_dialog(state: &IcedState) -> Element<'_, Message> {
    let Some(ed) = state.session_editor.as_ref() else {
        return Space::new().into();
    };

    let tokens = session_form_tokens(state);
    let i18n = &state.model.i18n;

    // 判断入口模式：快速连接用简化模式，设置中心用完整模式
    let mode = if state.quick_connect_open {
        SessionFormMode::QuickConnect
    } else {
        SessionFormMode::SettingsCenter
    };

    let scrim = container(Space::new().width(iced::Length::Fill).height(iced::Length::Fill))
        .style(layered_scrim_style(tokens.clone(), 0));

    // 根据模式构建表单
    let form_body = match mode {
        SessionFormMode::QuickConnect => quick_connect_form_body(tokens.clone(), i18n, ed),
        SessionFormMode::SettingsCenter => settings_center_form_body(tokens.clone(), i18n, ed),
    };

    // 对话框容器
    let dialog_width = match mode {
        SessionFormMode::QuickConnect => Length::Fixed(520.0),
        SessionFormMode::SettingsCenter => Length::Fixed(800.0),
    };

    let card = container(form_body)
        .width(dialog_width)
        .padding(16)
        .style(top_bar_material_style(tokens.clone()));

    let centered = container(card)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Alignment::Center);

    iced::widget::Stack::with_children([scrim.into(), centered.into()])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}

/// 快速连接表单（简化版本）
fn quick_connect_form_body<'a>(
    tokens: DesignTokens,
    i18n: &'a I18n,
    ed: &'a crate::app::state::SessionEditorState,
) -> Element<'a, Message> {
    let title = if ed.profile_id.is_some() {
        i18n.tr("session_form.title_edit")
    } else {
        i18n.tr("session_form.title_new")
    };

    // 认证方式选项
    let auth_options: Vec<AuthMethod> = vec![
        AuthMethod::Password,
        AuthMethod::Agent,
        AuthMethod::Interactive,
        AuthMethod::Key { private_key_path: String::new() },
    ];

    let mut body = column![
        // 标题栏
        row![
            text(title).size(16),
            Space::new().width(Length::Fill),
            icon_close_button(tokens.clone()),
        ]
        .align_y(Alignment::Center),

        // 主机 + 端口
        row![
            text_input(i18n.tr("session_form.field.host"), &ed.host)
                .on_input(Message::SessionEditorHostChanged)
                .width(Length::FillPortion(3)),
            text_input(i18n.tr("session_form.field.port"), &ed.port)
                .on_input(Message::SessionEditorPortChanged)
                .width(Length::FillPortion(1)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),

        // 用户名
        text_input(i18n.tr("session_form.field.user"), &ed.user)
            .on_input(Message::SessionEditorUserChanged)
            .width(Length::Fill),

        // 认证方式
        pick_list(
            auth_options,
            Some(ed.auth.clone()),
            Message::SessionEditorAuthChanged,
        )
        .width(Length::Fill),
    ]
    .spacing(10)
    .width(Length::Fill);

    // 根据认证方式显示额外字段
    match &ed.auth {
        AuthMethod::Password => {
            body = body.push(
                text_input(i18n.tr("session_form.field.password"), ed.password.expose_secret())
                    .secure(true)
                    .on_input(Message::SessionEditorPasswordChanged)
                    .width(Length::Fill),
            );
        }
        AuthMethod::Key { .. } => {
            body = body.push(
                text_input(i18n.tr("session_form.field.private_key"), &ed.private_key_path)
                    .on_input(Message::SessionEditorPrivateKeyPathChanged)
                    .width(Length::Fill),
            );
            body = body.push(
                text_input(i18n.tr("session_form.field.passphrase"), ed.passphrase.expose_secret())
                    .secure(true)
                    .on_input(Message::SessionEditorPassphraseChanged)
                    .width(Length::Fill),
            );
        }
        _ => {}
    }

    // 错误提示
    if let Some(err) = &ed.error {
        body = body.push(
            container(text(err).size(12))
                .padding(10)
                .style(top_bar_material_style(tokens.clone())),
        );
    }

    // 按钮行
    let actions = row![
        button(text(i18n.tr("session_form.btn.save")).size(13))
            .on_press(Message::SessionEditorSave)
            .style(style_chrome_primary(tokens.clone())),
        button(text(i18n.tr("session_form.btn.cancel")).size(13))
            .on_press(Message::SessionEditorClose)
            .style(style_chrome_secondary(tokens.clone())),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    body.push(actions).spacing(12).into()
}

/// 设置中心表单（完整版本，含侧边栏）
fn settings_center_form_body<'a>(
    tokens: DesignTokens,
    i18n: &'a I18n,
    ed: &'a crate::app::state::SessionEditorState,
) -> Element<'a, Message> {
    let title = if ed.profile_id.is_some() {
        i18n.tr("session_form.title_edit")
    } else {
        i18n.tr("session_form.title_new")
    };

    // 认证方式选项
    let auth_options: Vec<AuthMethod> = vec![
        AuthMethod::Password,
        AuthMethod::Agent,
        AuthMethod::Interactive,
        AuthMethod::Key { private_key_path: String::new() },
    ];

    // 左侧边栏（使用 clone 避免生命周期问题）
    let sidebar_elem = sidebar(&SidebarPage::General, tokens.clone(), i18n);

    // 右侧内容区
    let mut content = column![
        // 标题栏
        row![
            text(title).size(16),
            Space::new().width(Length::Fill),
            icon_close_button(tokens.clone()),
        ]
        .align_y(Alignment::Center),

        // 名称输入框
        text_input(i18n.tr("session_form.field.name"), &ed.name)
            .on_input(Message::SessionEditorNameChanged)
            .width(Length::Fill),

        // 主机 + 端口
        row![
            text_input(i18n.tr("session_form.field.host"), &ed.host)
                .on_input(Message::SessionEditorHostChanged)
                .width(Length::FillPortion(3)),
            text_input(i18n.tr("session_form.field.port"), &ed.port)
                .on_input(Message::SessionEditorPortChanged)
                .width(Length::FillPortion(1)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),

        // 用户名
        text_input(i18n.tr("session_form.field.user"), &ed.user)
            .on_input(Message::SessionEditorUserChanged)
            .width(Length::Fill),

        // 认证方式
        pick_list(
            auth_options,
            Some(ed.auth.clone()),
            Message::SessionEditorAuthChanged,
        )
        .width(Length::Fill),
    ]
    .spacing(12)
    .width(Length::Fill);

    // 根据认证方式显示额外字段
    match &ed.auth {
        AuthMethod::Password => {
            content = content.push(
                text_input(i18n.tr("session_form.field.password"), ed.password.expose_secret())
                    .secure(true)
                    .on_input(Message::SessionEditorPasswordChanged)
                    .width(Length::Fill),
            );
        }
        AuthMethod::Key { .. } => {
            content = content.push(
                text_input(i18n.tr("session_form.field.private_key"), &ed.private_key_path)
                    .on_input(Message::SessionEditorPrivateKeyPathChanged)
                    .width(Length::Fill),
            );
            content = content.push(
                text_input(i18n.tr("session_form.field.passphrase"), ed.passphrase.expose_secret())
                    .secure(true)
                    .on_input(Message::SessionEditorPassphraseChanged)
                    .width(Length::Fill),
            );
        }
        _ => {}
    }

    // 错误提示
    if let Some(err) = &ed.error {
        content = content.push(
            container(text(err).size(12))
                .padding(10)
                .style(top_bar_material_style(tokens.clone())),
        );
    }

    // 底部按钮
    let actions = row![
        button(text(i18n.tr("session_form.btn.test_connection")).size(13))
            .on_press(Message::SessionEditorTestConnection)
            .style(style_chrome_secondary(tokens.clone())),
        Space::new().width(Length::Fill),
        button(text(i18n.tr("session_form.btn.cancel")).size(13))
            .on_press(Message::SessionEditorClose)
            .style(style_chrome_secondary(tokens.clone())),
        button(text(i18n.tr("session_form.btn.save")).size(13))
            .on_press(Message::SessionEditorSave)
            .style(style_chrome_primary(tokens.clone())),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    content = content.push(actions).spacing(16);

    // 左右分栏布局
    row![sidebar_elem, container(content).padding(24).width(Length::Fill)]
        .spacing(0)
        .into()
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 创建关闭图标按钮
fn icon_close_button(tokens: DesignTokens) -> Element<'static, Message> {
    let close_icon = icon_view_with(
        IconOptions::new(IconId::Close)
            .with_size(14)
            .with_color(tokens.text_secondary),
        Message::SessionEditorClose,
    );
    button(close_icon)
        .on_press(Message::SessionEditorClose)
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .style(style_top_icon(tokens))
        .into()
}
