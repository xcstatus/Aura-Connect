use iced::alignment::Alignment;
use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text, tooltip, Stack};
use iced::{Background, Element, Length, Pixels, Theme};

use crate::app::chrome::{
    TOP_BAR_H, TOP_CONTROL_GROUP_W, TOP_ICON_BTN, TRAFFIC_LIGHT_BAND_W, TRAFFIC_LIGHT_DIAMETER,
    tab_strip_width,
};
use crate::app::components::helpers::tokens_for_state;
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::widgets::chrome_button::{lg_icon_button, lg_icon_button_flat, lg_tab_button};
use crate::theme::DesignTokens;
use crate::theme::icons::{IconId, IconOptions, icon_view_with};
use crate::theme::layout::{
    TAB_CHIP_MIN_WIDTH, TAB_CHIP_PAD_H, TAB_CHIP_WIDTH, TAB_CLOSE_HIT_W, TAB_CLOSE_ICON_W,
    TAB_LABEL_CLOSE_SPACING,
};

/// Build the top bar (tab strip + action buttons + control buttons).
pub(crate) fn top_bar(state: &IcedState, _tick_ms: f32) -> Element<'_, Message> {
    let tokens = tokens_for_state(state);
    let tabs_row = build_tab_strip(state);
    let action_group = build_action_group(state, tokens);
    let control_group = build_control_group(state, tokens);

    // 左侧区域：traffic_light + 标签栏 + 操作按钮组
    let left_area: Element<'_, Message> = {
        let mut left_row = row![].spacing(0);

        #[cfg(target_os = "macos")]
        {
            left_row = left_row.push(
                container(Space::new().height(iced::Length::Fixed(TRAFFIC_LIGHT_DIAMETER)))
                    .width(iced::Length::Fixed(TRAFFIC_LIGHT_BAND_W))
                    .style(top_bar_ambient_style(tokens)),
            );
        }

        left_row = left_row.push(tabs_row);

        container(left_row)
            .width(iced::Length::Fill)
            .height(iced::Length::Fixed(TOP_BAR_H))
            .into()
    };

    // 顶栏：左侧区域 + 控制组，无 spacing，直接贴靠
    let top_bar_row = row![left_area, action_group, control_group]
        .spacing(0)
        .align_y(Alignment::Center);

    container(top_bar_row)
        .height(iced::Length::Fixed(TOP_BAR_H))
        .padding(0)
        .style(top_bar_ambient_style(tokens))
        .into()
}

/// 欢迎页专用的简化顶栏：只有 traffic lights + 操作按钮，无标签栏。
pub(crate) fn title_bar(state: &IcedState, _tick_ms: f32) -> Element<'_, Message> {
    let tokens = tokens_for_state(state);
    let action_group = build_action_group(state, tokens);
    let control_group = build_control_group(state, tokens);

    let left_area: Element<'_, Message> = {
        let mut left_row = row![].spacing(0);

        #[cfg(target_os = "macos")]
        {
            left_row = left_row.push(
                container(Space::new().height(iced::Length::Fixed(TRAFFIC_LIGHT_DIAMETER)))
                    .width(iced::Length::Fixed(TRAFFIC_LIGHT_BAND_W))
                    .style(top_bar_ambient_style(tokens)),
            );
        }

        container(left_row)
            .width(iced::Length::Fill)
            .height(iced::Length::Fixed(TOP_BAR_H))
            .into()
    };

    let top_bar_row = row![left_area, action_group, control_group]
        .spacing(0)
        .align_y(Alignment::Center);

    container(top_bar_row)
        .height(iced::Length::Fixed(TOP_BAR_H))
        .padding(0)
        .style(top_bar_ambient_style(tokens))
        .into()
}

