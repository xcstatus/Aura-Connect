use iced::alignment::Alignment;
use iced::widget::Id;
use iced::widget::scrollable::{Direction as ScrollDirection, Scrollbar};
use iced::widget::tooltip::Position;
use iced::widget::{
    Space, Stack, button, checkbox, column, container, mouse_area, pick_list, row, scrollable,
    text, text_input, tooltip,
};
use iced::{Element, Theme};

use secrecy::ExposeSecret;

use crate::theme::layout::BOTTOM_BAR_HEIGHT;

use super::chrome::{
    TAB_STRIP_SCROLLABLE_ID, TOP_BAR_EDGE_PAD, TOP_BAR_H, TOP_CONTROL_GROUP_W, TOP_ICON_BTN,
    main_chrome_style, tab_scroll_needs_fade, tab_scroll_right_fade, top_bar_material_style,
    top_bar_vertical_rule, unified_titlebar_padding,
};
use super::components::helpers::modal_scrim_style;
use super::components::quick_connect;
use super::components::overlays;
use super::engine_adapter::EngineAdapter;
use super::message::Message;
use super::settings_modal;
use super::state::IcedState;
use super::state::{VaultFlowMode, VaultStatus};
use super::terminal_widget;
use super::terminal_viewport;
use super::widgets::chrome_button::{
    style_chrome_primary, style_chrome_secondary, style_tab_strip, style_top_icon,
};

