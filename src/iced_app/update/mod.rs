//! Update handlers for iced application.
//!
//! This module is organized into submodules by responsibility:
//! - `tick.rs` - Tick message handler (session pumping, perf counters, cursor blink)
//! - `connection.rs` - Connection logic (SSH, interactive auth, host keys)
//! - `session.rs` - Tab and session editor management
//! - `settings.rs` - Settings handling
//! - `vault.rs` - Vault (password manager) flows

pub mod connection;
pub mod session;
pub mod settings;
pub mod tick;
pub mod vault;

use iced::keyboard;
use iced::Size;
use iced::Task;
use iced::event::Event;
use iced::mouse;
use iced::mouse::ScrollDelta;
use iced::widget::Id;
use iced::widget::operation::{AbsoluteOffset, scroll_by};

use crate::backend::ssh_session::AsyncSession;

use super::chrome::TAB_STRIP_SCROLLABLE_ID;
use super::message::Message;
use super::state::{IcedState, QuickConnectPanel, TabPane};
use super::terminal_event::TerminalEvent;
use super::terminal_host::TerminalHost;
use super::terminal_viewport;

/// Resize PTY/grid from window size for a single tab.
pub(crate) fn apply_terminal_grid_resize_for_pane(
    pane: &mut TabPane,
    window: Size,
    terminal_settings: &crate::settings::TerminalSettings,
) {
    let spec = terminal_viewport::terminal_viewport_spec_for_settings(terminal_settings);
    let (cols, rows) = terminal_viewport::grid_from_window_size_with_spec(window, &spec);

    let prev_rows = pane.terminal.grid_size().1;
    pane.terminal.resize(cols, rows);

    // Invalidate row widget cache when viewport row count changes (e.g. resize).
    if prev_rows != rows {
        pane.styled_row_cache.clear();
    }
}

/// Resize the active tab's PTY to current window size (syncs to SSH PTY if session exists).
pub(crate) fn sync_terminal_grid_to_session(state: &mut IcedState) {
    let term_settings = &state.model.settings.terminal;
    let pane = &mut state.tab_panes[state.active_tab];
    let window_size = state.window_size;
    let (cols, rows) = {
        let spec = terminal_viewport::terminal_viewport_spec_for_settings(term_settings);
        terminal_viewport::grid_from_window_size_with_spec(window_size, &spec)
    };
    let prev_rows = pane.terminal.grid_size().1;
    if let Some(session) = state.session_manager.session_mut(state.active_tab) {
        let _ = pane.terminal.resize_and_sync_pty(session, cols, rows);
    } else {
        pane.terminal.resize(cols, rows);
    }
    if prev_rows != rows {
        pane.styled_row_cache.clear();
    }
}

pub(crate) fn disconnect_active_tab_session(state: &mut IcedState) {
    state
        .session_manager
        .detach_session(state.active_tab);
    state.active_pane_mut().terminal.clear_pty_resize_anchor();
    state.model.status = "Disconnected".to_string();
    state.last_activity_ms = crate::settings::unix_time_ms();
}

/// Install session on the **active** tab, PTY resize, then DEC 1004 focus once.
pub(crate) fn complete_new_ssh_session(
    state: &mut IcedState,
    session: Box<dyn AsyncSession>,
    recent: crate::settings::RecentConnectionRecord,
    tab_title: String,
    profile_id: Option<String>,
) {
    // Attach session to the active tab (replaces any existing session on that tab).
    state
        .session_manager
        .attach_session(state.active_tab, session);

    // Resize PTY to current window size (active tab already has the new session attached).
    sync_terminal_grid_to_session(state);

    state.active_pane_mut().last_terminal_focus_sent = None;
    TerminalHost::sync_focus_report(state);
    state.model.status = "Connected".to_string();
    state.model.record_recent_connection(recent);
    state.quick_connect_open = false;
    state.settings_modal_open = false;
    state.quick_connect_panel = QuickConnectPanel::Picker;
    if let Some(tab) = state.tabs.get_mut(state.active_tab) {
        tab.title = tab_title;
        tab.profile_id = profile_id;
    }
}

