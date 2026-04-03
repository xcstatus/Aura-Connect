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

use crate::session::{SessionProfile, TransportConfig};
use crate::theme::layout::BOTTOM_BAR_HEIGHT;

use super::chrome::{
    TAB_STRIP_SCROLLABLE_ID, TOP_BAR_EDGE_PAD, TOP_BAR_H, TOP_CONTROL_GROUP_W, TOP_ICON_BTN,
    main_chrome_style, tab_scroll_needs_fade, tab_scroll_right_fade, top_bar_material_style,
    top_bar_vertical_rule, unified_titlebar_padding,
};
use super::engine_adapter::EngineAdapter;
use super::message::Message;
use super::settings_modal;
use super::state::IcedState;
use super::state::{VaultFlowMode, VaultStatus};
use super::terminal_rich;
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

    let mut tabs_row = row![].spacing(4).align_y(Alignment::Center);
    for (i, tab) in state.tabs.iter().enumerate() {
        let active = i == state.active_tab;
        let tab_label = tab.title.clone();
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
                    .width(iced::Length::Fixed(126.0))
                    .height(iced::Length::Fixed(body_h))
                    .align_y(Alignment::Center),
                ]
                .spacing(0),
            )
            .padding([0, 4])
            .width(iced::Length::Fixed(134.0))
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
                let term_font = terminal_rich::iced_terminal_font(&state.model.settings.terminal);
                // Now safe to take mutable borrows.
                let terminal = &*state.active_terminal();
                let cache = &state.tab_panes[state.active_tab].styled_row_cache;
                terminal_rich::styled_terminal(
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
        if state.quick_connect_open {
            layers.push(quick_connect_modal_stack(state));
        }
        if state.settings_modal_open {
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
        text_input("主密码", unlock.password.expose_secret())
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

    body = body.push(
        row![
            button(text("确认").size(13))
                .on_press(Message::VaultUnlockSubmit)
                .style(style_chrome_primary(13.0)),
            button(text("取消").size(13))
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
        .style(modal_scrim_style),
    )
    .on_press(Message::VaultClose);

    let title = match flow.mode {
        VaultFlowMode::Initialize => "初始化保险箱",
        VaultFlowMode::ChangePassword => "修改主密码",
    };

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
            text_input("旧密码", flow.old_password.expose_secret())
                .secure(true)
                .on_input(Message::VaultOldPasswordChanged),
        );
    }
    body = body
        .push(
            text_input("新密码", flow.new_password.expose_secret())
                .secure(true)
                .on_input(Message::VaultNewPasswordChanged),
        )
        .push(
            text_input("确认新密码", flow.confirm_password.expose_secret())
                .secure(true)
                .on_input(Message::VaultConfirmPasswordChanged),
        );

    if let Some(err) = flow.error.as_ref() {
        body = body.push(text(err).size(12));
    }

    body = body.push(
        row![
            button(text("确认").size(13))
                .on_press(Message::VaultSubmit)
                .style(style_chrome_primary(13.0)),
            button(text("取消").size(13))
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

fn modal_scrim_style(_theme: &Theme) -> container::Style {
    container::Style::default().background(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.42))
}

fn quick_connect_modal_stack(state: &IcedState) -> Element<'_, Message> {
    let scrim = mouse_area(
        container(
            Space::new()
                .width(iced::Length::Fill)
                .height(iced::Length::Fill),
        )
        .style(modal_scrim_style),
    )
    .on_press(Message::QuickConnectDismiss);

    let anchored = container(
        container(quick_connect_panel_content(state))
            .max_width(520.0)
            .width(iced::Length::Fill),
    )
    .width(iced::Length::Fill)
    .height(iced::Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Top)
    .padding(iced::Padding {
        top: 6.0,
        right: 16.0,
        bottom: 16.0,
        left: 16.0,
    });

    Stack::with_children([scrim.into(), anchored.into()])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}

fn quick_connect_group_header_style(theme: &Theme) -> container::Style {
    let t = theme.extended_palette();
    container::Style::default().background(t.background.strong.color)
}

fn grouped_ssh_profiles(profiles: &[SessionProfile]) -> Vec<(String, Vec<SessionProfile>)> {
    let mut default_v: Vec<SessionProfile> = Vec::new();
    let mut groups: std::collections::BTreeMap<String, Vec<SessionProfile>> =
        std::collections::BTreeMap::new();
    for p in profiles {
        let TransportConfig::Ssh(_) = &p.transport else {
            continue;
        };
        match p
            .folder
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            None => default_v.push(p.clone()),
            Some(name) => groups.entry(name).or_default().push(p.clone()),
        }
    }
    default_v.sort_by_key(|s| s.name.to_lowercase());
    for v in groups.values_mut() {
        v.sort_by_key(|s| s.name.to_lowercase());
    }
    let mut out = Vec::new();
    if !default_v.is_empty() {
        out.push(("__default__".to_string(), default_v));
    }
    for (k, v) in groups {
        out.push((k, v));
    }
    out
}