pub(crate) fn view(state: &IcedState) -> Element<'_, Message> {
    let term_vp =
        terminal_viewport::terminal_viewport_spec_for_settings(&state.model.settings.terminal);
    let i18n = &state.model.i18n;
    let is_connected = state.active_session_is_connected();
    let current_node = state
        .model
        .selected_session_id
        .as_deref()
        .unwrap_or(i18n.tr("iced.breadcrumb.not_connected"));
    let cwd = std::env::current_dir()
        .ok()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "-".to_string());
    let cell_w_hit = terminal_viewport::terminal_scroll_cell_geometry(
        state.window_size,
        &term_vp,
        EngineAdapter::active(state).grid_size().0,
    )
    .1;

    let tick_ms = state.tick_ms();

    let mut tabs_row = row![].spacing(4).align_y(Alignment::Center);
    for (i, tab) in state.tabs.iter().enumerate() {
        let active = i == state.active_tab;
        let tab_label = tab.title.clone();
        let tab_w = state.tab_animated_width(i, tick_ms);
        let select_btn = button(text(tab_label).size(11))
            .on_press(Message::TabSelected(i))
            .width(iced::Length::Fill)
            .style(style_tab_strip(11.0));
        let close_slot: Element<'_, Message> = if state.tab_hover_index == Some(i) {
            button(text("×").size(12))
                .on_press(Message::TabClose(i))
                .width(iced::Length::Fixed(22.0))
                .style(style_tab_strip(12.0))
                .into()
        } else {
            Space::new()
                .width(iced::Length::Fixed(22.0))
                .height(iced::Length::Fixed(1.0))
                .into()
        };
        let body_h = if active { TOP_BAR_H - 2.0 } else { TOP_BAR_H };
        let top_line = container(
            Space::new()
                .width(iced::Length::Fill)
                .height(iced::Length::Fixed(if active { 2.0 } else { 0.0 })),
        )
        .style(move |theme: &Theme| {
            if active {
                let c = theme.extended_palette().primary.base.color;
                container::Style::default().background(c)
            } else {
                container::Style::default()
            }
        });
        let chip = mouse_area(
            container(
                column![
                    top_line,
                    container(
                        row![select_btn, close_slot]
                            .spacing(0)
                            .align_y(Alignment::Center),
                    )
                    .width(iced::Length::Fixed(tab_w))
                    .height(iced::Length::Fixed(body_h))
                    .align_y(Alignment::Center),
                ]
                .spacing(0),
            )
            .padding([0, 4])
            .width(iced::Length::Fixed(tab_w + 8.0))
            .height(iced::Length::Fixed(TOP_BAR_H)),
        )
        .on_enter(Message::TabChipHover(Some(i)))
        .on_exit(Message::TabChipHover(None));
        tabs_row = tabs_row.push(chip);
    }

    let btn_quick = button(text("⚡").size(14))
        .on_press(Message::TopQuickConnect)
        .width(iced::Length::Fixed(TOP_ICON_BTN))
        .height(iced::Length::Fixed(TOP_ICON_BTN))
        .style(style_top_icon(TOP_ICON_BTN));
    let btn_new = button(text("+").size(18))
        .on_press(Message::TopAddTab)
        .width(iced::Length::Fixed(TOP_ICON_BTN))
        .height(iced::Length::Fixed(TOP_ICON_BTN))
        .style(style_top_icon(TOP_ICON_BTN));
    let quick_tip = text(i18n.tr("iced.topbar.quick_connect")).size(12);
    let new_tip = text(i18n.tr("iced.topbar.new_tab")).size(12);
    // 顶栏色块：与毛玻璃顶栏一致，紧挨竖线右侧
    let action_group = container(
        row![
            tooltip(btn_quick, quick_tip, Position::Bottom),
            tooltip(btn_new, new_tip, Position::Bottom),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .height(iced::Length::Fixed(TOP_BAR_H))
    .padding([0, 8])
    .style(top_bar_material_style)
    .align_y(Alignment::Center);

    // 仅标签 + 竖线在横向 scrollable 内；操作组固定在右侧不参与滚动。
    let tabs_only_scroll = scrollable(
        row![tabs_row, top_bar_vertical_rule()]
            .spacing(0)
            .align_y(Alignment::Center),
    )
    .id(Id::new(TAB_STRIP_SCROLLABLE_ID))
    .direction(ScrollDirection::Horizontal(Scrollbar::hidden()))
    .height(iced::Length::Fixed(TOP_BAR_H))
    .width(iced::Length::Fill);

    let tab_scroll_core = container(tabs_only_scroll)
        .width(iced::Length::Fill)
        .height(iced::Length::Fixed(TOP_BAR_H))
        .style(main_chrome_style);

    let tab_scroll_host = mouse_area(tab_scroll_core).on_scroll(Message::TabStripWheel);

    let show_tab_scroll_fade = tab_scroll_needs_fade(state.tabs.len(), state.window_size.width);
    let tab_strip_area: Element<'_, Message> = if show_tab_scroll_fade {
        Stack::with_children([tab_scroll_host.into(), tab_scroll_right_fade().into()])
            .width(iced::Length::Fill)
            .height(iced::Length::Fixed(TOP_BAR_H))
            .into()
    } else {
        tab_scroll_host.into()
    };

    let win_controls: Element<'_, Message> = {
        #[cfg(not(target_os = "macos"))]
        {
            row![
                button(text("—").size(12))
                    .on_press(Message::WinMinimize)
                    .width(iced::Length::Fixed(28.0))
                    .height(iced::Length::Fixed(26.0))
                    .style(style_top_icon(12.0)),
                button(text("□").size(11))
                    .on_press(Message::WinToggleMaximize)
                    .width(iced::Length::Fixed(28.0))
                    .height(iced::Length::Fixed(26.0))
                    .style(style_top_icon(11.0)),
                button(text("×").size(12))
                    .on_press(Message::WinClose)
                    .width(iced::Length::Fixed(28.0))
                    .height(iced::Length::Fixed(26.0))
                    .style(style_top_icon(12.0)),
            ]
            .spacing(2)
            .align_y(Alignment::Center)
            .into()
        }
        #[cfg(target_os = "macos")]
        {
            Space::new().into()
        }
    };

    let btn_settings = button(text("⚙").size(15))
        .on_press(Message::TopOpenSettings)
        .width(iced::Length::Fixed(TOP_ICON_BTN))
        .height(iced::Length::Fixed(TOP_ICON_BTN))
        .style(style_top_icon(TOP_ICON_BTN));
    let settings_tip = text(i18n.tr("iced.topbar.settings_center")).size(12);
    let settings_ctrl = tooltip(btn_settings, settings_tip, Position::Bottom);

    // 控制组：顶栏色、整体顶栏最右侧，内容右对齐
    let control_group = container(
        row![
            Space::new().width(iced::Length::Fill),
            settings_ctrl,
            win_controls
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .width(iced::Length::Fixed(TOP_CONTROL_GROUP_W))
    .height(iced::Length::Fixed(TOP_BAR_H))
    .padding([0, 0])
    .style(top_bar_material_style);

    // 滚动区与控制组之间的窄分隔带（顶栏色，与右侧固定区分开）
    let scroll_control_gutter = container(Space::new().height(iced::Length::Fixed(TOP_BAR_H)))
        .width(super::chrome::SCROLL_TO_CONTROL_GUTTER_W)
        .style(top_bar_material_style);

    let mut top_bar_row = row![].spacing(0).align_y(Alignment::Center);
    #[cfg(target_os = "macos")]
    {
        top_bar_row = top_bar_row.push(
            container(Space::new().height(iced::Length::Fixed(TOP_BAR_H)))
                .width(super::chrome::TRAFFIC_LIGHT_BAND_W)
                .style(top_bar_material_style),
        );
    }
    top_bar_row = top_bar_row
        .push(tab_strip_area)
        .push(action_group)
        .push(scroll_control_gutter)
        .push(control_group);

    let top_bar = container(top_bar_row)
        .height(iced::Length::Fixed(TOP_BAR_H))
        .padding(iced::Padding::from([0.0, TOP_BAR_EDGE_PAD]));

    let _selection = EngineAdapter::active(state).selection();
    let conn_label = if is_connected {
        i18n.tr("iced.breadcrumb.connected")
    } else {
        i18n.tr("iced.breadcrumb.disconnected")
    };
    let breadcrumb = container(
        row![
            text(conn_label),
            text("/"),
            text(current_node),
            text("|"),
            text(cwd),
            row![
                button(text(i18n.tr("iced.btn.reconnect")).size(12))
                    .on_press(Message::ConnectPressed)
                    .style(style_chrome_secondary(12.0)),
                button(text(i18n.tr("iced.btn.sftp")).size(12)).style(style_chrome_secondary(12.0)),
                button(text(i18n.tr("iced.btn.port_fwd")).size(12))
                    .style(style_chrome_secondary(12.0)),
            ]
            .spacing(8),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding(term_vp.breadcrumb_padding())
    .height(iced::Length::Fixed(term_vp.breadcrumb_block_h()))
    .align_y(Alignment::Center);

    // Terminal body: always uses per-row Styled runs via `terminal_rich`.
    let terminal_panel: Element<'_, Message> = container(
        column![
            container({
                // Extract all state reads first, before any mutable borrows.
                let selection = {
                    let engine = EngineAdapter::active(state);
                    engine.selection()
                };
                let tick_count = state.tick_count;
                let term_font_px = term_vp.term_font_px;
                let term_cell_h = iced::Pixels(term_vp.term_cell_h().max(1.0));
                let term_font = terminal_widget::iced_terminal_font(&state.model.settings.terminal);
                // Now safe to take mutable borrows.
                let terminal = &*state.active_terminal();
                let cache = &state.tab_panes[state.active_tab].styled_row_cache;
                terminal_widget::styled_terminal(
                    terminal,
                    cache,
                    selection,
                    term_font_px,
                    term_cell_h,
                    term_font,
                    cell_w_hit,
                    tick_count,
                )
            })
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .style(container::bordered_box),
        ]
        .spacing(term_vp.terminal_panel_inner_spacing())
        .height(iced::Length::Fill),
    )
    .padding(term_vp.terminal_panel_padding())
    .height(iced::Length::Fill)
    .into();

    let main_body: Element<'_, Message> = terminal_panel;

    let status_word = if is_connected {
        i18n.tr("iced.breadcrumb.connected")
    } else {
        i18n.tr("iced.breadcrumb.disconnected")
    };
    let engine = EngineAdapter::active(state);
    let term_scroll = engine.scroll();
    let term_in_scrollback = engine.is_in_scrollback();
    let term_scroll_word = if term_in_scrollback {
        "回滚中"
    } else {
        "跟随底部"
    };
    let vault_word = match state.vault_status {
        VaultStatus::Uninitialized => "未初始化",
        VaultStatus::Unlocked => "已解锁",
        VaultStatus::Locked => "已锁定",
        VaultStatus::Unavailable => "不可用",
    };
    let bottom_bar = container(
        row![
            text(format!(
                "{}: {}",
                i18n.tr("iced.footer.status"),
                status_word
            )),
            text("|"),
            text(format!(
                "{}: {}",
                "终端",
                if term_in_scrollback {
                    format!(
                        "{} ({}/{})",
                        term_scroll_word,
                        term_scroll.offset_rows,
                        term_scroll
                            .total_rows
                            .saturating_sub(term_scroll.viewport_rows)
                    )
                } else {
                    term_scroll_word.to_string()
                }
            )),
            text("|"),
            text(format!(
                "{}: {}",
                i18n.tr("iced.footer.hint"),
                state.model.status
            )),
            text("|"),
            text(format!("{}: {}", i18n.tr("iced.footer.vault"), vault_word)),
            text("|"),
            text(format!("{}: iced", i18n.tr("iced.footer.runtime"))),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .width(iced::Length::Fill)
    .height(iced::Length::Fixed(BOTTOM_BAR_HEIGHT))
    .padding([0, 12])
    .style(main_chrome_style);

    // 顶栏 32px | 终端区（面包屑 + 主内容 + 底栏，快速连接浮层叠在终端区内）
    let below_top_fill = column![breadcrumb, main_body, bottom_bar,]
        .spacing(term_vp.main_column_spacing())
        .height(iced::Length::Fill);

    let under_top_bar: Element<'_, Message> = {
        let mut layers: Vec<Element<'_, Message>> = vec![below_top_fill.into()];

        // Terminal-inline overlay: connection progress bar (shown while modal is closed).
        layers.push(overlays::inline_connecting_overlay(state));
        // Terminal-inline overlay: password/passphrase input (modal closed, need credential).
        layers.push(overlays::inline_password_overlay(state));

        // Render quick connect modal when anim phase is opening/open/closing
        // (not yet fully closed). This allows the close animation to play.
        if state.quick_connect_anim.phase != super::state::ModalAnimPhase::Closed {
            layers.push(quick_connect::quick_connect_modal_stack(state));
        }
        if state.settings_anim.phase != super::state::ModalAnimPhase::Closed {
            layers.push(settings_modal::modal_stack(state));
            layers.push(session_editor_modal_stack(state));
            layers.push(vault_modal_stack(state));
            layers.push(auto_probe_consent_modal_stack(state));
        }
        layers.push(host_key_prompt_stack(state));
        // Vault unlock modal must be above all other modals.
        // (e.g. connect-from-saved requiring vault unlock, or post-connect save-credential prompt)
        layers.push(vault_unlock_modal_stack(state));
        // Debug overlay: toggled with Ctrl+Shift+D; renders on top of everything.
        if state.perf.debug_overlay_enabled {
            layers.push(super::widgets::debug_overlay::make_debug_overlay(state));
        }
        Stack::with_children(layers)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into()
    };

    let terminal_region = container(under_top_bar)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .style(main_chrome_style);

    let chrome = column![top_bar, terminal_region].height(iced::Length::Fill);

    container(chrome)
        .padding(unified_titlebar_padding())
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}

fn auto_probe_consent_modal_stack(state: &IcedState) -> Element<'_, Message> {
    let Some(_m) = state.auto_probe_consent_modal.as_ref() else {
        return Space::new().into();
    };
    let scrim = mouse_area(
        container(
            Space::new()
                .width(iced::Length::Fill)
                .height(iced::Length::Fill),
        )
        .style(modal_scrim_style),
    )
    .on_press(Message::AutoProbeConsentUsePassword);

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
        .style(top_bar_material_style);
    let centered = container(card)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    Stack::with_children([scrim.into(), centered.into()])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}

fn vault_unlock_modal_stack(state: &IcedState) -> Element<'_, Message> {
    let Some(unlock) = state.vault_unlock.as_ref() else {
        return Space::new().into();
    };
    let scrim = mouse_area(
        container(
            Space::new()
                .width(iced::Length::Fill)
                .height(iced::Length::Fill),
        )
        .style(modal_scrim_style),
    )
    .on_press(Message::VaultUnlockClose);

    let title = if unlock.pending_save_credentials_profile_id.is_some() {
        state
            .model
            .i18n
            .tr("iced.vault_unlock.title_save_credentials")
    } else {
        state.model.i18n.tr("iced.vault_unlock.title")
    };
    let mut body = column![
        row![
            text(title).size(16),
            Space::new().width(iced::Length::Fill),
            button(text("×").size(14))
                .on_press(Message::VaultUnlockClose)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(28.0))
                .style(style_top_icon(14.0)),
        ]
        .align_y(Alignment::Center),
        text_input(state.model.i18n.tr("iced.vault_unlock.password_placeholder"), unlock.password.expose_secret())
            .secure(true)
            .on_input(Message::VaultUnlockPasswordChanged),
    ]
    .spacing(10)
    .width(iced::Length::Fill);

    if unlock.pending_save_credentials_profile_id.is_some() {
        body = body.push(
            text(
                state
                    .model
                    .i18n
                    .tr("iced.vault_unlock.hint_save_credentials"),
            )
            .size(12),
        );
    }

    if let Some(err) = unlock.error.as_ref() {
        body = body.push(text(err).size(12));
    }

    let i18n = &state.model.i18n;
    body = body.push(
        row![
            button(text(i18n.tr("iced.vault_unlock.btn.confirm")).size(13))
                .on_press(Message::VaultUnlockSubmit)
                .style(style_chrome_primary(13.0)),
            button(text(i18n.tr("iced.vault_unlock.btn.cancel")).size(13))
                .on_press(Message::VaultUnlockClose)
                .style(style_chrome_secondary(13.0)),
        ]
        .spacing(8),
    );

    let card = container(body)
        .width(iced::Length::Fixed(520.0))
        .padding(16)
        .style(top_bar_material_style);
    let centered = container(card)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    Stack::with_children([scrim.into(), centered.into()])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}