/// 与内置 [`scrollable`] 类似：Lines 使用 x60 缩放；垂直分量映射为横向滚动，无需 Shift。
fn tab_strip_wheel_to_offset_x(delta: ScrollDelta) -> f32 {
    match delta {
        ScrollDelta::Lines { x, y } => -(x + y) * 60.0,
        ScrollDelta::Pixels { x, y } => -(x + y),
    }
}

pub(crate) fn update(state: &mut IcedState, message: Message) -> Task<Message> {
    match message {
        // --- Tick ---
        Message::Tick => tick::handle_tick(state),

        // --- Window ---
        Message::WindowResized(size) => {
            state.window_size = size;
            let term_settings = state.model.settings.terminal.clone();
            let window_size = state.window_size;
            for pane in &mut state.tab_panes {
                apply_terminal_grid_resize_for_pane(pane, window_size, &term_settings);
            }
            Task::none()
        }
        Message::ClipboardPaste(text) => {
            let Some(t) = text else {
                return Task::none();
            };
            TerminalHost::handle_event(state, TerminalEvent::Paste(t))
        }
        Message::ClipboardWriteDone => Task::none(),
        Message::EventOccurred(event) => {
            if let Event::Window(win) = &event {
                match win {
                    iced::window::Event::Opened { size, .. } => {
                        state.window_size = *size;
                        sync_terminal_grid_to_session(state);
                    }
                    iced::window::Event::Focused => {
                        state.window_focused = true;
                        return TerminalHost::handle_event(
                            state,
                            TerminalEvent::FocusChanged(true),
                        );
                    }
                    iced::window::Event::Unfocused => {
                        state.window_focused = false;
                        return TerminalHost::handle_event(
                            state,
                            TerminalEvent::FocusChanged(false),
                        );
                    }
                    _ => {}
                }
            }
            if let Event::Keyboard(key_event) = &event {
                // Ctrl+Shift+D: toggle debug overlay.
                if let keyboard::Event::KeyPressed { key, modifiers, .. } = key_event {
                    if modifiers.contains(keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT) {
                        if let keyboard::Key::Character(s) = key {
                            if s.eq_ignore_ascii_case(&"d") {
                                return update(state, Message::ToggleDebugOverlay);
                            }
                        }
                    }
                }
                if let Some(tev) = TerminalEvent::from_keyboard_event(key_event) {
                    return TerminalHost::handle_event(state, tev);
                }
            }
            if let Event::Mouse(mouse_event) = &event {
                match *mouse_event {
                    mouse::Event::CursorMoved { position } => {
                        return TerminalHost::handle_event(
                            state,
                            TerminalEvent::MouseMoved(position),
                        );
                    }
                    mouse::Event::WheelScrolled { delta } => {
                        if let Some(p) = state.last_cursor_pos {
                            return TerminalHost::handle_event(
                                state,
                                TerminalEvent::MouseWheel { delta, at: p },
                            );
                        }
                    }
                    mouse::Event::ButtonPressed(mouse::Button::Left) => {
                        if let Some(p) = state.last_cursor_pos {
                            return TerminalHost::handle_event(
                                state,
                                TerminalEvent::MouseLeftDown(p),
                            );
                        }
                    }
                    mouse::Event::ButtonPressed(mouse::Button::Right) => {
                        if let Some(p) = state.last_cursor_pos {
                            return TerminalHost::handle_event(
                                state,
                                TerminalEvent::MouseRightClick(p),
                            );
                        }
                    }
                    mouse::Event::ButtonReleased(mouse::Button::Left) => {
                        if let Some(p) = state.last_cursor_pos {
                            return TerminalHost::handle_event(
                                state,
                                TerminalEvent::MouseLeftUp(p),
                            );
                        }
                    }
                    _ => {}
                }
            }
            Task::none()
        }

        // --- Top actions ---
        Message::TopAddTab => session::handle_add_tab(state),
        Message::TopQuickConnect => {
            state.settings_modal_open = false;
            state.quick_connect_open = true;
            state.quick_connect_panel = QuickConnectPanel::Picker;
            state.quick_connect_query.clear();
            state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            state.quick_connect_error_kind = None;
            state.quick_connect_interactive = None;
            state.host_key_prompt = None;
            state.connection_stage = super::state::ConnectionStage::None;
            state.model.status = "Quick connect".to_string();
            Task::none()
        }
        Message::TopOpenSettings => {
            state.quick_connect_open = false;
            state.quick_connect_panel = QuickConnectPanel::Picker;
            state.settings_modal_open = true;
            Task::none()
        }

        // --- Quick connect panels ---
        Message::QuickConnectDismiss => {
            state.quick_connect_open = false;
            state.quick_connect_panel = QuickConnectPanel::Picker;
            state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            state.quick_connect_error_kind = None;
            state.quick_connect_interactive = None;
            state.host_key_prompt = None;
            state.connection_stage = super::state::ConnectionStage::None;
            Task::none()
        }
        Message::QuickConnectNewConnection => {
            state.model.selected_session_id = None;
            state.quick_connect_panel = QuickConnectPanel::NewConnection;
            state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            state.quick_connect_error_kind = None;
            state.quick_connect_interactive = None;
            state.host_key_prompt = None;
            state.connection_stage = super::state::ConnectionStage::None;
            Task::none()
        }
        Message::QuickConnectBackToList => {
            state.quick_connect_panel = QuickConnectPanel::Picker;
            state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            state.quick_connect_error_kind = None;
            state.quick_connect_interactive = None;
            state.host_key_prompt = None;
            state.connection_stage = super::state::ConnectionStage::None;
            Task::none()
        }
        Message::QuickConnectQueryChanged(q) => {
            state.quick_connect_query = q;
            Task::none()
        }
        Message::QuickConnectDirectSubmit => {
            let parts = crate::connection_input::parse_direct_input(&state.quick_connect_query);
            let Some(p) = parts else {
                return Task::none();
            };
            state.model.selected_session_id = None;
            state.model.draft.source = crate::app_model::DraftSource::DirectInput;
            state.model.draft.profile_id = None;
            state.model.draft.recent = None;
            state.model.draft.edited = false;
            state.model.draft.last_error = None;
            state.model.draft.password_error_count = 0;
            state.model.draft.host = p.host;
            state.model.draft.port = p.port.unwrap_or(22).to_string();
            state.model.draft.user = p.user.unwrap_or_default();
            if state.model.draft.user.trim().is_empty() {
                state.quick_connect_flow = super::state::QuickConnectFlow::NeedUser;
            } else {
                state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            }
            state.quick_connect_error_kind = None;
            state.connection_stage = super::state::ConnectionStage::None;
            state.quick_connect_panel = QuickConnectPanel::NewConnection;
            Task::none()
        }
        Message::QuickConnectPickRecent(rec) => {
            if let Some(pid) = rec.profile_id.clone() {
                if let Some(p) = state.model.profiles().iter().find(|s| s.id == pid).cloned() {
                    return connection::handle_profile_connect(state, p);
                }
            }
            state.model.draft.host = rec.host.clone();
            state.model.draft.port = rec.port.to_string();
            state.model.draft.user = rec.user.clone();
            state.model.draft.password = secrecy::SecretString::from(String::new());
            state.model.draft.source = crate::app_model::DraftSource::Recent;
            state.model.draft.profile_id = rec.profile_id.clone();
            state.model.draft.recent = Some(rec.clone());
            state.model.draft.edited = false;
            state.model.draft.last_error = None;
            state.model.draft.password_error_count = 0;
            state.model.selected_session_id = rec.profile_id.clone();
            state.quick_connect_panel = QuickConnectPanel::NewConnection;
            state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            state.quick_connect_error_kind = None;
            state.quick_connect_interactive = None;
            state.connection_stage = super::state::ConnectionStage::None;
            Task::none()
        }

        // --- Settings ---
        Message::SettingsDismiss => settings::handle_settings_dismiss(state),
        Message::SettingsCategoryChanged(cat) => settings::handle_settings_category(state, cat),
        Message::SettingsSubTabChanged(sub) => settings::handle_settings_sub_tab(state, sub),
        Message::SettingsFieldChanged(field) => settings::handle_settings_field(state, field),
        Message::BiometricsToggle(desired) => settings::handle_biometrics_toggle(state, desired),
        Message::SettingsRestartAcknowledged => settings::handle_settings_restart_ack(state),
        Message::SaveSettings => settings::handle_save_settings(state),

        // --- Session management ---
        Message::DeleteSessionProfile(id) => session::handle_delete_session(state, id),
        Message::OpenSessionEditor(profile_id) => {
            session::handle_open_session_editor(state, profile_id)
        }
        Message::SessionEditorClose => session::handle_session_editor_close(state),
        Message::SessionEditorHostChanged(v) => session::handle_session_editor_host(state, v),
        Message::SessionEditorPortChanged(v) => session::handle_session_editor_port(state, v),
        Message::SessionEditorUserChanged(v) => session::handle_session_editor_user(state, v),
        Message::SessionEditorAuthChanged(v) => session::handle_session_editor_auth(state, v),
        Message::SessionEditorPasswordChanged(v) => {
            session::handle_session_editor_password(state, v)
        }
        Message::SessionEditorClearPasswordToggled(v) => {
            session::handle_session_editor_clear_password(state, v)
        }
        Message::SessionEditorSave => session::handle_session_editor_save(state),

        // --- Tabs ---
        Message::TabChipHover(ix) => session::handle_tab_chip_hover(state, ix),
        Message::TabStripWheel(delta) => {
            let dx = tab_strip_wheel_to_offset_x(delta);
            if dx.abs() < f32::EPSILON {
                return Task::none();
            }
            scroll_by(
                Id::new(TAB_STRIP_SCROLLABLE_ID),
                AbsoluteOffset { x: dx, y: 0.0 },
            )
        }
        Message::TabSelected(i) => session::handle_tab_selected(state, i),
        Message::TabClose(i) => session::handle_tab_close(state, i),

        // --- Window controls ---
        #[cfg(not(target_os = "macos"))]
        Message::WinClose => iced::window::latest().then(|opt| {
            if let Some(id) = opt {
                iced::window::close::<Message>(id)
            } else {
                Task::none()
            }
        }),
        #[cfg(not(target_os = "macos"))]
        Message::WinMinimize => iced::window::latest().then(|opt| {
            if let Some(id) = opt {
                iced::window::minimize::<Message>(id, true)
            } else {
                Task::none()
            }
        }),
        #[cfg(not(target_os = "macos"))]
        Message::WinToggleMaximize => iced::window::latest().then(|opt| {
            if let Some(id) = opt {
                iced::window::toggle_maximize::<Message>(id)
            } else {
                Task::none()
            }
        }),

        // --- Connection form ---
        Message::HostChanged(v) => {
            state.model.draft.host = v;
            state.model.draft.edited = true;
            state.model.draft.last_error = None;
            state.model.draft.password_error_count = 0;
            state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            state.quick_connect_error_kind = None;
            state.quick_connect_interactive = None;
            state.host_key_prompt = None;
            Task::none()
        }
        Message::PortChanged(v) => {
            state.model.draft.port = v;
            state.model.draft.edited = true;
            state.model.draft.last_error = None;
            state.model.draft.password_error_count = 0;
            state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            state.quick_connect_error_kind = None;
            state.quick_connect_interactive = None;
            state.host_key_prompt = None;
            Task::none()
        }
        Message::UserChanged(v) => {
            state.model.draft.user = v;
            state.model.draft.edited = true;
            state.model.draft.last_error = None;
            state.model.draft.password_error_count = 0;
            state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            state.quick_connect_error_kind = None;
            state.quick_connect_interactive = None;
            state.host_key_prompt = None;
            Task::none()
        }
        Message::PasswordChanged(v) => {
            state.model.draft.password = secrecy::SecretString::from(v);
            state.model.draft.edited = true;
            state.model.draft.last_error = None;
            state.model.draft.password_error_count = 0;
            if matches!(
                state.quick_connect_flow,
                super::state::QuickConnectFlow::NeedAuthPassword
            ) {
                state.quick_connect_error_kind = None;
            } else {
                state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
                state.quick_connect_error_kind = None;
            }
            state.quick_connect_interactive = None;
            Task::none()
        }
        Message::QuickConnectAuthChanged(v) => {
            state.model.draft.auth = v;
            state.model.draft.edited = true;
            state.model.draft.last_error = None;
            state.model.draft.password_error_count = 0;
            state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            state.quick_connect_error_kind = None;
            state.quick_connect_interactive = None;
            state.host_key_prompt = None;
            Task::none()
        }
        Message::QuickConnectKeyPathChanged(v) => {
            state.model.draft.private_key_path = v;
            state.model.draft.edited = true;
            state.model.draft.last_error = None;
            state.model.draft.password_error_count = 0;
            state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            state.quick_connect_error_kind = None;
            state.quick_connect_interactive = None;
            Task::none()
        }
        Message::QuickConnectPassphraseChanged(v) => {
            state.model.draft.passphrase = secrecy::SecretString::from(v);
            state.model.draft.edited = true;
            state.model.draft.last_error = None;
            state.model.draft.password_error_count = 0;
            state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            state.quick_connect_error_kind = None;
            state.quick_connect_interactive = None;
            Task::none()
        }

        // --- Connection ---
        Message::ConnectPressed => connection::handle_connect(state),
        Message::HostKeyAcceptOnce => connection::handle_host_key_accept_once(state),
        Message::HostKeyAlwaysTrust => connection::handle_host_key_always_trust(state),
        Message::HostKeyReject => connection::handle_host_key_reject(state),
        Message::QuickConnectInteractiveAnswerChanged(i, v) => {
            if let Some(flow) = state.quick_connect_interactive.as_mut() {
                if i < flow.ui.answers.len() {
                    flow.ui.answers[i] = v;
                    flow.ui.error = None;
                }
            }
            Task::none()
        }
        Message::QuickConnectInteractiveSubmit => connection::handle_interactive_submit(state),
        Message::AutoProbeConsentOpen => {
            state.auto_probe_consent_modal = Some(super::state::AutoProbeConsentModalState {});
            Task::none()
        }
        Message::AutoProbeConsentAllowOnce => {
            state.auto_probe_consent_modal = None;
            update(state, Message::ConnectPressed)
        }
        Message::AutoProbeConsentAlwaysAllow => {
            state.model.settings.security.auto_probe_consent =
                crate::settings::AutoProbeConsent::AlwaysAllow;
            state.model.settings.save_with_log();
            state.auto_probe_consent_modal = None;
            update(state, Message::ConnectPressed)
        }
        Message::AutoProbeConsentUsePassword => {
            state.model.draft.auth = crate::session::AuthMethod::Password;
            state.auto_probe_consent_modal = None;
            update(state, Message::ConnectPressed)
        }
        Message::DisconnectPressed => connection::handle_disconnect(state),
        Message::ProfileConnect(profile) => connection::handle_profile_connect(state, profile),

        // --- Vault ---
        Message::VaultOpen => vault::handle_vault_open(state),
        Message::VaultClose => vault::handle_vault_close(state),
        Message::VaultOldPasswordChanged(v) => vault::handle_vault_old_password(state, v),
        Message::VaultNewPasswordChanged(v) => vault::handle_vault_new_password(state, v),
        Message::VaultConfirmPasswordChanged(v) => vault::handle_vault_confirm_password(state, v),
        Message::VaultSubmit => vault::handle_vault_submit(state),
        Message::VaultUnlockOpenConnect(profile) => {
            vault::handle_vault_unlock_open_connect(state, profile)
        }
        Message::VaultUnlockOpenDelete(id) => vault::handle_vault_unlock_open_delete(state, id),
        Message::VaultUnlockOpenSaveSession => vault::handle_vault_unlock_open_save_session(state),
        Message::VaultUnlockClose => vault::handle_vault_unlock_close(state),
        Message::VaultUnlockPasswordChanged(v) => vault::handle_vault_unlock_password(state, v),
        Message::VaultUnlockSubmit => vault::handle_vault_unlock_submit(state),
        Message::ToggleDebugOverlay => {
            state.perf.debug_overlay_enabled = !state.perf.debug_overlay_enabled;
            log::info!(
                target: "term-perf",
                "debug_overlay={}",
                state.perf.debug_overlay_enabled
            );
            Task::none()
        }
    }
}
