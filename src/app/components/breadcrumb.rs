//! Breadcrumb 导航栏组件。

use iced::alignment::Alignment;
use iced::widget::{Space, button, container, mouse_area, row, text};
use iced::{Border, Element, Length};

use crate::app::components::helpers::tokens_for_state;
use crate::app::message::Message;
use crate::app::state::{BreadcrumbAnimPhase, IcedState};
use crate::app::terminal_viewport;
use crate::theme::icons::{icon_button_auto, icon_view_with, IconButtonSize, IconId, IconOptions};
use crate::theme::layout::{
    BREADCRUMB_BTN_GAP, BREADCRUMB_FLOAT_ICON_MARGIN, BREADCRUMB_FLOAT_ICON_SIZE,
    BREADCRUMB_SECTIONS_GAP,
};
use crate::theme::liquid_glass::glass_icon_button_style;

/// 构建会话标识文本。
/// 优先使用会话名称，如果没有则使用 username@host[:port]。
fn build_session_label(state: &IcedState) -> String {
    // 1. 优先使用会话名称
    if let Some(tab) = state.tabs.get(state.active_tab) {
        if !tab.title.is_empty() && tab.title != state.model.i18n.tr("iced.tab.new") {
            return tab.title.clone();
        }
    }

    // 2. 回退到 user@host[:port]
    let draft = &state.model.draft;
    let mut label = format!("{}@{}", draft.user, draft.host);
    if draft.port != "22" {
        label.push_str(&format!(":{}", draft.port));
    }
    label
}

/// 构建 breadcrumb 导航栏。
///
/// 根据动画状态动态调整高度和背景透明度：
/// - Collapsed: height=0, alpha=0
/// - Expanded: height=fixed, alpha=1
/// - Expanding: 高度和透明度从 0 过渡到 full
/// - Collapsing: 高度和透明度从 full 过渡到 0
pub(crate) fn breadcrumb(state: &IcedState) -> Element<'_, Message> {
    let tokens = tokens_for_state(state);
    let term_vp =
        terminal_viewport::terminal_viewport_spec_for_settings(&state.model.settings.terminal);
    let full_h = term_vp.breadcrumb_block_h();

    // 动画优化：展开/收起期间高度固定，透明度过渡。
    // 高度跳变仅在动画结束时（Expanded/Collapsed）发生，
    // 避免每帧重建布局树导致卡顿。
    let tick_ms = state.tick_ms();
    let (h, bg_alpha) = match state.breadcrumb_anim.phase {
        BreadcrumbAnimPhase::Collapsed => (0.0, 0.0),
        BreadcrumbAnimPhase::Expanded => (full_h, 1.0),
        BreadcrumbAnimPhase::Expanding => {
            // 展开中：高度固定为 full，透明度从 0 过渡到 1
            let p = state.breadcrumb_anim_progress(tick_ms);
            (full_h, p)
        }
        BreadcrumbAnimPhase::Collapsing => {
            // 收起中：高度固定为 full，透明度从 1 过渡到 0
            let p = state.breadcrumb_anim_progress(tick_ms);
            (full_h, 1.0 - p)
        }
    };

    // 连接状态圆点
    let is_connected = state.active_session_is_connected();
    let status_dot_color = if is_connected {
        tokens.success
    } else {
        tokens.error
    };
    let status_dot = container(
        Space::new()
            .width(Length::Fixed(8.0))
            .height(Length::Fixed(8.0)),
    )
    .width(Length::Fixed(8.0))
    .height(Length::Fixed(8.0))
    .style(move |_: &_| iced::widget::container::Style {
        background: Some(iced::Background::Color(status_dot_color)),
        border: Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    });

    // 会话标识
    let session_label = build_session_label(state);
    let session_text = text(session_label).size(13);

    // 当前目录
    let cwd_text = text(state.remote_cwd.clone()).size(13);

    // 左侧区域：连接状态圆点 + 分隔 + 会话标识 + 目录
    let left_area = row![
        status_dot,
        Space::new().width(Length::Fixed(6.0)),
        session_text,
        text(" · ").size(13),
        cwd_text
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    // 右侧按钮
    let right_buttons = build_action_buttons(state);

    // 完整布局：左侧区域 + 弹性空间 + 右侧按钮（自动右对齐）
    // 动态高度由 breadcrumb_anim.phase 决定，透明度同步过渡
    let bg_color = iced::Color {
        a: bg_alpha,
        ..tokens.terminal_bg
    };
    container(
        row![left_area, Space::new().width(Length::Fill), right_buttons]
            .spacing(BREADCRUMB_SECTIONS_GAP)
            .align_y(Alignment::Center),
    )
    .padding(term_vp.breadcrumb_padding())
    .height(iced::Length::Fixed(h))
    .align_y(Alignment::Center)
    .style(move |_: &_| iced::widget::container::Style {
        background: Some(iced::Background::Color(bg_color)),
        ..Default::default()
    })
    .into()
}