fn host_key_prompt_stack(state: &IcedState) -> Element<'_, Message> {
    let Some(p) = state.host_key_prompt.as_ref() else {
        return Space::new().into();
    };
    let i18n = &state.model.i18n;
    let info = &p.info;

    let scrim = mouse_area(
        container(
            Space::new()
                .width(iced::Length::Fill)
                .height(iced::Length::Fill),
        )
        .style(modal_scrim_style),
    )
    .on_press(Message::HostKeyReject);

    let mut body = column![
        row![
            text(i18n.tr("iced.host_key_prompt.title")).size(16),
            Space::new().width(iced::Length::Fill),
            button(text("×").size(14))
                .on_press(Message::HostKeyReject)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(28.0))
                .style(style_top_icon(14.0)),
        ]
        .align_y(Alignment::Center),
        text(i18n.tr_fmt(
            "iced.host_key_prompt.host_line",
            &[
                ("host", &info.host),
                ("port", &info.port.to_string()),
                ("algo", &info.algo),
            ],
        ))
        .size(12),
    ]
    .spacing(10)
    .width(iced::Length::Fill);

    if let Some(old) = info.old_fingerprint.as_ref() {
        body = body.push(
            text(i18n.tr_fmt("iced.host_key_prompt.old_fingerprint", &[("fp", old)])).size(12),
        );
    }

    body = body
        .push(
            text(i18n.tr_fmt(
                "iced.host_key_prompt.new_fingerprint",
                &[("fp", &info.fingerprint)],
            ))
            .size(12),
        )
        .push(
            text(match state.model.settings.security.host_key_policy {
                crate::settings::HostKeyPolicy::Strict => {
                    i18n.tr("settings.security.hosts.policy.strict")
                }
                crate::settings::HostKeyPolicy::Ask => {
                    i18n.tr("settings.security.hosts.policy.ask")
                }
                crate::settings::HostKeyPolicy::AcceptNew => {
                    i18n.tr("settings.security.hosts.policy.accept_new")
                }
            })
            .size(12),
        )
        .push(
            row![
                button(text(i18n.tr("iced.host_key_prompt.accept_once")).size(13))
                    .on_press(Message::HostKeyAcceptOnce)
                    .style(style_chrome_secondary(13.0)),
                button(text(i18n.tr("iced.host_key_prompt.always_trust")).size(13))
                    .on_press(Message::HostKeyAlwaysTrust)
                    .style(style_chrome_primary(13.0)),
                button(text(i18n.tr("iced.host_key_prompt.reject")).size(13))
                    .on_press(Message::HostKeyReject)
                    .style(style_chrome_secondary(13.0)),
            ]
            .spacing(8),
        );

    let card = container(body)
        .width(iced::Length::Fixed(560.0))
        .padding(16)
        .style(top_bar_material_style);
    let centered = container(card)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    Stack::with_children([scrim.into(), centered.into()])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}