fn quick_connect_panel_content(state: &IcedState) -> Element<'_, Message> {
    use super::state::QuickConnectPanel;

    match state.quick_connect_panel {
        QuickConnectPanel::Picker => quick_connect_picker(state),
        QuickConnectPanel::NewConnection => quick_connect_new_form(state),
    }
}

fn quick_connect_picker(state: &IcedState) -> Element<'_, Message> {
    let i18n = &state.model.i18n;

    let query = state.quick_connect_query.trim();
    let direct_parts = crate::connection_input::parse_direct_input(query);
    let is_direct = crate::connection_input::is_direct_candidate(query) && direct_parts.is_some();
    let direct_label = direct_parts.as_ref().map(|p| {
        let user = p.user.as_deref().unwrap_or("<user>");
        let port = p.port.unwrap_or(22);
        format!("{user}@{}:{port}", p.host)
    });
    let direct_cta: Element<'_, Message> = if is_direct && direct_label.is_some() {
        button(
            text(i18n.tr_fmt(
                "iced.quick_connect.direct_cta",
                &[("target", direct_label.as_deref().unwrap_or(""))],
            ))
            .size(13),
        )
        .on_press(Message::QuickConnectDirectSubmit)
        .width(iced::Length::Fill)
        .padding([8, 12])
        .style(style_chrome_primary(13.0))
        .into()
    } else {
        Space::new().into()
    };

    let mut recent_block = column![text(i18n.tr("iced.quick_connect.recent")).size(13)]
        .spacing(6)
        .align_x(Alignment::Start);
    let recent = state.model.recent_connections();
    if recent.is_empty() {
        recent_block = recent_block.push(
            text(i18n.tr("iced.quick_connect.empty_recent"))
                .size(12)
                .style(|theme: &Theme| text::Style {
                    color: Some(
                        theme
                            .extended_palette()
                            .background
                            .base
                            .text
                            .scale_alpha(0.55),
                    ),
                }),
        );
    } else {
        for r in recent.iter() {
            let rec = r.clone();
            let subtitle = format!("{} · {}", r.user, r.host);
            recent_block = recent_block.push(
                button(
                    column![
                        text(r.label.clone()).size(13),
                        text(subtitle).size(11).style(|theme: &Theme| text::Style {
                            color: Some(
                                theme
                                    .extended_palette()
                                    .background
                                    .base
                                    .text
                                    .scale_alpha(0.6)
                            ),
                        }),
                    ]
                    .spacing(2)
                    .align_x(Alignment::Start),
                )
                .on_press(Message::QuickConnectPickRecent(rec))
                .width(iced::Length::Fill)
                .padding([6, 10])
                .style(style_chrome_secondary(13.0)),
            );
        }
    }

    let mut saved_block = column![text(i18n.tr("iced.quick_connect.saved")).size(13)]
        .spacing(8)
        .align_x(Alignment::Start);
    saved_block = saved_block.push(
        button(text(i18n.tr("iced.quick_connect.new_connection")).size(13))
            .on_press(Message::QuickConnectNewConnection)
            .width(iced::Length::Fill)
            .padding([8, 12])
            .style(style_chrome_primary(13.0)),
    );

    let q_lower = query.to_lowercase();
    let saved_profiles: Vec<SessionProfile> = if !q_lower.is_empty() && !is_direct {
        state
            .model
            .profiles()
            .iter()
            .filter(|p| matches!(p.transport, TransportConfig::Ssh(_)))
            .filter(|p| {
                let mut hay = p.name.to_lowercase();
                if let TransportConfig::Ssh(ssh) = &p.transport {
                    hay.push(' ');
                    hay.push_str(&ssh.host.to_lowercase());
                    hay.push(' ');
                    hay.push_str(&ssh.user.to_lowercase());
                }
                hay.contains(&q_lower)
            })
            .cloned()
            .collect()
    } else {
        state
            .model
            .profiles()
            .iter()
            .filter(|p| matches!(p.transport, TransportConfig::Ssh(_)))
            .cloned()
            .collect()
    };

    let default_label = i18n.tr("iced.quick_connect.group_default").to_string();
    for (group_key, sessions) in grouped_ssh_profiles(&saved_profiles) {
        let display = if group_key == "__default__" {
            default_label.clone()
        } else {
            group_key.clone()
        };
        saved_block = saved_block.push(
            container(text(display).size(12))
                .padding(iced::Padding {
                    top: 6.0,
                    right: 0.0,
                    bottom: 4.0,
                    left: 4.0,
                })
                .style(quick_connect_group_header_style)
                .width(iced::Length::Fill),
        );
        for p in sessions {
            let entry_title = p.name.clone();
            let subtitle = if let TransportConfig::Ssh(ssh) = &p.transport {
                format!("{} · {}", ssh.user, ssh.host)
            } else {
                String::new()
            };
            let prof = p.clone();
            saved_block = saved_block.push(
                button(
                    column![
                        text(entry_title).size(13),
                        text(subtitle).size(11).style(|theme: &Theme| text::Style {
                            color: Some(
                                theme
                                    .extended_palette()
                                    .background
                                    .base
                                    .text
                                    .scale_alpha(0.6)
                            ),
                        }),
                    ]
                    .spacing(2)
                    .align_x(Alignment::Start),
                )
                .on_press(Message::ProfileConnect(prof))
                .width(iced::Length::Fill)
                .padding([6, 10])
                .style(style_chrome_secondary(13.0)),
            );
        }
    }

    let body = column![
        row![
            text(i18n.tr("iced.topbar.quick_connect")).size(16),
            Space::new().width(iced::Length::Fill),
            button(text("×").size(14))
                .on_press(Message::QuickConnectDismiss)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(28.0))
                .style(style_top_icon(14.0)),
        ]
        .align_y(Alignment::Center),
        text_input(
            i18n.tr("iced.quick_connect.search_or_direct"),
            &state.quick_connect_query
        )
        .on_input(Message::QuickConnectQueryChanged)
        .on_submit(Message::QuickConnectDirectSubmit)
        .padding([8, 10]),
        direct_cta,
        scrollable(
            column![recent_block, saved_block]
                .spacing(16)
                .width(iced::Length::Fill),
        )
        .height(iced::Length::Fixed(440.0))
        .width(iced::Length::Fill),
    ]
    .spacing(10)
    .width(iced::Length::Fill);

    container(body)
        .width(iced::Length::Fill)
        .padding(16)
        .style(top_bar_material_style)
        .into()
}