/// 构建右侧操作按钮行（统一 24×24 紧凑尺寸）。
fn build_action_buttons(state: &IcedState) -> Element<'static, Message> {
    let color_scheme = &state.model.settings.color_scheme;
    let reconnect_btn =
        icon_button_auto(IconId::FnReload, IconButtonSize::COMPACT, Message::ConnectPressed, color_scheme);
    let sftp_btn =
        icon_button_auto(IconId::ModSftp, IconButtonSize::COMPACT, Message::BreadcrumbSftp, color_scheme);
    let port_btn =
        icon_button_auto(IconId::ModPortForward, IconButtonSize::COMPACT, Message::BreadcrumbPortForward, color_scheme);
    let pin_icon = if state.breadcrumb_pinned {
        IconId::ModPin
    } else {
        IconId::ModUnpin
    };
    let pin_btn =
        icon_button_auto(pin_icon, IconButtonSize::COMPACT, Message::BreadcrumbTogglePin, color_scheme);

    row![reconnect_btn, sftp_btn, port_btn, pin_btn]
        .spacing(BREADCRUMB_BTN_GAP)
        .align_y(Alignment::Center)
        .into()
}

/// 构建 breadcrumb 浮动图标。
///
/// 固定在终端区域右上角，始终渲染，通过 alpha 实现透明度动画。
pub(crate) fn breadcrumb_float_icon(state: &IcedState) -> Element<'static, Message> {
    // 浮动图标透明度 = 1 - breadcrumb_progress
    // 即 breadcrumb 展开时图标淡出，收起时图标淡入
    let alpha = 1.0 - state.breadcrumb_anim_progress(state.tick_ms());
    breadcrumb_float_icon_with_alpha(state, alpha)
}

/// 构建带指定透明度的浮动图标。
pub(crate) fn breadcrumb_float_icon_with_alpha(
    state: &IcedState,
    alpha: f32,
) -> Element<'static, Message> {
    let tokens = tokens_for_state(state);

    // 使用 Unpin 图标（展开按钮），点击后显示 breadcrumb
    // 使用 .small() (12px) 符合浮动图标的规范尺寸
    let icon = icon_view_with(
        IconOptions::new(IconId::ModUnpin)
            .small()
            .with_color(iced::Color {
                a: alpha,
                ..tokens.text_secondary
            }),
        Message::BreadcrumbShowTemp,
    );
    let icon_btn: Element<'_, Message> = button(icon)
        .width(Length::Fixed(BREADCRUMB_FLOAT_ICON_SIZE))
        .height(Length::Fixed(BREADCRUMB_FLOAT_ICON_SIZE))
        // .padding(0)
        .style(glass_icon_button_style(tokens.clone()))
        .into();

    // 鼠标进入时展开，移出时收起
    let icon_btn = mouse_area(icon_btn)
        .on_press(Message::BreadcrumbShowTemp)
        .on_exit(Message::BreadcrumbHideTemp);

    // 使用 Space 将图标推到右上角
    container(
        row![Space::new().width(Length::Fill), icon_btn,]
            .spacing(0)
            .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(
        BREADCRUMB_FLOAT_ICON_SIZE + BREADCRUMB_FLOAT_ICON_MARGIN * 2.0,
    ))
    .padding(BREADCRUMB_FLOAT_ICON_MARGIN)
    .into()
}