fn session_editor_modal_stack(state: &IcedState) -> Element<'_, Message> {
    let Some(ed) = state.session_editor.as_ref() else {
        return Space::new().into();
    };
    let i18n = &state.model.i18n;
    let scrim = mouse_area(
        container(
            Space::new()
                .width(iced::Length::Fill)
                .height(iced::Length::Fill),
        )
        .style(modal_scrim_style),
    )
    .on_press(Message::SessionEditorClose);

    let auth_options: Vec<crate::session::AuthMethod> = vec![
        crate::session::AuthMethod::Password,
        crate::session::AuthMethod::Agent,
        crate::session::AuthMethod::Interactive,
        crate::session::AuthMethod::Key {
            private_key_path: String::new(),
        },
    ];
    let mut body = column![
        row![
            text(i18n.tr("iced.settings.conn.edit")).size(16),
            Space::new().width(iced::Length::Fill),
            button(text("×").size(14))
                .on_press(Message::SessionEditorClose)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(28.0))
                .style(style_top_icon(14.0)),
        ]
        .align_y(Alignment::Center),
        text_input(i18n.tr("iced.field.host"), &ed.host)
            .on_input(Message::SessionEditorHostChanged)
            .width(iced::Length::Fill),
        text_input(i18n.tr("iced.field.port"), &ed.port)
            .on_input(Message::SessionEditorPortChanged)
            .width(iced::Length::Fill),
        text_input(i18n.tr("iced.field.user"), &ed.user)
            .on_input(Message::SessionEditorUserChanged)
            .width(iced::Length::Fill),
        pick_list(
            auth_options,
            Some(ed.auth.clone()),
            Message::SessionEditorAuthChanged
        )
        .placeholder("Auth"),
    ]
    .spacing(10)
    .width(iced::Length::Fill);

    if matches!(ed.auth, crate::session::AuthMethod::Password) {
        body = body.push(
            text_input(i18n.tr("iced.field.password"), ed.password.expose_secret())
                .secure(true)
                .on_input(Message::SessionEditorPasswordChanged)
                .width(iced::Length::Fill),
        );
    }
    if ed.existing_credential_id.is_some() {
        body = body.push(
            checkbox(ed.clear_saved_password)
                .label("清除已保存密码")
                .on_toggle(Message::SessionEditorClearPasswordToggled),
        );
    }

    if let Some(err) = ed.error.as_ref() {
        body = body.push(text(err).size(12));
    }

    let actions = row![
        button(text(i18n.tr("iced.btn.save_settings")).size(13))
            .on_press(Message::SessionEditorSave)
            .style(style_chrome_primary(13.0)),
        button(text(i18n.tr("iced.quick_connect.back")).size(13))
            .on_press(Message::SessionEditorClose)
            .style(style_chrome_secondary(13.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    body = body.push(actions);

    let card = container(body)
        .width(iced::Length::Fixed(520.0))
        .padding(16)
        .style(top_bar_material_style);
    let centered = container(card)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    Stack::with_children([scrim.into(), centered.into()])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}

fn vault_modal_stack(state: &IcedState) -> Element<'_, Message> {
    let Some(flow) = state.vault_flow.as_ref() else {
        return Space::new().into();
    };
    let scrim = mouse_area(
        container(
            Space::new()
                .width(iced::Length::Fill)
                .height(iced::Length::Fill),
        )
        .style(super::components::helpers::modal_scrim_style),
    )
    .on_press(Message::VaultClose);

    let title = match flow.mode {
        VaultFlowMode::Initialize => state.model.i18n.tr("iced.vault.title.initialize"),
        VaultFlowMode::ChangePassword => state.model.i18n.tr("iced.vault.title.change_password"),
    };
    let i18n = &state.model.i18n;

    let mut body = column![
        row![
            text(title).size(16),
            Space::new().width(iced::Length::Fill),
            button(text("×").size(14))
                .on_press(Message::VaultClose)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(28.0))
                .style(style_top_icon(14.0)),
        ]
        .align_y(Alignment::Center),
    ]
    .spacing(10)
    .width(iced::Length::Fill);

    if matches!(flow.mode, VaultFlowMode::ChangePassword) {
        body = body.push(
            text_input(state.model.i18n.tr("iced.vault.label.old_password"), flow.old_password.expose_secret())
                .secure(true)
                .on_input(Message::VaultOldPasswordChanged),
        );
    }
    body = body
        .push(
            text_input(state.model.i18n.tr("iced.vault.label.new_password"), flow.new_password.expose_secret())
                .secure(true)
                .on_input(Message::VaultNewPasswordChanged),
        )
        .push(
            text_input(state.model.i18n.tr("iced.vault.label.confirm_password"), flow.confirm_password.expose_secret())
                .secure(true)
                .on_input(Message::VaultConfirmPasswordChanged),
        );

    if let Some(err) = flow.error.as_ref() {
        body = body.push(text(err).size(12));
    }

    body = body.push(
        row![
            button(text(i18n.tr("iced.btn.confirm")).size(13))
                .on_press(Message::VaultSubmit)
                .style(style_chrome_primary(13.0)),
            button(text(i18n.tr("iced.btn.cancel")).size(13))
                .on_press(Message::VaultClose)
                .style(style_chrome_secondary(13.0)),
        ]
        .spacing(8),
    );

    let card = container(body)
        .width(iced::Length::Fixed(520.0))
        .padding(16)
        .style(top_bar_material_style);
    let centered = container(card)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    Stack::with_children([scrim.into(), centered.into()])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}