fn quick_connect_new_form(state: &IcedState) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let is_connected = state.active_session_is_connected();
    let flow = state.quick_connect_flow;
    let err_kind = state.quick_connect_error_kind;
    let stage = state.connection_stage;
    let is_connecting = matches!(flow, super::state::QuickConnectFlow::Connecting);

    // Form inputs are disabled while connecting.
    let host_row = row![
        text_input(i18n.tr("iced.field.host"), &state.model.draft.host)
            .on_input(Message::HostChanged)
            .width(iced::Length::FillPortion(3)),
        text_input(i18n.tr("iced.field.port"), &state.model.draft.port)
            .on_input(Message::PortChanged)
            .width(iced::Length::FillPortion(1)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let user_row = row![
        text_input(i18n.tr("iced.field.user"), &state.model.draft.user)
            .on_input(Message::UserChanged)
            .width(iced::Length::FillPortion(1)),
        text_input(
            i18n.tr("iced.field.password"),
            state.model.draft.password.expose_secret(),
        )
        .secure(true)
        .on_input(Message::PasswordChanged)
        .width(iced::Length::FillPortion(1)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let auth_options: Vec<crate::session::AuthMethod> = vec![
        crate::session::AuthMethod::Password,
        crate::session::AuthMethod::Interactive,
        crate::session::AuthMethod::Key {
            private_key_path: String::new(),
        },
    ];
    let auth_row = row![
        pick_list(
            auth_options,
            Some(state.model.draft.auth.clone()),
            Message::QuickConnectAuthChanged
        )
        .width(iced::Length::Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let flow_banner: Option<Element<'_, Message>> = match flow {
        super::state::QuickConnectFlow::NeedUser => Some(
            container(
                text(
                    err_kind
                        .unwrap_or(crate::app_model::ConnectErrorKind::MissingHostOrUser)
                        .user_message(),
                )
                .size(12),
            )
            .padding(10)
            .style(top_bar_material_style)
            .into(),
        ),
        super::state::QuickConnectFlow::NeedAuthPassword => {
            let msg = if err_kind == Some(crate::app_model::ConnectErrorKind::AuthFailed) {
                let n = state.model.draft.password_error_count;
                format!("SSH  密码错误（{}/3）：请重新输入密码。", n)
            } else {
                "SSH  需要密码认证。".to_string()
            };
            Some(
                container(text(msg).size(12))
                    .padding(10)
                    .style(top_bar_material_style)
                    .into(),
            )
        }
        super::state::QuickConnectFlow::AuthLocked => Some(
            container(text("SSH  密码多次错误，已中断本次连接。请编辑后重试或切换认证方式。").size(12))
                .padding(10)
                .style(top_bar_material_style)
                .into(),
        ),
        super::state::QuickConnectFlow::Failed => Some(
            container(
                text(
                    err_kind
                        .unwrap_or(crate::app_model::ConnectErrorKind::Unknown)
                        .user_message(),
                )
                .size(12),
            )
            .padding(10)
            .style(top_bar_material_style)
            .into(),
        ),
        _ => None,
    };

    // Animated connecting progress indicator: shows current connection stage with animated dots.
    let connecting_progress: Option<Element<'_, Message>> = if is_connecting {
        let dots = match state.tick_count % 3 {
            0 => "",
            1 => ".",
            _ => "..",
        };
        let stage_label = match stage {
            super::state::ConnectionStage::VaultLoading => {
                state.model.i18n.tr("iced.stage.vault_loading")
            }
            super::state::ConnectionStage::SshConnecting => {
                state.model.i18n.tr("iced.stage.ssh_connecting")
            }
            super::state::ConnectionStage::Authenticating => {
                state.model.i18n.tr("iced.stage.authenticating")
            }
            super::state::ConnectionStage::SessionSetup => {
                state.model.i18n.tr("iced.stage.session_setup")
            }
            _ => state.model.i18n.tr("iced.term.connecting"),
        };
        Some(
            container(
                row![
                    text("⟳").size(14),
                    text(format!("{stage_label}{dots}")).size(12),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding(10)
            .style(top_bar_material_style)
            .into(),
        )
    } else {
        None
    };

    let key_row: Option<Element<'_, Message>> = if matches!(
        state.model.draft.auth,
        crate::session::AuthMethod::Key { .. }
    ) {
        Some(
            column![
                text_input("Private key path", &state.model.draft.private_key_path)
                    .on_input(Message::QuickConnectKeyPathChanged)
                    .width(iced::Length::Fill),
                text_input(
                    "Passphrase (optional)",
                    state.model.draft.passphrase.expose_secret()
                )
                .secure(true)
                .on_input(Message::QuickConnectPassphraseChanged)
                .width(iced::Length::Fill),
            ]
            .spacing(8)
            .into(),
        )
    } else {
        None
    };

    let interactive_row: Option<Element<'_, Message>> = if matches!(
        state.quick_connect_flow,
        super::state::QuickConnectFlow::NeedAuthInteractive
    ) {
        state.quick_connect_interactive.as_ref().map(|flow| {
            let mut col = column![
                text(flow.ui.name.clone()).size(13),
                text(flow.ui.instructions.clone()).size(12),
            ]
            .spacing(6)
            .width(iced::Length::Fill);
            for (i, p) in flow.ui.prompts.iter().enumerate() {
                let ans = flow.ui.answers.get(i).cloned().unwrap_or_default();
                col = col.push(
                    column![
                        text(p.prompt.clone()).size(12),
                        text_input("", &ans)
                            .secure(!p.echo)
                            .on_input(move |v| Message::QuickConnectInteractiveAnswerChanged(i, v))
                            .width(iced::Length::Fill),
                    ]
                    .spacing(4),
                );
            }
            if let Some(err) = flow.ui.error.as_ref() {
                col = col.push(text(err).size(12));
            }
            col = col.push(
                row![
                    button(text("提交").size(13))
                        .on_press(Message::QuickConnectInteractiveSubmit)
                        .style(style_chrome_primary(13.0)),
                ]
                .spacing(8),
            );
            container(col)
                .padding(12)
                .style(top_bar_material_style)
                .width(iced::Length::Fill)
                .into()
        })
    } else {
        None
    };

    let saved_session_hint: Option<Element<'_, Message>> =
        state.model.selected_session_id.as_deref().and_then(|pid| {
            state
                .model
                .profiles()
                .iter()
                .find(|s| s.id == pid)
                .map(|p| {
                    text(format!(
                        "Saved session: {} — confirm below, then click Connect.",
                        p.name
                    ))
                    .size(12)
                    .style(|theme: &Theme| text::Style {
                        color: Some(
                            theme
                                .extended_palette()
                                .background
                                .base
                                .text
                                .scale_alpha(0.75),
                        ),
                    })
                    .into()
                })
        });

    // Loading spinner: animated rotating symbol based on tick parity.
    let spinner = if state.tick_count % 2 == 0 { "◐" } else { "◓" };
    let connecting_label = i18n.tr("iced.btn.connecting");
    let connect_btn_text = if is_connecting {
        format!("{spinner} {connecting_label}")
    } else {
        i18n.tr("iced.btn.connect").to_string()
    };
    let actions = row![
        button(text(connect_btn_text).size(13))
            .on_press_maybe(
                (!matches!(
                    flow,
                    super::state::QuickConnectFlow::Connecting
                        | super::state::QuickConnectFlow::AuthLocked
                        | super::state::QuickConnectFlow::NeedAuthInteractive
                ))
                .then_some(Message::ConnectPressed),
            )
            .style(style_chrome_primary(13.0)),
        button(text(i18n.tr("iced.btn.disconnect")).size(13))
            .on_press_maybe(is_connected.then_some(Message::DisconnectPressed))
            .style(style_chrome_secondary(13.0)),
        button(text(i18n.tr("iced.btn.save_settings")).size(13))
            .on_press(Message::SaveSettings)
            .style(style_chrome_secondary(13.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // During connecting: hide back/dismiss buttons and show a spinner in the header.
    let header_row: Element<'_, Message> = if is_connecting {
        row![
            Space::new().width(iced::Length::Fixed(22.0)),
            text(format!("{spinner} {connecting_label}")).size(16),
            Space::new().width(iced::Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    } else {
        row![
            button(text(i18n.tr("iced.quick_connect.back")).size(12))
                .on_press(Message::QuickConnectBackToList)
                .style(style_chrome_secondary(12.0)),
            text(i18n.tr("iced.quick_connect.new_title")).size(16),
            Space::new().width(iced::Length::Fill),
            button(text("×").size(14))
                .on_press(Message::QuickConnectDismiss)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(28.0))
                .style(style_top_icon(14.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    };

    let mut form_cols = column![header_row, text(i18n.tr("iced.title.subtitle")).size(12)]
        .spacing(14)
        .width(iced::Length::Fill);
    if let Some(b) = flow_banner {
        form_cols = form_cols.push(b);
    }
    if let Some(p) = connecting_progress {
        form_cols = form_cols.push(p);
    }
    if let Some(h) = saved_session_hint {
        form_cols = form_cols.push(h);
    }
    let mut form_cols = form_cols.push(host_row).push(auth_row).push(user_row);
    if let Some(k) = key_row {
        form_cols = form_cols.push(k);
    }
    if let Some(ir) = interactive_row {
        form_cols = form_cols.push(ir);
    }
    let body = form_cols.push(actions).spacing(14);

    container(body)
        .width(iced::Length::Fill)
        .padding(16)
        .style(top_bar_material_style)
        .into()
}
