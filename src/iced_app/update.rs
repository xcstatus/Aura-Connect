use iced::event::Event;
use iced::mouse;
use iced::mouse::ScrollDelta;
use iced::widget::operation::{scroll_by, AbsoluteOffset};
use iced::widget::Id;
use iced::Task;
use iced::Size;

use secrecy::ExposeSecret;

use crate::backend::ssh_session::AsyncSession;
use crate::terminal_core::TerminalController;

use super::message::{Message, SettingsField};
use super::settings_modal::clamp_sub_tab;
use super::state::{
    IcedState, IcedTab, QuickConnectPanel, TabPane, SessionEditorState, VaultFlowMode, VaultFlowState,
    VaultStatus,
};
use super::chrome::TAB_STRIP_SCROLLABLE_ID;
use super::terminal_event::TerminalEvent;
use super::terminal_host::TerminalHost;
use super::terminal_viewport;

/// Resize PTY/grid from window size and emit `term_viewport` log when geometry changes.
fn apply_terminal_grid_resize(
    window: Size,
    terminal: &mut TerminalController,
    session: &mut dyn AsyncSession,
    terminal_settings: &crate::settings::TerminalSettings,
) {
    let spec = terminal_viewport::terminal_viewport_spec_for_settings(terminal_settings);
    let (cols, rows) = terminal_viewport::grid_from_window_size_with_spec(window, &spec);
    let pty_sent = match terminal.resize_and_sync_pty(session, cols, rows) {
        Ok(b) => b,
        Err(e) => {
            log::warn!(target: "term_viewport", "resize_and_sync_pty failed: {e}");
            false
        }
    };
    terminal_viewport::log_viewport_geometry_if_changed(window, &spec, cols, rows, pty_sent);
}

fn apply_terminal_grid_resize_for_pane(
    window: Size,
    pane: &mut TabPane,
    terminal_settings: &crate::settings::TerminalSettings,
) {
    let spec = terminal_viewport::terminal_viewport_spec_for_settings(terminal_settings);
    let (cols, rows) = terminal_viewport::grid_from_window_size_with_spec(window, &spec);
    if let Some(session) = pane.session.as_mut() {
        let _ = pane
            .terminal
            .resize_and_sync_pty(session.as_mut(), cols, rows);
    } else {
        pane.terminal.resize(cols, rows);
    }
}

fn sync_terminal_grid_to_session(state: &mut IcedState) {
    let term_settings = state.model.settings.terminal.clone();
    let window_size = state.window_size;
    let pane = state.active_pane_mut();
    let Some(session) = pane.session.as_mut() else {
        return;
    };
    apply_terminal_grid_resize(
        window_size,
        &mut pane.terminal,
        session.as_mut(),
        &term_settings,
    );
}

fn disconnect_active_tab_session(state: &mut IcedState) {
    let pane = state.active_pane_mut();
    pane.session = None;
    pane.terminal.clear_pty_resize_anchor();
    state.model.status = "Disconnected".to_string();
    state.last_activity_ms = crate::settings::unix_time_ms();
}

/// If `single_shared_session`, disconnect sessions on all tabs except `keep_tab` (if any).
fn enforce_single_session_policy(state: &mut IcedState, keep_tab: Option<usize>) {
    if !state.model.settings.quick_connect.single_shared_session {
        return;
    }
    for (i, p) in state.tab_panes.iter_mut().enumerate() {
        if keep_tab == Some(i) {
            continue;
        }
        p.session = None;
        p.terminal.clear_pty_resize_anchor();
    }
}