/// 顶栏背景样式：使用 bar_default 统一顶栏颜色
fn top_bar_ambient_style(
    tokens: DesignTokens,
) -> impl Fn(&Theme) -> iced::widget::container::Style + 'static {
    move |_: &Theme| container::Style {
        background: Some(Background::Color(tokens.bar_default)),
        border: iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn build_tab_strip(state: &IcedState) -> Element<'_, Message> {
    let tokens = tokens_for_state(state);
    let available_w = tab_strip_width(state.window_size.width);
    let tabs_count = state.tabs.len();

    // 计算每个 chip 的目标宽度
    let avg_chip_w = if tabs_count > 0 {
        (available_w / tabs_count as f32).max(TAB_CHIP_MIN_WIDTH)
    } else {
        TAB_CHIP_WIDTH
    };
    let target_chip_w = TAB_CHIP_WIDTH.min(avg_chip_w);

    // 构建标签行
    let mut tabs_row = row![].spacing(0).align_y(Alignment::Center);

    for (i, tab) in state.tabs.iter().enumerate() {
        let tab_label = tab.title.clone();
        let is_active = i == state.active_tab;

        // 关闭按钮：悬停时显示，固定 24px 可交互区域
        let is_hovered = Some(i) == state.tab_hover_index;
        let show_close = is_hovered;

        let select_btn = button(text(tab_label).size(11).align_y(Alignment::Center))
            .on_press(Message::TabScrollTo(i))
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .style(lg_tab_button(tokens));

        let close_btn_elem: Element<'static, Message> = if show_close && !state.show_welcome {
            icon_tab_close_button(tokens, i, TAB_CLOSE_HIT_W)
        } else {
            Space::new().width(Pixels(0.0)).height(Pixels(0.0)).into()
        };

        let top_line = container(
            Space::new()
                .width(iced::Length::Fill)
                .height(iced::Length::Fixed(if is_active { 2.0 } else { 0.0 })),
        )
        .style(move |_theme: &Theme| {
            if is_active {
                container::Style::default().background(tokens.accent_base)
            } else {
                container::Style::default()
            }
        });

        // 每个 chip 的背景色
        // 选中态 (bar_active): 与终端区域同色，视觉打通
        // 未选中态 (bar_default): 顶栏默认色
        let chip_bg = if is_active { tokens.bar_active } else { tokens.bar_default };
        let chip_bg_style = move |_: &Theme| container::Style {
            background: Some(iced::Background::Color(chip_bg)),
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: if is_active {
                    iced::border::Radius::new(6.0)
                        .bottom_left(0.0)
                        .bottom_right(0.0)
                } else {
                    6.0.into()
                },
            },
            ..Default::default()
        };

        // Stack 实现绝对定位：select_btn 在下，close_btn 浮在右上角
        let content_with_close = Stack::new()
            .push(select_btn)
            .push(
                container(close_btn_elem)
                    .align_x(iced::alignment::Horizontal::Right)
                    .align_y(iced::alignment::Vertical::Center)
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .padding(iced::Padding {
                        top: 0.0,
                        right: TAB_CHIP_PAD_H,
                        bottom: 0.0,
                        left: 0.0,
                    }),
            );

        // chip 内层容器：无背景，仅负责内容布局与垂直居中
        // column 顺序：top_line → content_with_close 
        let inner = container(
            column![top_line, content_with_close].spacing(0),
        )
        .padding(0)
        .width(iced::Length::Fixed(target_chip_w))
        .height(iced::Length::Fixed(TOP_BAR_H))
        .style(chip_bg_style);

        let chip = mouse_area(inner)
            .on_enter(Message::TabChipHover(Some(i)))
            .on_exit(Message::TabChipHover(None));

        tabs_row = tabs_row.push(chip);
    }

    // 计算溢出数量
    let min_chips = (available_w / TAB_CHIP_MIN_WIDTH).floor() as usize;
    let overflow_count = tabs_count.saturating_sub(min_chips);

    // 标签滚动区
    let scrollable_tabs = scrollable(tabs_row)
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::hidden(),
        ))
        .width(iced::Length::Fill)
        .height(iced::Length::Fixed(TOP_BAR_H))
        .on_scroll(|viewport| Message::TabScrollTick(viewport.absolute_offset().x));

    // 动态构建行：无徽章时不添加徽章元素
    let mut final_row = row![scrollable_tabs].spacing(0).align_y(Alignment::Center);
    if overflow_count > 0 {
        let badge = build_overflow_badge(overflow_count, tokens);
        final_row = final_row.push(badge);
    }

    container(final_row)
        .width(iced::Length::Fill)
        .height(iced::Length::Fixed(TOP_BAR_H))
        .style(top_bar_ambient_style(tokens))
        .into()
}

fn build_overflow_badge(overflow_count: usize, tokens: DesignTokens) -> Element<'static, Message> {
    let badge = button(
        text(format!("+{}", overflow_count))
            .size(11)
            .color(tokens.text_secondary)
            .align_y(Alignment::Center),
    )
    .on_press(Message::TabOverflowToggle)
    .width(iced::Length::Fixed(TOP_ICON_BTN))
    .height(iced::Length::Fixed(TOP_ICON_BTN))
    .style(lg_icon_button(tokens));

    container(badge)
        .width(iced::Length::Shrink)
        .height(iced::Length::Fixed(TOP_BAR_H))
        .align_y(Alignment::Center)
        .into()
}

fn build_action_group(
    state: &IcedState,
    tokens: crate::theme::DesignTokens,
) -> Element<'static, Message> {
    let i18n = &state.model.i18n;

    // 快速连接图标 (QuickConnect)
    let quick_icon = icon_view_with(
        IconOptions::new(IconId::FnQuickConnect).with_size(15),
        Message::TopQuickConnect,
    );
    let btn_quick = button(quick_icon)
        .on_press(Message::TopQuickConnect)
        .width(iced::Length::Fixed(TOP_ICON_BTN))
        .height(iced::Length::Fixed(TOP_ICON_BTN))
        .padding(10)
        .style(lg_icon_button(tokens));

    // 新建标签页图标 (Plus)
    let plus_icon = icon_view_with(
        IconOptions::new(IconId::FnPlus).with_size(15),
        Message::TopAddTab,
    );
    let btn_new = button(plus_icon)
        .on_press(Message::TopAddTab)
        .width(iced::Length::Fixed(TOP_ICON_BTN))
        .height(iced::Length::Fixed(TOP_ICON_BTN))
        .style(lg_icon_button(tokens));

    let quick_tip = text(i18n.tr("iced.topbar.quick_connect")).size(12);
    let new_tip = text(i18n.tr("iced.topbar.new_tab")).size(12);

    // SFTP 传输列表按钮（仅在有活跃传输时显示）
    let transfer_count = state.tab_panes.get(state.active_tab)
        .and_then(|p| p.sftp_panel.as_ref())
        .map(|s| s.transfers.iter().filter(|t| !t.status.is_terminal()).count())
        .unwrap_or(0);

    let btn_transfer: Element<'static, Message> = if transfer_count > 0 {
        let transfer_icon = icon_view_with(
            IconOptions::new(IconId::ActionDownload2Line).with_size(15),
            Message::SftpTab(crate::app::message::SftpTabMessage::SftpToggleTransferList),
        );
        let transfer_btn = button(transfer_icon)
            .on_press(Message::SftpTab(crate::app::message::SftpTabMessage::SftpToggleTransferList))
            .width(iced::Length::Fixed(TOP_ICON_BTN))
            .height(iced::Length::Fixed(TOP_ICON_BTN))
            .style(lg_icon_button(tokens));
        let badge = text(format!("{}", transfer_count))
            .size(9)
            .color(tokens.on_accent_label);
        let badge_container = container(badge)
            .padding([1, 4])
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(tokens.accent_base)),
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });
        iced::widget::Stack::new()
            .push(transfer_btn)
            .push(
                container(badge_container)
                    .align_x(iced::alignment::Horizontal::Right)
                    .align_y(iced::alignment::Vertical::Top)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(iced::Padding {
                        top: 2.0,
                        right: 4.0,
                        bottom: 0.0,
                        left: 0.0,
                    }),
            )
            .into()
    } else {
        Space::new().width(Pixels(0.0)).height(Pixels(0.0)).into()
    };

    let transfer_tip = text(i18n.tr("iced.sftp.transfer.title")).size(12);

    container(
        row![
            tooltip(
                btn_quick,
                quick_tip,
                iced::widget::tooltip::Position::Bottom
            )
            .delay(std::time::Duration::from_millis(500)),
            tooltip(btn_new, new_tip, iced::widget::tooltip::Position::Bottom)
                .delay(std::time::Duration::from_millis(500)),
            if transfer_count > 0 {
                tooltip(btn_transfer, transfer_tip, iced::widget::tooltip::Position::Bottom)
                    .delay(std::time::Duration::from_millis(500))
                    .into()
            } else {
                btn_transfer
            },
        ]
        .align_y(Alignment::Center),
    )
    .height(iced::Length::Fixed(TOP_BAR_H))
    .padding([0, 0])
    .style(top_bar_ambient_style(tokens))
    .into()
}