/// Install session on the **active** tab, PTY resize, then DEC 1004 focus once.
///
/// **Order:** the tab's `session` must be set before [`sync_terminal_focus_report`], which only
/// writes to the current session — otherwise the first CSI I/O for a new PTY would be dropped.
fn complete_new_ssh_session(
    state: &mut IcedState,
    session: Box<dyn AsyncSession>,
    recent: crate::settings::RecentConnectionRecord,
    tab_title: String,
    profile_id: Option<String>,
) {
    let term_settings = state.model.settings.terminal.clone();
    if state.model.settings.quick_connect.single_shared_session {
        enforce_single_session_policy(state, None);
    } else {
        let pane = state.active_pane_mut();
        pane.session = None;
        pane.terminal.clear_pty_resize_anchor();
    }
    {
        let window_size = state.window_size;
        let pane = state.active_pane_mut();
        pane.session = Some(session);
        let Some(sess) = pane.session.as_mut() else {
            return;
        };
        apply_terminal_grid_resize(
            window_size,
            &mut pane.terminal,
            sess.as_mut(),
            &term_settings,
        );
    }
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

pub(crate) fn update(state: &mut IcedState, message: Message) -> Task<Message> {
    match message {
        Message::Tick => {
            let now = crate::settings::unix_time_ms();
            state.tick_count = state.tick_count.wrapping_add(1);
            state.perf.ticks += 1;

            let bg_pump_every_ms: i64 = if state.window_focused { 200 } else { 250 };

            if state.model.settings.quick_connect.single_shared_session {
                // Single-session: only active tab should ever have a live session.
                let active = state.active_tab;
                let mut rebuilds = 0u64;
                let mut bytes_in = 0u64;
                let mut pumped = false;
                if let Some(pane) = state.tab_panes.get_mut(active) {
                    if let Some(session) = pane.session.as_mut() {
                        pumped = true;
                        if let Ok(n) = pane.terminal.pump_output(session.as_mut()) {
                            if n > 0 {
                                bytes_in = n as u64;
                            }
                        }
                        rebuilds = pane.terminal.take_plain_join_rebuilds();
                    }
                }
                if pumped {
                    state.perf.pump_calls += 1;
                }
                if bytes_in > 0 {
                    state.last_activity_ms = now;
                    state.perf.bytes_in += bytes_in;
                }
                state.perf.rebuilds += rebuilds;
            } else {
                // Multi-session: active tab high frequency, background tabs throttled.
                let active = state.active_tab;
                for (i, p) in state.tab_panes.iter_mut().enumerate() {
                    let Some(session) = p.session.as_mut() else {
                        continue;
                    };
                    let should_pump = if i == active {
                        true
                    } else {
                        now.saturating_sub(p.last_pump_ms) >= bg_pump_every_ms
                    };
                    if !should_pump {
                        continue;
                    }
                    p.last_pump_ms = now;
                    state.perf.pump_calls += 1;
                    if let Ok(n) = p.terminal.pump_output(session.as_mut()) {
                        if n > 0 {
                            state.last_activity_ms = now;
                            state.perf.bytes_in += n as u64;
                        }
                    }
                    state.perf.rebuilds += p.terminal.take_plain_join_rebuilds();
                }
            }

            // Cursor blink / styled cursor refresh: 500ms sampling, only when focused.
            let blink_due = now.saturating_sub(state.last_blink_tick_ms) >= 500;
            if blink_due && state.window_focused {
                for (i, p) in state.tab_panes.iter_mut().enumerate() {
                    if state.model.settings.quick_connect.single_shared_session && i != state.active_tab {
                        continue;
                    }
                    p.terminal.on_frame_tick();
                }
                state.last_blink_tick_ms = now;
            }

            // Aggregated perf log (every ~8s).
            if now.saturating_sub(state.perf.last_log_ms) >= 8_000 {
                let dt = (now - state.perf.last_log_ms).max(1) as f64 / 1000.0;
                let ticks = state.perf.ticks - state.perf.ticks_at_log;
                let pumps = state.perf.pump_calls - state.perf.pump_calls_at_log;
                let bytes = state.perf.bytes_in - state.perf.bytes_in_at_log;
                let rebuilds = state.perf.rebuilds - state.perf.rebuilds_at_log;
                let mut key_fb_named = 0u64;
                let mut key_fb_text = 0u64;
                for (i, p) in state.tab_panes.iter_mut().enumerate() {
                    if state.model.settings.quick_connect.single_shared_session && i != state.active_tab {
                        continue;
                    }
                    let (n_named, n_text) = p.terminal.take_key_fallback_counts();
                    key_fb_named = key_fb_named.saturating_add(n_named);
                    key_fb_text = key_fb_text.saturating_add(n_text);
                }
                if let Some(path) = state.perf.dump_path.as_deref() {
                    if !state.perf.dump_header_written {
                        let _ = std::fs::create_dir_all(
                            std::path::Path::new(path)
                                .parent()
                                .unwrap_or_else(|| std::path::Path::new(".")),
                        );
                        let header = "ts_ms,tick_rate_per_s,pump_calls_per_s,bytes_in_per_s,rebuilds_per_s,key_fb_named_per_s,key_fb_text_per_s,focused,shared_session\n";
                        let _ = std::fs::write(path, header);
                        state.perf.dump_header_written = true;
                    }
                    let line = format!(
                        "{},{:.3},{:.3},{},{},{:.3},{:.3},{},{}\n",
                        now,
                        (ticks as f64) / dt,
                        (pumps as f64) / dt,
                        (bytes as f64 / dt) as u64,
                        (rebuilds as f64 / dt) as u64,
                        (key_fb_named as f64) / dt,
                        (key_fb_text as f64) / dt,
                        state.window_focused,
                        state.model.settings.quick_connect.single_shared_session
                    );
                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                        use std::io::Write;
                        let _ = f.write_all(line.as_bytes());
                    }
                }
                log::debug!(
                    target: "term-prof",
                    "perf tick_rate={:.1}/s pump_calls={:.1}/s bytes_in={}/s rebuilds={}/s key_fb_named={:.2}/s key_fb_text={:.2}/s focused={} shared_session={}",
                    (ticks as f64) / dt,
                    (pumps as f64) / dt,
                    (bytes as f64 / dt) as u64,
                    (rebuilds as f64 / dt) as u64,
                    (key_fb_named as f64) / dt,
                    (key_fb_text as f64) / dt,
                    state.window_focused,
                    state.model.settings.quick_connect.single_shared_session
                );
                state.perf.last_log_ms = now;
                state.perf.ticks_at_log = state.perf.ticks;
                state.perf.pump_calls_at_log = state.perf.pump_calls;
                state.perf.bytes_in_at_log = state.perf.bytes_in;
                state.perf.rebuilds_at_log = state.perf.rebuilds;
            }
            Task::none()
        }
        Message::WindowResized(size) => {
            state.window_size = size;
            if state.model.settings.quick_connect.single_shared_session {
                sync_terminal_grid_to_session(state);
            } else {
                // Multi-session: keep all live PTYs in sync with the viewport.
                let term_settings = state.model.settings.terminal.clone();
                let window_size = state.window_size;
                for p in &mut state.tab_panes {
                    let Some(session) = p.session.as_mut() else {
                        continue;
                    };
                    apply_terminal_grid_resize(
                        window_size,
                        &mut p.terminal,
                        session.as_mut(),
                        &term_settings,
                    );
                }
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
                        return TerminalHost::handle_event(state, TerminalEvent::FocusChanged(true));
                    }
                    iced::window::Event::Unfocused => {
                        state.window_focused = false;
                        return TerminalHost::handle_event(state, TerminalEvent::FocusChanged(false));
                    }
                    _ => {}
                }
            }
            if let Event::Keyboard(key_event) = &event {
                if let Some(tev) = TerminalEvent::from_keyboard_event(key_event) {
                    // Forward even if capture is off: handler owns gating and shortcuts.
                    return TerminalHost::handle_event(state, tev);
                }
            }
            if let Event::Mouse(mouse_event) = &event {
                match *mouse_event {
                    mouse::Event::CursorMoved { position } => {
                        return TerminalHost::handle_event(state, TerminalEvent::MouseMoved(position));
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
                            return TerminalHost::handle_event(state, TerminalEvent::MouseLeftDown(p));
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
                            return TerminalHost::handle_event(state, TerminalEvent::MouseLeftUp(p));
                        }
                    }
                    _ => {}
                }
            }
            Task::none()
        }
        Message::TopAddTab => {
            state.quick_connect_open = false;
            state.settings_modal_open = false;
            let title = state.model.i18n.tr("iced.tab.new").to_string();
            state.tabs.push(IcedTab {
                title,
                profile_id: None,
            });
            state
                .tab_panes
                .push(TabPane::new(&state.model.settings.terminal));
            state.active_tab = state.tabs.len() - 1;
            state.model.status = "New tab: use Quick connect (⚡) to open a session".to_string();
            Task::none()
        }
        Message::TopQuickConnect => {
            state.settings_modal_open = false;
            state.quick_connect_open = true;
            state.quick_connect_panel = QuickConnectPanel::Picker;
            state.quick_connect_query.clear();
            state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            state.quick_connect_error_kind = None;
            state.quick_connect_interactive = None;
            state.host_key_prompt = None;
            state.model.status = "Quick connect".to_string();
            Task::none()
        }
        Message::QuickConnectDismiss => {
            state.quick_connect_open = false;
            state.quick_connect_panel = QuickConnectPanel::Picker;
            state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            state.quick_connect_error_kind = None;
            state.quick_connect_interactive = None;
            state.host_key_prompt = None;
            Task::none()
        }
        Message::QuickConnectNewConnection => {
            state.model.selected_session_id = None;
            state.quick_connect_panel = QuickConnectPanel::NewConnection;
            state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            state.quick_connect_error_kind = None;
            state.quick_connect_interactive = None;
            state.host_key_prompt = None;
            Task::none()
        }
        Message::QuickConnectBackToList => {
            state.quick_connect_panel = QuickConnectPanel::Picker;
            state.quick_connect_flow = super::state::QuickConnectFlow::Idle;
            state.quick_connect_error_kind = None;
            state.quick_connect_interactive = None;
            state.host_key_prompt = None;
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
            state.quick_connect_panel = QuickConnectPanel::NewConnection;
            Task::none()
        }
        Message::QuickConnectPickRecent(rec) => {
            if let Some(pid) = rec.profile_id.clone() {
                if let Some(p) = state
                    .model
                    .profiles()
                    .iter()
                    .find(|s| s.id == pid)
                    .cloned()
                {
                    return update(state, Message::ProfileConnect(p));
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
            Task::none()
        }
        Message::TopOpenSettings => {
            state.quick_connect_open = false;
            state.quick_connect_panel = QuickConnectPanel::Picker;
            state.settings_modal_open = true;
            Task::none()
        }
        Message::SettingsDismiss => {
            state.settings_modal_open = false;
            Task::none()
        }
        Message::SettingsCategoryChanged(cat) => {
            state.settings_category = cat;
            let i = cat as usize;
            state.settings_sub_tab[i] = clamp_sub_tab(cat, state.settings_sub_tab[i]);
            Task::none()
        }
        Message::SettingsSubTabChanged(sub) => {
            let cat = state.settings_category;
            let i = cat as usize;
            state.settings_sub_tab[i] = clamp_sub_tab(cat, sub);
            Task::none()
        }
        Message::SettingsFieldChanged(field) => {
            apply_settings_field(state, field);
            Task::none()
        }
        Message::BiometricsToggle(desired) => {
            if desired {
                let reason = state
                    .model
                    .i18n
                    .tr("settings.security.biometrics.reason.toggle");
                match crate::security::biometric_auth::authenticate_user_presence(reason) {
                    Ok(()) => {
                        state.model.settings.security.use_biometrics = true;
                        let _ = state.model.settings.save();
                        state.model.status = state
                            .model
                            .i18n
                            .tr("toast.biometrics.updated")
                            .to_string();
                    }
                    Err(e) => {
                        state.model.status = state.model.i18n.tr(e.i18n_key()).to_string();
                    }
                }
            } else {
                state.model.settings.security.use_biometrics = false;
                let _ = state.model.settings.save();
            }
            Task::none()
        }
        Message::SettingsRestartAcknowledged => {
            state.settings_needs_restart = false;
            Task::none()
        }
        Message::DeleteSessionProfile(id) => {
            // Cleanup for SSH credential stored in vault (requires runtime unlock when vault is initialized).
            let mut vault_cleanup_err: Option<String> = None;
            if let Some(p) = state.model.profiles().iter().find(|p| p.id == id) {
                if let crate::session::TransportConfig::Ssh(ssh) = &p.transport {
                    if let Some(cid) = ssh.credential_id.as_deref() {
                        if state.model.settings.security.vault.is_some()
                            && state.model.vault_master_password.is_none()
                        {
                            return update(
                                state,
                                Message::VaultUnlockOpenDelete(id),
                            );
                        }
                        if let Some(master) = state.model.vault_master_password.as_ref() {
                            if let Err(e) = crate::vault::session_credentials::delete_credential_with_master(
                                &state.model.settings,
                                master,
                                cid,
                            ) {
                                vault_cleanup_err = Some(format!("Vault 凭据清理失败（{e}）"));
                            }
                        }
                    }
                }
            }
            state.vault_status = VaultStatus::compute(
                &state.model.settings,
                state.model.vault_master_password.is_some(),
            );
            let res = state
                .rt
                .block_on(state.model.session_manager.delete_session(&id));
            state.model.status = match res {
                Ok(()) => vault_cleanup_err.unwrap_or_else(|| {
                    state.model.i18n.tr("iced.settings.conn.deleted").to_string()
                }),
                Err(e) => {
                    let base = format!("Delete failed: {e}");
                    if let Some(warn) = vault_cleanup_err {
                        format!("{base}. {warn}")
                    } else {
                        base
                    }
                }
            };
            Task::none()
        }
        Message::VaultOpen => {
            let mode = if state.model.settings.security.vault.is_some() {
                VaultFlowMode::ChangePassword
            } else {
                VaultFlowMode::Initialize
            };
            state.vault_flow = Some(VaultFlowState::new(mode));
            Task::none()
        }
        Message::VaultClose => {
            state.vault_flow = None;
            Task::none()
        }
        Message::VaultOldPasswordChanged(v) => {
            if let Some(flow) = state.vault_flow.as_mut() {
                flow.old_password = secrecy::SecretString::from(v);
                flow.error = None;
            }
            Task::none()
        }
        Message::VaultNewPasswordChanged(v) => {
            if let Some(flow) = state.vault_flow.as_mut() {
                flow.new_password = secrecy::SecretString::from(v);
                flow.error = None;
            }
            Task::none()
        }
        Message::VaultConfirmPasswordChanged(v) => {
            if let Some(flow) = state.vault_flow.as_mut() {
                flow.confirm_password = secrecy::SecretString::from(v);
                flow.error = None;
            }
            Task::none()
        }
        Message::VaultSubmit => {
            let Some(flow) = state.vault_flow.as_mut() else {
                return Task::none();
            };

            let new_pwd = flow.new_password.expose_secret().trim().to_string();
            let confirm = flow.confirm_password.expose_secret().trim().to_string();
            if new_pwd.is_empty() || new_pwd != confirm {
                flow.error = Some("两次输入的新密码不一致".to_string());
                return Task::none();
            }

            if let VaultFlowMode::ChangePassword = flow.mode {
                let Some(old_meta) = state.model.settings.security.vault.as_ref() else {
                    flow.error = Some("Vault 未初始化".to_string());
                    return Task::none();
                };
                let old_pwd = secrecy::SecretString::from(flow.old_password.expose_secret().to_string());
                if !crate::vault::VaultManager::verify_password(&old_pwd, old_meta) {
                    flow.error = Some("旧密码校验失败".to_string());
                    return Task::none();
                }
            }

            let new_password = secrecy::SecretString::from(new_pwd);
            let new_meta = match crate::vault::VaultManager::setup_vault(&new_password) {
                Ok(m) => m,
                Err(e) => {
                    flow.error = Some(format!("Vault 初始化失败：{e}"));
                    return Task::none();
                }
            };

            let Some(vault_path) = crate::storage::StorageManager::get_vault_path() else {
                flow.error = Some("无法定位 vault 路径".to_string());
                return Task::none();
            };

            let new_secret = secrecy::SecretString::from(new_meta.verifier_hash.clone());
            // IMPORTANT: do NOT overwrite settings meta unless vault file operation succeeds.
            let vault_ok = match flow.mode {
                VaultFlowMode::Initialize => crate::vault::core::CredentialVault::initialize(&new_secret)
                    .and_then(|v| v.save_to_file(&vault_path))
                    .is_ok(),
                VaultFlowMode::ChangePassword => {
                    let Some(old_meta) = state.model.settings.security.vault.as_ref() else {
                        flow.error = Some("Vault 未初始化".to_string());
                        return Task::none();
                    };
                    let old_secret = secrecy::SecretString::from(old_meta.verifier_hash.clone());
                    if vault_path.exists() {
                        crate::vault::core::CredentialVault::load_from_file(&vault_path)
                            .and_then(|mut v| {
                                v.unlock(&old_secret)?;
                                v.rekey(&new_secret)?;
                                v.save_to_file(&vault_path)?;
                                Ok(())
                            })
                            .is_ok()
                    } else {
                        crate::vault::core::CredentialVault::initialize(&new_secret)
                            .and_then(|v| v.save_to_file(&vault_path))
                            .is_ok()
                    }
                }
            };

            if !vault_ok {
                flow.error = Some("Vault 文件写入/重加密失败，请检查权限或文件损坏".to_string());
                return Task::none();
            }

            state.model.settings.security.vault = Some(new_meta);
            let _ = state.model.settings.save();
            // Unlock with the new derived secret for this runtime.
            state.model.vault_master_password =
                Some(secrecy::SecretString::from(state.model.settings.security.vault.as_ref().unwrap().verifier_hash.clone()));
            state.vault_status = VaultStatus::compute(
                &state.model.settings,
                state.model.vault_master_password.is_some(),
            );

            state.vault_flow = None;
            state.model.status = "Vault 已更新".to_string();
            Task::none()
        }
        Message::OpenSessionEditor(profile_id) => {
            let mut st = SessionEditorState {
                profile_id: profile_id.clone(),
                host: String::new(),
                port: "22".to_string(),
                user: String::new(),
                auth: crate::session::AuthMethod::Password,
                password: secrecy::SecretString::new("".into()),
                existing_credential_id: None,
                password_dirty: false,
                clear_saved_password: false,
                error: None,
            };
            if let Some(pid) = profile_id {
                if let Some(p) = state.model.profiles().iter().find(|p| p.id == pid) {
                    if let crate::session::TransportConfig::Ssh(ssh) = &p.transport {
                        st.host = ssh.host.clone();
                        st.port = ssh.port.to_string();
                        st.user = ssh.user.clone();
                        st.auth = ssh.auth.clone();
                        st.existing_credential_id = ssh.credential_id.clone();
                    }
                }
            }
            state.session_editor = Some(st);
            Task::none()
        }
        Message::SessionEditorClose => {
            state.session_editor = None;
            Task::none()
        }
        Message::SessionEditorHostChanged(v) => {
            if let Some(ed) = state.session_editor.as_mut() {
                ed.host = v;
                ed.error = None;
            }
            Task::none()
        }
        Message::SessionEditorPortChanged(v) => {
            if let Some(ed) = state.session_editor.as_mut() {
                ed.port = v;
                ed.error = None;
            }
            Task::none()
        }
        Message::SessionEditorUserChanged(v) => {
            if let Some(ed) = state.session_editor.as_mut() {
                ed.user = v;
                ed.error = None;
            }
            Task::none()
        }
        Message::SessionEditorAuthChanged(v) => {
            if let Some(ed) = state.session_editor.as_mut() {
                ed.auth = v;
                ed.error = None;
            }
            Task::none()
        }
        Message::SessionEditorPasswordChanged(v) => {
            if let Some(ed) = state.session_editor.as_mut() {
                ed.password = secrecy::SecretString::from(v);
                ed.password_dirty = true;
                ed.error = None;
            }
            Task::none()
        }
        Message::SessionEditorClearPasswordToggled(v) => {
            if let Some(ed) = state.session_editor.as_mut() {
                ed.clear_saved_password = v;
                ed.error = None;
            }
            Task::none()
        }
        Message::SessionEditorSave => {
            let Some(ed) = state.session_editor.as_mut() else {
                return Task::none();
            };

            let host = ed.host.trim().to_string();
            let user = ed.user.trim().to_string();
            let port_ok = ed
                .port
                .trim()
                .parse::<u32>()
                .ok()
                .is_some_and(|p| p >= 1 && p <= 65535);
            if host.is_empty() {
                ed.error = Some("Host 为必填项".to_string());
                return Task::none();
            }
            if user.is_empty() {
                ed.error = Some("User 为必填项".to_string());
                return Task::none();
            }
            if !port_ok {
                ed.error = Some("端口范围 1–65535".to_string());
                return Task::none();
            }

            let id = ed
                .profile_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let port: u16 = ed.port.trim().parse().unwrap_or(22);
            let auth = ed.auth.clone();

            // Credential lifecycle policy:
            // - If user did NOT touch password and did NOT explicitly clear -> keep existing credential_id unchanged.
            // - If user explicitly clears -> delete credential (requires runtime unlock).
            // - If user edited password and provided non-empty -> write/overwrite credential (requires runtime unlock).
            let mut credential_id = ed.existing_credential_id.clone();
            let needs_vault_op = ed.clear_saved_password
                || (ed.password_dirty
                    && matches!(auth, crate::session::AuthMethod::Password)
                    && !ed.password.expose_secret().trim().is_empty());
            if needs_vault_op {
                if state.model.settings.security.vault.is_some()
                    && state.model.vault_master_password.is_none()
                {
                    return update(state, Message::VaultUnlockOpenSaveSession);
                }
                let Some(master) = state.model.vault_master_password.as_ref() else {
                    ed.error = Some("Vault 未解锁".to_string());
                    return Task::none();
                };
                if ed.clear_saved_password {
                    if let Some(cid) = ed.existing_credential_id.as_deref() {
                        if let Err(e) = crate::vault::session_credentials::delete_credential_with_master(
                            &state.model.settings,
                            master,
                            cid,
                        ) {
                            ed.error = Some(format!("清理凭据失败：{e}"));
                            return Task::none();
                        }
                    }
                    credential_id = None;
                } else {
                    let pw = ed.password.expose_secret().trim();
                    let password_ref = (!pw.is_empty()).then_some(&ed.password);
                    match crate::vault::session_credentials::sync_ssh_credentials_with_master(
                        &state.model.settings,
                        master,
                        &id,
                        password_ref,
                        None,
                    ) {
                        Ok(cid) => credential_id = cid,
                        Err(e) => {
                            ed.error = Some(format!("保存失败：无法写入 Vault（{e}）"));
                            return Task::none();
                        }
                    }
                }
            }
            state.vault_status = VaultStatus::compute(
                &state.model.settings,
                state.model.vault_master_password.is_some(),
            );

            let existing_folder = state
                .model
                .profiles()
                .iter()
                .find(|p| p.id == id)
                .and_then(|p| p.folder.clone());
            let profile = crate::session::SessionProfile {
                id: id.clone(),
                name: format!("{}@{}", user, host),
                folder: existing_folder,
                color_tag: None,
                transport: crate::session::TransportConfig::Ssh(crate::session::SshConfig {
                    host,
                    port,
                    user,
                    auth,
                    credential_id,
                }),
            };

            let res = state
                .rt
                .block_on(state.model.session_manager.upsert_session(profile));
            state.model.status = match res {
                Ok(()) => "Session saved".to_string(),
                Err(e) => format!("Save failed: {e}"),
            };
            state.session_editor = None;
            Task::none()
        }
        Message::TabChipHover(ix) => {
            state.tab_hover_index = ix;
            Task::none()
        }
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
        Message::TabSelected(i) => {
            if i >= state.tabs.len() {
                return Task::none();
            }
            let old = state.active_tab;
            // Explicit DEC 1004 transition when switching sessions.
            if old != i {
                TerminalHost::sync_focus_report_for_tab(state, old, false);
            }
            if state.model.settings.quick_connect.single_shared_session
                && old != i
                && state
                    .tab_panes
                    .get(old)
                    .is_some_and(|p| p.session.is_some())
            {
                state.tab_panes[old].session = None;
                state.tab_panes[old].terminal.clear_pty_resize_anchor();
                state.model.status =
                    "Disconnected previous tab (single-session mode)".to_string();
            }
            state.active_tab = i;
            if let Some(pid) = state.tabs[i].profile_id.clone() {
                state.model.select_profile(pid);
            }
            sync_terminal_grid_to_session(state);
            TerminalHost::sync_focus_report(state);
            Task::none()
        }
        Message::TabClose(i) => {
            if state.tabs.len() <= 1 {
                return Task::none();
            }
            if i >= state.tabs.len() {
                return Task::none();
            }
            let was_active = i == state.active_tab;
            state.tabs.remove(i);
            state.tab_panes.remove(i);
            let len = state.tabs.len();
            if was_active {
                state.active_tab = state.active_tab.min(len.saturating_sub(1));
            } else if i < state.active_tab {
                state.active_tab -= 1;
            }
            if let Some(pid) = state.tabs[state.active_tab].profile_id.clone() {
                state.model.select_profile(pid);
            }
            sync_terminal_grid_to_session(state);
            TerminalHost::sync_focus_report(state);
            Task::none()
        }
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
            // If we were waiting for password, keep that state but clear error.
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
        Message::ConnectPressed => {
            // User required gate (NeedUser).
            if state.model.draft.host.trim().is_empty() || state.model.draft.user.trim().is_empty()
            {
                state.quick_connect_flow = super::state::QuickConnectFlow::NeedUser;
                state.quick_connect_error_kind =
                    Some(crate::app_model::ConnectErrorKind::MissingHostOrUser);
                return Task::none();
            }

            // Password lockout: stop repeated wrong passwords within the same draft context.
            if matches!(state.model.draft.auth, crate::session::AuthMethod::Password)
                && state.model.draft.password_error_count >= 3
            {
                state.quick_connect_flow = super::state::QuickConnectFlow::AuthLocked;
                state.quick_connect_error_kind =
                    Some(crate::app_model::ConnectErrorKind::AuthFailed);
                return Task::none();
            }

            // NeedAuthPassword gate: password auth but no password provided yet.
            if matches!(state.model.draft.auth, crate::session::AuthMethod::Password)
                && state.model.draft.password.expose_secret().trim().is_empty()
            {
                state.quick_connect_flow = super::state::QuickConnectFlow::NeedAuthPassword;
                state.quick_connect_error_kind = None;
                return Task::none();
            }

            // One-time consent gate before automatic auth probing (agent/key).
            let needs_probe = matches!(
                state.model.draft.auth,
                crate::session::AuthMethod::Agent | crate::session::AuthMethod::Key { .. }
            );
            if needs_probe
                && matches!(
                    state.model.settings.security.auto_probe_consent,
                    crate::settings::AutoProbeConsent::Ask
                )
                && state.auto_probe_consent_modal.is_none()
            {
                return update(state, Message::AutoProbeConsentOpen);
            }

            // Interactive: drive as a state machine (NeedAuthInteractive) instead of one-shot connect.
            if matches!(state.model.draft.auth, crate::session::AuthMethod::Interactive) {
                let msg = state.model.i18n.tr("iced.term.connecting");
                state.active_pane_mut().terminal.inject_local_lines(&[msg]);
                state.quick_connect_flow = super::state::QuickConnectFlow::Connecting;
                state.quick_connect_error_kind = None;
                let host = state.model.draft.host.trim().to_string();
                let user = state.model.draft.user.trim().to_string();
                let port: u16 = state.model.draft.port.trim().parse().unwrap_or(22);
                    let known_hosts = state.model.settings.security.known_hosts.clone();
                    let start = state.rt.block_on(async {
                    let sess =
                        crate::backend::ssh_session::InteractiveAuthSession::connect(
                            &host,
                            port,
                            &user,
                            &known_hosts,
                        )
                            .await?;
                    Ok::<_, anyhow::Error>(sess)
                });
                let sess = match start {
                    Ok(s) => s,
                    Err(_e) => {
                        let kind = crate::app_model::ConnectErrorKind::HostUnreachable;
                        state.quick_connect_flow = super::state::QuickConnectFlow::Failed;
                        state.quick_connect_error_kind = Some(kind);
                        return Task::none();
                    }
                };
                let (sess, step) = match state.rt.block_on(sess.start()) {
                    Ok(v) => v,
                    Err(_e) => {
                        state.quick_connect_flow = super::state::QuickConnectFlow::Failed;
                        state.quick_connect_error_kind =
                            Some(crate::app_model::ConnectErrorKind::Unknown);
                        return Task::none();
                    }
                };
                match step {
                    crate::backend::ssh_session::KeyboardInteractiveStep::Success => {
                        let out = state.rt.block_on(sess.finish_into_session());
                        match out {
                            Ok(ssh_sess) => {
                                // Reuse existing "success" path by installing session into tab.
                                // (Strategy A + recent + credential handling are handled below for normal sessions.)
                                let boxed: Box<dyn AsyncSession> = Box::new(ssh_sess);
                                // Fake by wrapping into same success path: treat it like a connected session.
                                // We'll reuse the existing Ok(session) branch logic by using `boxed`.
                                // (To minimize diff, inline the same body as below.)
                                let session: Box<dyn AsyncSession> = boxed;
                                // ---- begin shared success body ----
                                let host = state.model.draft.host.trim().to_string();
                                let user = state.model.draft.user.trim().to_string();
                                let port: u16 = state.model.draft.port.trim().parse().unwrap_or(22);
                                let mut profile_id = state.model.selected_session_id.clone();
                                if profile_id.is_none()
                                    && matches!(
                                        state.model.draft.source,
                                        crate::app_model::DraftSource::DirectInput
                                    )
                                    && !host.is_empty()
                                    && !user.is_empty()
                                {
                                    let existing = state.model.profiles().iter().find(|p| {
                                        let crate::session::TransportConfig::Ssh(ssh) = &p.transport
                                        else {
                                            return false;
                                        };
                                        ssh.host == host && ssh.port == port && ssh.user == user
                                    });
                                    let id = existing
                                        .map(|p| p.id.clone())
                                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                                    let profile = if let Some(ex) = existing.cloned() {
                                        ex
                                    } else {
                                        crate::session::SessionProfile {
                                            id: id.clone(),
                                            name: format!("{user}@{host}"),
                                            folder: None,
                                            color_tag: None,
                                            transport: crate::session::TransportConfig::Ssh(
                                                crate::session::SshConfig {
                                                    host: host.clone(),
                                                    port,
                                                    user: user.clone(),
                                                    auth: state.model.draft.auth.clone(),
                                                    credential_id: None,
                                                },
                                            ),
                                        }
                                    };
                                    let _ = state
                                        .rt
                                        .block_on(state.model.session_manager.upsert_session(profile));
                                    profile_id = Some(id.clone());
                                    state.model.selected_session_id = profile_id.clone();
                                }
                                let label = match &profile_id {
                                    Some(pid) => state
                                        .model
                                        .profiles()
                                        .iter()
                                        .find(|s| s.id == *pid)
                                        .map(|p| p.name.clone())
                                        .unwrap_or_else(|| format!("{user}@{host}")),
                                    None => format!("{user}@{host}"),
                                };
                                let recent = state
                                    .model
                                    .recent_record_for_draft_with_profile(label.clone(), profile_id.clone());
                                complete_new_ssh_session(state, session, recent, label, profile_id);
                                state.quick_connect_flow = super::state::QuickConnectFlow::Connected;
                                state.quick_connect_error_kind = None;
                                // ---- end shared success body ----
                                return Task::none();
                            }
                            Err(_e) => {
                                state.quick_connect_flow = super::state::QuickConnectFlow::Failed;
                                state.quick_connect_error_kind =
                                    Some(crate::app_model::ConnectErrorKind::Unknown);
                                return Task::none();
                            }
                        }
                    }
                    crate::backend::ssh_session::KeyboardInteractiveStep::Failure => {
                        state.quick_connect_flow = super::state::QuickConnectFlow::Failed;
                        state.quick_connect_error_kind =
                            Some(crate::app_model::ConnectErrorKind::AuthFailed);
                        return Task::none();
                    }
                    crate::backend::ssh_session::KeyboardInteractiveStep::InfoRequest(info) => {
                        state.quick_connect_interactive = Some(super::state::InteractiveAuthFlow {
                            session: sess,
                            ui: super::state::InteractivePromptState {
                                name: info.name,
                                instructions: info.instructions,
                                prompts: info.prompts.clone(),
                                answers: vec![String::new(); info.prompts.len()],
                                error: None,
                            },
                        });
                        state.quick_connect_flow = super::state::QuickConnectFlow::NeedAuthInteractive;
                        state.quick_connect_error_kind = None;
                        return Task::none();
                    }
                }
            }

            let msg = state.model.i18n.tr("iced.term.connecting");
            state.active_pane_mut().terminal.inject_local_lines(&[msg]);
            state.quick_connect_flow = super::state::QuickConnectFlow::Connecting;
            state.quick_connect_error_kind = None;
            // Merge persisted known hosts and runtime overrides ("accept once").
            let mut merged_known_hosts = state.model.settings.security.known_hosts.clone();
            for r in &state.runtime_known_hosts {
                if !merged_known_hosts.iter().any(|x| x.host == r.host && x.port == r.port) {
                    merged_known_hosts.push(r.clone());
                }
            }
            let result = state.rt.block_on(async {
                // Temporarily call backend with merged known hosts by swapping settings slice.
                // (We keep the persisted settings unchanged.)
                // NOTE: AppModel reads `settings.security.known_hosts`; so we pass merged slice by directly calling SshSession here would be invasive.
                // For now, update model's settings copy for this connect attempt only.
                let saved = std::mem::take(&mut state.model.settings.security.known_hosts);
                state.model.settings.security.known_hosts = merged_known_hosts;
                let out = state.model.connect_from_draft().await;
                // restore
                state.model.settings.security.known_hosts = saved;
                out
            });
            match result {
                Ok(session) => {
                    let host = state.model.draft.host.trim().to_string();
                    let user = state.model.draft.user.trim().to_string();
                    let port: u16 = state.model.draft.port.trim().parse().unwrap_or(22);

                    // Strategy A (minimal): if this connect came from direct input and is not yet
                    // associated with a saved session, upsert a session profile by de-dup key K=(user,host,port).
                    let mut profile_id = state.model.selected_session_id.clone();
                    if profile_id.is_none()
                        && matches!(state.model.draft.source, crate::app_model::DraftSource::DirectInput)
                        && !host.is_empty()
                        && !user.is_empty()
                    {
                        let existing = state.model.profiles().iter().find(|p| {
                            let crate::session::TransportConfig::Ssh(ssh) = &p.transport else {
                                return false;
                            };
                            ssh.host == host && ssh.port == port && ssh.user == user
                        });
                        let id = existing
                            .map(|p| p.id.clone())
                            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                        // Merge policy (doc Strategy A):
                        // - If an existing profile matches K, reuse it and avoid overriding user-customized fields (name/folder/auth).
                        // - For a new profile, keep folder None (avoid persisting localized labels).
                        let profile = if let Some(ex) = existing.cloned() {
                            ex
                        } else {
                            crate::session::SessionProfile {
                                id: id.clone(),
                                name: format!("{user}@{host}"),
                                folder: None,
                                color_tag: None,
                                transport: crate::session::TransportConfig::Ssh(crate::session::SshConfig {
                                    host: host.clone(),
                                    port,
                                    user: user.clone(),
                                    auth: state.model.draft.auth.clone(),
                                    credential_id: None,
                                }),
                            }
                        };
                        let _ = state.rt.block_on(state.model.session_manager.upsert_session(profile));
                        profile_id = Some(id.clone());
                        state.model.selected_session_id = profile_id.clone();
                    }

                    // Save credentials (minimal): only when vault is initialized and unlocked in this runtime.
                    if let (Some(pid), crate::session::AuthMethod::Password) =
                        (profile_id.clone(), state.model.draft.auth.clone())
                    {
                        let pw_non_empty =
                            !state.model.draft.password.expose_secret().trim().is_empty();
                        if pw_non_empty && state.model.settings.security.vault.is_some() {
                            if let Some(master) = state.model.vault_master_password.as_ref() {
                                if let Ok(Some(cid)) =
                                    crate::vault::session_credentials::sync_ssh_credentials_with_master(
                                        &state.model.settings,
                                        master,
                                        &pid,
                                        Some(&state.model.draft.password),
                                        None,
                                    )
                                {
                                    if let Some(existing) = state
                                        .model
                                        .profiles()
                                        .iter()
                                        .find(|p| p.id == pid)
                                        .cloned()
                                    {
                                        if let crate::session::TransportConfig::Ssh(mut ssh) =
                                            existing.transport
                                        {
                                            if ssh.credential_id.as_deref() != Some(cid.as_str())
                                            {
                                                ssh.credential_id = Some(cid);
                                                let updated = crate::session::SessionProfile {
                                                    transport: crate::session::TransportConfig::Ssh(
                                                        ssh,
                                                    ),
                                                    ..existing
                                                };
                                                let _ = state
                                                    .rt
                                                    .block_on(state.model.session_manager.upsert_session(updated));
                                            }
                                        }
                                    }
                                }
                                state.vault_status = VaultStatus::compute(
                                    &state.model.settings,
                                    state.model.vault_master_password.is_some(),
                                );
                            } else {
                                // Vault initialized but locked: prompt to unlock so we can save credentials.
                                // Do not block the session open; show modal after connect succeeds.
                                state.vault_unlock = Some(super::state::VaultUnlockState {
                                    pending_connect: None,
                                    pending_delete_profile_id: None,
                                    pending_save_session: false,
                                    pending_save_credentials_profile_id: Some(pid.clone()),
                                    password: secrecy::SecretString::from(String::new()),
                                    error: None,
                                });
                            }
                        }
                    }

                    let label = match &profile_id {
                        Some(pid) => state
                            .model
                            .profiles()
                            .iter()
                            .find(|s| s.id == *pid)
                            .map(|p| p.name.clone())
                            .unwrap_or_else(|| format!("{user}@{host}")),
                        None => format!("{user}@{host}"),
                    };
                    let recent = state
                        .model
                        .recent_record_for_draft_with_profile(label.clone(), profile_id.clone());
                    complete_new_ssh_session(state, session, recent, label, profile_id);
                    state.quick_connect_flow = super::state::QuickConnectFlow::Connected;
                    state.quick_connect_error_kind = None;
                    let msg = state.model.i18n.tr("iced.term.connected");
                    state.active_pane_mut().terminal.inject_local_lines(&[msg]);
                }
                Err(e) => {
                    if e == crate::app_model::ConnectErrorKind::AuthFailed
                        && matches!(state.model.draft.auth, crate::session::AuthMethod::Password)
                    {
                        state.model.draft.password_error_count =
                            state.model.draft.password_error_count.saturating_add(1);
                        if state.model.draft.password_error_count >= 3 {
                            state.quick_connect_flow = super::state::QuickConnectFlow::AuthLocked;
                            state.quick_connect_error_kind = Some(e);
                            return Task::none();
                        }
                    }
                    // Password auth failures should go to NeedAuthPassword to allow retry without losing context.
                    if e == crate::app_model::ConnectErrorKind::AuthFailed
                        && matches!(state.model.draft.auth, crate::session::AuthMethod::Password)
                    {
                        state.quick_connect_flow = super::state::QuickConnectFlow::NeedAuthPassword;
                        state.quick_connect_error_kind = Some(e);
                    } else {
                        state.quick_connect_flow = super::state::QuickConnectFlow::Failed;
                        state.quick_connect_error_kind = Some(e);
                    }
                    let fail = state.model.i18n.tr("iced.term.connection_failed");
                    let reason = format!("[rustssh] Reason: {:?}", e);
                    state.active_pane_mut().terminal.inject_local_lines(&[fail, &reason]);

                    // Host key policy branching.
                    if matches!(
                        e,
                        crate::app_model::ConnectErrorKind::HostKeyUnknown
                            | crate::app_model::ConnectErrorKind::HostKeyChanged
                    ) {
                        if let Some(info) = state.model.draft.host_key_error.clone() {
                            match state.model.settings.security.host_key_policy {
                                crate::settings::HostKeyPolicy::AcceptNew
                                    if e == crate::app_model::ConnectErrorKind::HostKeyUnknown =>
                                {
                                    // Auto accept & persist, then retry.
                                    state
                                        .model
                                        .settings
                                        .security
                                        .known_hosts
                                        .retain(|r| !(r.host == info.host && r.port == info.port));
                                    state.model.settings.security.known_hosts.push(
                                        crate::settings::KnownHostRecord {
                                            host: info.host.clone(),
                                            port: info.port,
                                            algo: info.algo.clone(),
                                            fingerprint: info.fingerprint.clone(),
                                            added_ms: crate::settings::unix_time_ms(),
                                        },
                                    );
                                    let _ = state.model.settings.save();
                                    state.host_key_prompt = None;
                                    return update(state, Message::ConnectPressed);
                                }
                                crate::settings::HostKeyPolicy::Ask => {
                                    state.host_key_prompt =
                                        Some(super::state::HostKeyPromptState { info });
                                    state.quick_connect_open = false;
                                    return Task::none();
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Task::none()
        }
        Message::HostKeyAcceptOnce => {
            let Some(p) = state.host_key_prompt.take() else {
                return Task::none();
            };
            let info = p.info;
            state.runtime_known_hosts.retain(|r| !(r.host == info.host && r.port == info.port));
            state.runtime_known_hosts.push(crate::settings::KnownHostRecord {
                host: info.host,
                port: info.port,
                algo: info.algo,
                fingerprint: info.fingerprint,
                added_ms: crate::settings::unix_time_ms(),
            });
            update(state, Message::ConnectPressed)
        }
        Message::HostKeyAlwaysTrust => {
            let Some(p) = state.host_key_prompt.take() else {
                return Task::none();
            };
            let info = p.info;
            state
                .model
                .settings
                .security
                .known_hosts
                .retain(|r| !(r.host == info.host && r.port == info.port));
            state.model.settings.security.known_hosts.push(crate::settings::KnownHostRecord {
                host: info.host,
                port: info.port,
                algo: info.algo,
                fingerprint: info.fingerprint,
                added_ms: crate::settings::unix_time_ms(),
            });
            let _ = state.model.settings.save();
            update(state, Message::ConnectPressed)
        }
        Message::HostKeyReject => {
            state.host_key_prompt = None;
            Task::none()
        }
        Message::QuickConnectInteractiveAnswerChanged(i, v) => {
            if let Some(flow) = state.quick_connect_interactive.as_mut() {
                if i < flow.ui.answers.len() {
                    flow.ui.answers[i] = v;
                    flow.ui.error = None;
                }
            }
            Task::none()
        }
        Message::QuickConnectInteractiveSubmit => {
            let Some(flow) = state.quick_connect_interactive.take() else {
                return Task::none();
            };
            let answers = flow.ui.answers.clone();
            let (sess, step) = match state.rt.block_on(flow.session.respond(answers)) {
                Ok(v) => v,
                Err(_e) => {
                    state.quick_connect_flow = super::state::QuickConnectFlow::Failed;
                    state.quick_connect_error_kind =
                        Some(crate::app_model::ConnectErrorKind::Unknown);
                    return Task::none();
                }
            };
            match step {
                crate::backend::ssh_session::KeyboardInteractiveStep::Success => {
                    match state.rt.block_on(sess.finish_into_session()) {
                        Ok(ssh_sess) => {
                            let session: Box<dyn AsyncSession> = Box::new(ssh_sess);
                            let host = state.model.draft.host.trim().to_string();
                            let user = state.model.draft.user.trim().to_string();
                            let port: u16 = state.model.draft.port.trim().parse().unwrap_or(22);
                            let label = format!("{user}@{host}:{port}");
                            let profile_id = state.model.selected_session_id.clone();
                            let recent = state
                                .model
                                .recent_record_for_draft_with_profile(label.clone(), profile_id.clone());
                            complete_new_ssh_session(state, session, recent, label, profile_id);
                            state.quick_connect_flow = super::state::QuickConnectFlow::Connected;
                            state.quick_connect_error_kind = None;
                        }
                        Err(_e) => {
                            state.quick_connect_flow = super::state::QuickConnectFlow::Failed;
                            state.quick_connect_error_kind =
                                Some(crate::app_model::ConnectErrorKind::Unknown);
                        }
                    }
                }
                crate::backend::ssh_session::KeyboardInteractiveStep::Failure => {
                    state.quick_connect_flow = super::state::QuickConnectFlow::Failed;
                    state.quick_connect_error_kind =
                        Some(crate::app_model::ConnectErrorKind::AuthFailed);
                }
                crate::backend::ssh_session::KeyboardInteractiveStep::InfoRequest(info) => {
                    state.quick_connect_interactive = Some(super::state::InteractiveAuthFlow {
                        session: sess,
                        ui: super::state::InteractivePromptState {
                            name: info.name,
                            instructions: info.instructions,
                            prompts: info.prompts.clone(),
                            answers: vec![String::new(); info.prompts.len()],
                            error: None,
                        },
                    });
                    state.quick_connect_flow = super::state::QuickConnectFlow::NeedAuthInteractive;
                    state.quick_connect_error_kind = None;
                }
            }
            Task::none()
        }
        Message::AutoProbeConsentOpen => {
            state.auto_probe_consent_modal = Some(super::state::AutoProbeConsentModalState {});
            Task::none()
        }
        Message::AutoProbeConsentAllowOnce => {
            state.auto_probe_consent_modal = None;
            // Continue pending action (connect).
            update(state, Message::ConnectPressed)
        }
        Message::AutoProbeConsentAlwaysAllow => {
            state.model.settings.security.auto_probe_consent = crate::settings::AutoProbeConsent::AlwaysAllow;
            let _ = state.model.settings.save();
            state.auto_probe_consent_modal = None;
            update(state, Message::ConnectPressed)
        }
        Message::AutoProbeConsentUsePassword => {
            // For this attempt: switch to password flow and skip probing.
            state.model.draft.auth = crate::session::AuthMethod::Password;
            state.auto_probe_consent_modal = None;
            update(state, Message::ConnectPressed)
        }
        Message::DisconnectPressed => {
            disconnect_active_tab_session(state);
            state.active_pane_mut().last_terminal_focus_sent = None;
            Task::none()
        }
        Message::ProfileConnect(profile) => {
            // If this profile expects credentials from vault but we don't have a runtime master password yet,
            // ask user to unlock first.
            let needs_vault = match &profile.transport {
                crate::session::TransportConfig::Ssh(ssh) => ssh.credential_id.is_some(),
                _ => false,
            };
            if needs_vault
                && state.model.settings.security.vault.is_some()
                && state.model.vault_master_password.is_none()
            {
                // Expected UX: close quick connect first, then show vault unlock.
                state.quick_connect_open = false;
                state.quick_connect_panel = QuickConnectPanel::Picker;
                let a = state.model.i18n.tr("iced.term.vault_needed");
                let b = state.model.i18n.tr("iced.term.vault_unlock_to_continue");
                state.active_pane_mut().terminal.inject_local_lines(&[a, b]);
                return update(state, Message::VaultUnlockOpenConnect(profile));
            }

            let master = state.model.vault_master_password.clone();
            match state
                .model
                .fill_draft_from_profile(&profile, master.as_ref()) {
                Ok(()) => {
                    // Auto-connect policy (Epic 4): if we already have enough auth info, connect immediately.
                    let mut can_auto_connect = false;
                    if let crate::session::TransportConfig::Ssh(_ssh) = &profile.transport {
                        match &state.model.draft.auth {
                            crate::session::AuthMethod::Password => {
                                can_auto_connect = !state
                                    .model
                                    .draft
                                    .password
                                    .expose_secret()
                                    .trim()
                                    .is_empty();
                            }
                            crate::session::AuthMethod::Key { private_key_path } => {
                                can_auto_connect = !private_key_path.trim().is_empty();
                            }
                            crate::session::AuthMethod::Agent => {
                                // Agent has no additional UI fields.
                                can_auto_connect = true;
                            }
                            crate::session::AuthMethod::Interactive => {
                                // For now, interactive is treated as "needs user input" (future: prompt loop).
                                can_auto_connect = false;
                            }
                        }
                    }

                    state.quick_connect_open = true;
                    state.quick_connect_panel = QuickConnectPanel::NewConnection;
                    if can_auto_connect {
                        return update(state, Message::ConnectPressed);
                    }
                }
                Err(msg) => {
                    state.model.status = msg;
                }
            }
            Task::none()
        }
        Message::VaultUnlockOpenConnect(profile) => {
            state.vault_unlock = Some(super::state::VaultUnlockState {
                pending_connect: Some(profile),
                pending_delete_profile_id: None,
                pending_save_session: false,
                pending_save_credentials_profile_id: None,
                password: secrecy::SecretString::from(String::new()),
                error: None,
            });
            Task::none()
        }
        Message::VaultUnlockOpenDelete(id) => {
            state.vault_unlock = Some(super::state::VaultUnlockState {
                pending_connect: None,
                pending_delete_profile_id: Some(id),
                pending_save_session: false,
                pending_save_credentials_profile_id: None,
                password: secrecy::SecretString::from(String::new()),
                error: None,
            });
            Task::none()
        }
        Message::VaultUnlockOpenSaveSession => {
            state.vault_unlock = Some(super::state::VaultUnlockState {
                pending_connect: None,
                pending_delete_profile_id: None,
                pending_save_session: true,
                pending_save_credentials_profile_id: None,
                password: secrecy::SecretString::from(String::new()),
                error: None,
            });
            Task::none()
        }
        Message::VaultUnlockClose => {
            state.vault_unlock = None;
            Task::none()
        }
        Message::VaultUnlockPasswordChanged(v) => {
            if let Some(u) = state.vault_unlock.as_mut() {
                u.password = secrecy::SecretString::from(v);
                u.error = None;
            }
            Task::none()
        }
        Message::VaultUnlockSubmit => {
            let Some(u) = state.vault_unlock.as_mut() else {
                return Task::none();
            };
            let Some(meta) = state.model.settings.security.vault.as_ref() else {
                u.error = Some("Vault 未初始化".to_string());
                return Task::none();
            };
            if !crate::vault::VaultManager::verify_password(&u.password, meta) {
                u.error = Some("密码错误".to_string());
                return Task::none();
            }
            // Vault secret key is derived from persisted verifier hash (not the raw user password).
            state.model.vault_master_password = Some(secrecy::SecretString::from(meta.verifier_hash.clone()));
            let pending_connect = u.pending_connect.clone();
            let pending_delete = u.pending_delete_profile_id.clone();
            let pending_save = u.pending_save_session;
            let pending_save_credentials = u.pending_save_credentials_profile_id.clone();
            state.vault_unlock = None;
            state.vault_status = VaultStatus::compute(
                &state.model.settings,
                state.model.vault_master_password.is_some(),
            );
            if pending_save {
                return update(state, Message::SessionEditorSave);
            }
            if let Some(id) = pending_delete {
                return update(state, Message::DeleteSessionProfile(id));
            }
            if let Some(pid) = pending_save_credentials {
                // Save password entered in current draft after successful connect.
                let Some(master) = state.model.vault_master_password.as_ref() else {
                    return Task::none();
                };
                let pw_ok = !state.model.draft.password.expose_secret().trim().is_empty();
                if pw_ok {
                    if let Ok(Some(cid)) =
                        crate::vault::session_credentials::sync_ssh_credentials_with_master(
                            &state.model.settings,
                            master,
                            &pid,
                            Some(&state.model.draft.password),
                            None,
                        )
                    {
                        if let Some(existing) = state
                            .model
                            .profiles()
                            .iter()
                            .find(|p| p.id == pid)
                            .cloned()
                        {
                            if let crate::session::TransportConfig::Ssh(mut ssh) = existing.transport {
                                ssh.credential_id = Some(cid);
                                let updated = crate::session::SessionProfile {
                                    transport: crate::session::TransportConfig::Ssh(ssh),
                                    ..existing
                                };
                                let _ = state
                                    .rt
                                    .block_on(state.model.session_manager.upsert_session(updated));
                            }
                        }
                    }
                }
                state.vault_status = VaultStatus::compute(
                    &state.model.settings,
                    state.model.vault_master_password.is_some(),
                );
                return Task::none();
            }
            if let Some(prof) = pending_connect {
                let a = state.model.i18n.tr("iced.term.vault_unlocked");
                let b = state.model.i18n.tr("iced.term.connecting");
                {
                    let pane = state.active_pane_mut();
                    pane.terminal.clear_local_preconnect_ui();
                    pane.terminal.inject_local_lines(&[a, b]);
                }
                return update(state, Message::ProfileConnect(prof));
            }
            Task::none()
        }
        Message::SaveSettings => {
            state.model.status = if state.model.settings.save().is_ok() {
                let tr = state.model.settings.terminal.terminal_render_mode;
                let pu = state.model.settings.terminal.plain_text_update;
                for p in &mut state.tab_panes {
                    p.terminal.set_render_mode(tr);
                    p.terminal.set_plain_text_update(pu);
                }
                sync_terminal_grid_to_session(state);
                for p in &mut state.tab_panes {
                    p.terminal.refresh_terminal_snapshots();
                }
                "Settings saved".to_string()
            } else {
                "Settings save failed".to_string()
            };
            Task::none()
        }
    }
}

fn mark_restart_if_gpu_setting(state: &mut IcedState) {
    state.settings_needs_restart = true;
}

fn apply_settings_field(state: &mut IcedState, field: SettingsField) {
    let sync_layout = matches!(
        &field,
        SettingsField::LineHeight(_)
            | SettingsField::TerminalFontSize(_)
            | SettingsField::ApplyTerminalMetrics(_)
    );
    let sync_palette = matches!(&field, SettingsField::ColorScheme(_));
    let s = &mut state.model.settings;
    match field {
        SettingsField::Language(code) => {
            s.general.language = code.clone();
            let _ = s.save();
            state.model.set_ui_language(&code);
            return;
        }
        SettingsField::AutoCheckUpdate(v) => s.general.auto_check_update = v,
        SettingsField::Theme(v) => s.general.theme = v,
        SettingsField::AccentColor(v) => s.general.accent_color = v.trim().to_string(),
        SettingsField::FontSize(v) => s.general.font_size = v,
        SettingsField::GpuAcceleration(v) => {
            s.terminal.gpu_acceleration = v;
            mark_restart_if_gpu_setting(state);
        }
        SettingsField::TargetFps(v) => s.terminal.target_fps = v,
        SettingsField::AtlasResetOnPressure(v) => s.terminal.atlas_reset_on_pressure = v,
        SettingsField::ColorScheme(v) => s.terminal.color_scheme = v,
        SettingsField::TerminalFontSize(v) => s.terminal.font_size = v,
        SettingsField::LineHeight(v) => s.terminal.line_height = v,
        SettingsField::ApplyTerminalMetrics(v) => s.terminal.apply_terminal_metrics = v,
        SettingsField::FontFamily(v) => s.terminal.font_family = v,
        SettingsField::GpuFontPath(v) => {
            let t = v.trim().to_string();
            s.terminal.gpu_font_path = if t.is_empty() { None } else { Some(t) };
            mark_restart_if_gpu_setting(state);
        }
        SettingsField::GpuFontFaceIndex(v) => {
            let t = v.trim();
            s.terminal.gpu_font_face_index = if t.is_empty() {
                None
            } else {
                t.parse().ok()
            };
            mark_restart_if_gpu_setting(state);
        }
        SettingsField::RightClickPaste(v) => s.terminal.right_click_paste = v,
        SettingsField::BracketedPaste(v) => {
            s.terminal.bracketed_paste = v;
            for p in &mut state.tab_panes {
                p.terminal.set_bracketed_paste(v);
            }
        }
        SettingsField::KeepSelectionHighlight(v) => s.terminal.keep_selection_highlight = v,
        SettingsField::ScrollbackLimit(v) => s.terminal.scrollback_limit = v,
        SettingsField::HistorySearch(v) => s.terminal.history_search_enabled = v,
        SettingsField::PathCompletion(v) => s.terminal.local_path_completion_enabled = v,
        SettingsField::TerminalRenderMode(m) => {
            s.terminal.terminal_render_mode = m;
            for p in &mut state.tab_panes {
                p.terminal.set_render_mode(m);
                p.terminal.refresh_terminal_snapshots();
            }
        }
        SettingsField::PlainTextUpdate(m) => {
            s.terminal.plain_text_update = m;
            for p in &mut state.tab_panes {
                p.terminal.set_plain_text_update(m);
            }
        }
        SettingsField::SingleSharedSession(v) => {
            s.quick_connect.single_shared_session = v;
            if v {
                let keep = state.active_tab;
                enforce_single_session_policy(state, Some(keep));
                sync_terminal_grid_to_session(state);
                TerminalHost::sync_focus_report(state);
            }
        }
        SettingsField::IdleTimeoutMins(v) => s.security.idle_timeout_mins = v,
        SettingsField::LockOnSleep(v) => s.security.lock_on_sleep = v,
        SettingsField::HostKeyPolicy(p) => s.security.host_key_policy = p,
        SettingsField::ConnectionSearch(q) => {
            state.settings_connection_search = q;
            return;
        }
    }
    let _ = state.model.settings.save();
    if sync_layout {
        if state.model.settings.quick_connect.single_shared_session {
            sync_terminal_grid_to_session(state);
        } else {
            let window_size = state.window_size;
            let term_settings = state.model.settings.terminal.clone();
            for p in &mut state.tab_panes {
                apply_terminal_grid_resize_for_pane(window_size, p, &term_settings);
            }
        }
    }
    if sync_palette {
        let scheme = state.model.settings.terminal.color_scheme.clone();
        for p in &mut state.tab_panes {
            p.terminal
                .apply_terminal_palette_for_scheme(&scheme);
        }
    }
}

/// 与内置 [`scrollable`] 类似：Lines 使用 ×60 缩放；垂直分量映射为横向滚动，无需 Shift。
fn tab_strip_wheel_to_offset_x(delta: ScrollDelta) -> f32 {
    match delta {
        ScrollDelta::Lines { x, y } => -(x + y) * 60.0,
        ScrollDelta::Pixels { x, y } => -(x + y),
    }
}