fn build_control_group(
    state: &IcedState,
    tokens: crate::theme::DesignTokens,
) -> Element<'static, Message> {
    let i18n = &state.model.i18n;

    // 设置图标 (Gear)
    let gear_icon = icon_view_with(
        IconOptions::new(IconId::FnGear).with_size(15),
        Message::TopOpenSettings,
    );
    let btn_settings = button(gear_icon)
        .on_press(Message::TopOpenSettings)
        .width(iced::Length::Fixed(TOP_ICON_BTN))
        .height(iced::Length::Fixed(TOP_ICON_BTN))
        .style(lg_icon_button(tokens));
    let settings_tip = text(i18n.tr("iced.topbar.settings_center")).size(12);
    let settings_ctrl = tooltip(
        btn_settings,
        settings_tip,
        iced::widget::tooltip::Position::Bottom,
    )
    .delay(std::time::Duration::from_millis(500));

    // 窗口控制按钮
    let win_controls: Element<'static, Message> = {
        #[cfg(not(target_os = "macos"))]
        {
            // 最小化图标 (-)
            let minus_icon = icon_view_with(
                IconOptions::new(IconId::FnClose).with_size(12),
                Message::WinMinimize,
            );
            let btn_min = button(minus_icon)
                .on_press(Message::WinMinimize)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(26.0))
                .style(lg_icon_button(tokens));

            // 最大化/还原图标 (reload 用作占位)
            let max_icon = icon_view_with(
                IconOptions::new(IconId::FnReload).with_size(12),
                Message::WinToggleMaximize,
            );
            let btn_max = button(max_icon)
                .on_press(Message::WinToggleMaximize)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(26.0))
                .style(lg_icon_button(tokens));

            // 关闭图标 (×)
            let close_icon = icon_view_with(
                IconOptions::new(IconId::FnClose).with_size(12),
                Message::WinClose,
            );
            let btn_close = button(close_icon)
                .on_press(Message::WinClose)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(26.0))
                .style(lg_icon_button(tokens));

            row![btn_min, btn_max, btn_close,]
                .spacing(2)
                .align_y(Alignment::Center)
                .into()
        }
        #[cfg(target_os = "macos")]
        {
            Space::new().into()
        }
    };

    let control_row = row![settings_ctrl, win_controls].align_y(Alignment::Center);

    #[cfg(not(target_os = "macos"))]
    {
        control_row.spacing(4)
    }

    container(control_row)
        .width(iced::Length::Fixed(TOP_CONTROL_GROUP_W))
        .height(iced::Length::Fixed(TOP_BAR_H))
        .padding([0, 0])
        .align_x(Alignment::End)
        // .style(top_bar_ambient_style(tokens))
        .into()
}

/// 标签页关闭图标按钮
fn icon_tab_close_button(
    tokens: DesignTokens,
    tab_index: usize,
    hit_w: f32,
) -> Element<'static, Message> {
    let close_icon = icon_view_with(
        IconOptions::new(IconId::FnClose).with_size(TAB_CLOSE_ICON_W as u32),
        Message::TabClose(tab_index),
    );
    button(close_icon)
        .on_press(Message::TabClose(tab_index))
        .width(iced::Length::Fixed(hit_w))
        .height(iced::Length::Fixed(hit_w))
        .padding(if hit_w > TAB_CLOSE_ICON_W {
            (hit_w - TAB_CLOSE_ICON_W) / 2.0
        } else {
            0.0
        })
        .style(lg_icon_button_flat(tokens))
        .into()
}
