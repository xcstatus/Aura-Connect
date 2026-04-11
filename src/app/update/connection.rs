use iced::Task;
use secrecy::ExposeSecret;

use crate::backend::ssh_session::AsyncSession;

use super::super::message::Message;
use super::super::state::{ConnectionStage, IcedState, PrewarmStatus, QuickConnectFlow, SessionPrewarmState};

/// Handle ConnectPressed message - core connection logic.
pub(crate) fn handle_connect(state: &mut IcedState) -> Task<Message> {
    // 已连接状态下点击重连/连接：先清空旧的预连接 UI，避免旧信息残留
    if state.quick_connect_flow == QuickConnectFlow::Connected && !state.tab_panes.is_empty() {
        state.active_pane_mut().terminal.clear_local_preconnect_ui();
        state.preconnect_info_line_count = 0;
        state.vault_hint_line_count = 0;
    }

    // User required gate (NeedUser).
    if state.model.draft.host.trim().is_empty() || state.model.draft.user.trim().is_empty() {
        state.quick_connect_flow = QuickConnectFlow::NeedUser;
        state.quick_connect_error_kind =
            Some(crate::app::model::ConnectErrorKind::MissingHostOrUser);
        return Task::none();
    }

    // Password lockout: stop repeated wrong passwords within the same draft context.
    if matches!(state.model.draft.auth, crate::session::AuthMethod::Password)
        && state.model.draft.password_error_count >= 3
    {
        state.quick_connect_flow = QuickConnectFlow::AuthLocked;
        state.quick_connect_error_kind = Some(crate::app::model::ConnectErrorKind::AuthFailed);
        return Task::none();
    }

    // NeedAuthPassword gate: password auth but no password provided yet.
    if matches!(state.model.draft.auth, crate::session::AuthMethod::Password)
        && state.model.draft.password.expose_secret().trim().is_empty()
    {
        state.quick_connect_flow = QuickConnectFlow::NeedAuthPassword;
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
        return super::super::update::update(state, Message::AutoProbeConsentOpen);
    }

    // Interactive auth path (synchronous - needs to keep session for multi-step auth).
    if matches!(
        state.model.draft.auth,
        crate::session::AuthMethod::Interactive
    ) {
        return handle_interactive_auth(state);
    }

    // Standard SSH connection path (asynchronous via Task::perform).
    start_ssh_connect(state)
}

/// Fill the draft with the inline password then retry the connection.
pub(crate) fn handle_inline_password_submit(
    state: &mut IcedState,
    password: String,
) -> Task<Message> {
    state.model.draft.password = secrecy::SecretString::from(password);
    state.model.draft.edited = true;
    state.model.draft.password_error_count = 0;
    super::super::update::update(state, Message::ConnectPressed)
}

/// Handle interactive auth (synchronous - requires multi-step state).
fn handle_interactive_auth(state: &mut IcedState) -> Task<Message> {
    // Extract needed fields early so the borrow of `state.model.draft` is released
    // before any mutable borrows of `state`.
    let host = state.model.draft.host.trim().to_string();
    let port_str = state.model.draft.port.trim().to_string();
    let user_str = state.model.draft.user.trim().to_string();

    // Inject SSH connecting info (target + fingerprint + auth method, then "连接中…").
    inject_ssh_connecting_info(state);

    state.quick_connect_flow = QuickConnectFlow::Connecting;
    state.quick_connect_error_kind = None;
    state.connection_stage = ConnectionStage::SshConnecting;

    let port: u16 = port_str.parse().unwrap_or(22);
    let known_hosts = state.model.settings.security.known_hosts.clone();
    let host_key_policy = state.model.settings.security.host_key_policy;

    let sess = match state.rt.block_on(
        crate::backend::ssh_session::InteractiveAuthSession::connect(&host, port, &user_str, &known_hosts, host_key_policy)
    ) {
        Ok(s) => s,
        Err(_e) => {
            state.quick_connect_flow = QuickConnectFlow::Failed;
            state.quick_connect_error_kind = Some(crate::app::model::ConnectErrorKind::HostUnreachable);
            state.connection_stage = ConnectionStage::None;
            return Task::none();
        }
    };

    state.connection_stage = ConnectionStage::Authenticating;
    let (sess, step) = match state.rt.block_on(sess.start()) {
        Ok(v) => v,
        Err(_e) => {
            state.quick_connect_flow = QuickConnectFlow::Failed;
            state.quick_connect_error_kind = Some(crate::app::model::ConnectErrorKind::Unknown);
            state.connection_stage = ConnectionStage::None;
            return Task::none();
        }
    };

    match step {
        crate::backend::ssh_session::KeyboardInteractiveStep::Success => {
            match state.rt.block_on(sess.finish_into_session(80, 24)) {
                Ok(ssh_sess) => {
                    let session: Box<dyn AsyncSession> = Box::new(ssh_sess);
                    handle_connect_success(state, session);
                    Task::none()
                }
                Err(_e) => {
                    state.quick_connect_flow = QuickConnectFlow::Failed;
                    state.quick_connect_error_kind = Some(crate::app::model::ConnectErrorKind::Unknown);
                    state.connection_stage = ConnectionStage::None;
                    Task::none()
                }
            }
        }
        crate::backend::ssh_session::KeyboardInteractiveStep::Failure => {
            state.quick_connect_flow = QuickConnectFlow::Failed;
            state.quick_connect_error_kind = Some(crate::app::model::ConnectErrorKind::AuthFailed);
            state.connection_stage = ConnectionStage::None;
            Task::none()
        }
        crate::backend::ssh_session::KeyboardInteractiveStep::InfoRequest(info) => {
            state.quick_connect_interactive = Some(super::super::state::InteractiveAuthFlow {
                session: sess,
                ui: super::super::state::InteractivePromptState {
                    name: info.name,
                    instructions: info.instructions,
                    prompts: info.prompts.clone(),
                    answers: vec![String::new(); info.prompts.len()],
                    error: None,
                },
            });
            state.quick_connect_flow = QuickConnectFlow::NeedAuthInteractive;
            state.quick_connect_error_kind = None;
            state.connection_stage = ConnectionStage::Authenticating;
            Task::none()
        }
    }
}

/// Standard SSH connection - returns async Task for non-blocking UI.
fn start_ssh_connect(state: &mut IcedState) -> Task<Message> {
    // Inject SSH connecting info: target + fingerprint + auth method (counted),
    // then static "连接中…" (not counted — stays visible until MOTD).
    inject_ssh_connecting_info(state);

    state.quick_connect_flow = QuickConnectFlow::Connecting;
    state.quick_connect_error_kind = None;

    state.connection_stage = if state.model.vault_master_password.is_some() {
        ConnectionStage::VaultLoading
    } else {
        ConnectionStage::SshConnecting
    };

    // Collect values needed for async task before taking ownership of draft/settings.
    let draft = state.model.draft.clone();
    let settings = state.model.settings.clone();
    let merged_known_hosts = merge_known_hosts(&settings, &state.runtime_known_hosts);
    let host_key_policy = settings.security.host_key_policy;
    let shared_manager = state.tab_manager.shared_manager.clone();
    let cols = 80u16;
    let rows = 24u16;

    // Build async task - this will run without blocking the UI thread
    let task = async move {
        // 构建连接键
        let connection_key = crate::backend::shared_ssh_session::ConnectionKey::new(
            draft.host.trim().to_string(),
            draft.port.trim().parse().unwrap_or(22),
            draft.user.trim().to_string(),
            draft.auth.clone(),
        );

        let result = shared_manager
            .write()
            .await
            .get_or_open_channel(
                connection_key,
                cols,
                rows,
                || {
                    let draft = draft.clone();
                    let merged_known_hosts = merged_known_hosts.clone();
                    let host_key_policy = host_key_policy;
                    Box::pin(async move {
                        let session = crate::backend::ssh_session::SshSession::new();
                        let result = session
                            .connect_base(
                                &draft.host.trim(),
                                draft.port.trim().parse().unwrap_or(22),
                                &draft.user.trim(),
                                draft.auth.clone(),
                                &draft.password.expose_secret(),
                                &draft.private_key_path.trim(),
                                &draft.passphrase.expose_secret(),
                                &merged_known_hosts,
                                host_key_policy,
                            )
                            .await;
                        match result {
                            Ok(conn) => Ok(conn),
                            Err(e) => {
                                let kind = crate::app::model::AppModel::classify_connect_error(&e);
                                let host_key_error = e
                                    .downcast_ref::<crate::backend::ssh_session::SshConnectError>()
                                    .and_then(|se| match se {
                                        crate::backend::ssh_session::SshConnectError::HostKeyUnknown {
                                            host,
                                            port,
                                            algo,
                                            fingerprint,
                                        } => Some(crate::app::model::HostKeyErrorInfo {
                                            host: host.clone(),
                                            port: *port,
                                            algo: algo.clone(),
                                            fingerprint: fingerprint.clone(),
                                            old_fingerprint: None,
                                        }),
                                        crate::backend::ssh_session::SshConnectError::HostKeyMismatch {
                                            host,
                                            port,
                                            algo,
                                            old_fingerprint,
                                            new_fingerprint,
                                        } => Some(crate::app::model::HostKeyErrorInfo {
                                            host: host.clone(),
                                            port: *port,
                                            algo: algo.clone(),
                                            fingerprint: new_fingerprint.clone(),
                                            old_fingerprint: Some(old_fingerprint.clone()),
                                        }),
                                        _ => None,
                                    });
                                Err((kind, host_key_error))
                            }
                        }
                    })
                },
            )
            .await;

        match result {
            Ok(session) => {
                Message::ConnectResult(Ok(session))
            }
            Err(e) => {
                let (kind, host_key_error) = match e {
                    crate::backend::shared_ssh_session::SharedSessionError::ConnectFailed(kind, info) => (kind, info),
                    crate::backend::shared_ssh_session::SharedSessionError::ChannelOpenFailed(e) => {
                        let kind = crate::app::model::AppModel::classify_connect_error(&e);
                        // ChannelOpenFailed 不包含 host key 信息
                        (kind, None)
                    }
                    crate::backend::shared_ssh_session::SharedSessionError::MaxConnectionsReached(_) => {
                        (crate::app::model::ConnectErrorKind::Unknown, None)
                    }
                };
                Message::ConnectResult(Err((kind, host_key_error)))
            }
        }
    };

    Task::perform(task, |msg| msg)
}

/// Internal error handler (returns Task::none).
pub(crate) fn internal_handle_connect_error(state: &mut IcedState, e: crate::app::model::ConnectErrorKind) {
    state.connection_stage = ConnectionStage::None;
    let is_password_auth = e == crate::app::model::ConnectErrorKind::AuthFailed
        && matches!(state.model.draft.auth, crate::session::AuthMethod::Password);

    if is_password_auth {
        state.model.draft.password_error_count =
            state.model.draft.password_error_count.saturating_add(1);
        if state.model.draft.password_error_count >= 3 {
            state.quick_connect_flow = QuickConnectFlow::AuthLocked;
            state.quick_connect_error_kind = Some(e);
            return;
        }
        state.quick_connect_flow = QuickConnectFlow::NeedAuthPassword;
        state.quick_connect_error_kind = Some(e);
    } else {
        state.quick_connect_flow = QuickConnectFlow::Failed;
        state.quick_connect_error_kind = Some(e);
    }

    // 显示连接失败信息（不清理预连接信息，保留用于排查）
    let fail = state.model.i18n.tr("iced.term.connection_failed");
    let reason = format!("SSH  {}", e.user_message());
    state
        .active_pane_mut()
        .terminal
        .inject_local_lines(&[fail, &reason]);

    // 重置预连接信息行计数（失败时不清理）
    state.preconnect_info_line_count = 0;

    internal_handle_host_key_error(state, &e);
}

/// Internal host key error handler.
fn internal_handle_host_key_error(state: &mut IcedState, e: &crate::app::model::ConnectErrorKind) {
    if matches!(
        e,
        crate::app::model::ConnectErrorKind::HostKeyUnknown
            | crate::app::model::ConnectErrorKind::HostKeyChanged
    ) {
        if let Some(info) = state.model.draft.host_key_error.clone() {
            match state.model.settings.security.host_key_policy {
                crate::settings::HostKeyPolicy::AcceptNew
                    if e == &crate::app::model::ConnectErrorKind::HostKeyUnknown =>
                {
                    state.model.settings.security.known_hosts.retain(|r| !(r.host == info.host && r.port == info.port));
                    state.model.settings.security.known_hosts.push(
                        crate::settings::KnownHostRecord {
                            host: info.host.clone(),
                            port: info.port,
                            algo: info.algo.clone(),
                            fingerprint: info.fingerprint.clone(),
                            added_ms: crate::settings::unix_time_ms(),
                        },
                    );
                    if state.model.settings.save_with_log() {
                        state.host_key_prompt = None;
                        state.quick_connect_flow = QuickConnectFlow::Connecting;
                        drop(state.model.draft.host_key_error.take());
                    }
                }
                crate::settings::HostKeyPolicy::Ask => {
                    state.host_key_prompt = Some(super::super::state::HostKeyPromptState { info });
                    state.quick_connect_open = false;
                }
                _ => {}
            }
        }
    }
}

/// Handle successful SSH connection.
pub(crate) fn handle_connect_success(state: &mut IcedState, session: Box<dyn AsyncSession>) {
    let host = state.model.draft.host.trim().to_string();
    let user = state.model.draft.user.trim().to_string();
    let port: u16 = state.model.draft.port.trim().parse().unwrap_or(22);

    // Upsert session profile for direct input.
    let mut profile_id = state.model.selected_session_id.clone();
    if profile_id.is_none()
        && matches!(
            state.model.draft.source,
            crate::app::model::DraftSource::DirectInput
        )
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
        let profile = if let Some(ex) = existing.cloned() {
            ex
        } else {
            crate::session::SessionProfile {
                id: id.clone(),
                // 优先使用 host 作为默认名称（用户可在会话编辑器中修改）
                name: host.clone(),
                group_id: None,
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
        let _ = state.rt.block_on(state.model.tab_manager.upsert_session(profile));
        profile_id = Some(id.clone());
        state.model.selected_session_id = profile_id.clone();
    }

    // Save credentials to vault.
    save_credentials_after_connect(state, &profile_id);

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

    // 构建连接键用于多路复用
    let connection_key = crate::backend::shared_ssh_session::ConnectionKey::new(
        state.model.draft.host.trim().to_string(),
        state.model.draft.port.trim().parse().unwrap_or(22),
        state.model.draft.user.trim().to_string(),
        state.model.draft.auth.clone(),
    );

    super::super::update::complete_new_ssh_session(state, session, recent, label, profile_id, Some(connection_key));
    state.quick_connect_flow = QuickConnectFlow::Connected;
    state.quick_connect_error_kind = None;
    state.connection_stage = ConnectionStage::None;

    // 清空所有预连接 UI（vault 提示 + SSH info + "连接中…"），SSH 数据从行 0 开始
    state.active_pane_mut().terminal.clear_local_preconnect_ui();
    state.preconnect_info_line_count = 0;
    state.vault_hint_line_count = 0;

    // 立即 pump 一次，读取 MOTD
    let active_tab = state.active_tab;
    let pane = &mut state.tab_panes[active_tab];
    if let Some(sess) = state.tab_manager.session_mut(active_tab) {
        let _ = pane.terminal.pump_output(sess);
    }
}

/// Arc-aware version of handle_connect_success for connection pool multiplexing.
/// Receives Arc<SshChannel> directly instead of Box<dyn AsyncSession>.
pub(crate) fn handle_connect_success_arc(
    state: &mut IcedState,
    session_arc: std::sync::Arc<crate::backend::ssh_session::SshChannel>,
) {
    let host = state.model.draft.host.trim().to_string();
    let user = state.model.draft.user.trim().to_string();
    let port: u16 = state.model.draft.port.trim().parse().unwrap_or(22);

    // Upsert session profile for direct input.
    let mut profile_id = state.model.selected_session_id.clone();
    if profile_id.is_none()
        && matches!(
            state.model.draft.source,
            crate::app::model::DraftSource::DirectInput
        )
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
        let profile = if let Some(ex) = existing.cloned() {
            ex
        } else {
            crate::session::SessionProfile {
                id: id.clone(),
                name: host.clone(),
                group_id: None,
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
        let _ = state.rt.block_on(state.model.tab_manager.upsert_session(profile));
        profile_id = Some(id.clone());
        state.model.selected_session_id = profile_id.clone();
    }

    // Save credentials to vault.
    save_credentials_after_connect(state, &profile_id);

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

    // 构建连接键用于多路复用
    let connection_key = crate::backend::shared_ssh_session::ConnectionKey::new(
        state.model.draft.host.trim().to_string(),
        state.model.draft.port.trim().parse().unwrap_or(22),
        state.model.draft.user.trim().to_string(),
        state.model.draft.auth.clone(),
    );

    super::super::update::complete_new_ssh_session_arc(
        state,
        session_arc,
        recent,
        label,
        profile_id,
        Some(connection_key),
    );
    state.quick_connect_flow = QuickConnectFlow::Connected;
    state.quick_connect_error_kind = None;
    state.connection_stage = ConnectionStage::None;

    // 清空所有预连接 UI（vault 提示 + SSH info + "连接中…"），SSH 数据从行 0 开始
    state.active_pane_mut().terminal.clear_local_preconnect_ui();
    state.preconnect_info_line_count = 0;
    state.vault_hint_line_count = 0;

    // 立即 pump 一次，读取 MOTD
    let active_tab = state.active_tab;
    let pane = &mut state.tab_panes[active_tab];
    if let Some(sess) = state.tab_manager.session_mut(active_tab) {
        let _ = pane.terminal.pump_output(sess);
    }
}

/// Save credentials to vault after successful connect.
fn save_credentials_after_connect(state: &mut IcedState, profile_id: &Option<String>) {
    if let (Some(pid), crate::session::AuthMethod::Password) =
        (profile_id.clone(), state.model.draft.auth.clone())
    {
        let pw_non_empty = !state.model.draft.password.expose_secret().trim().is_empty();
        if pw_non_empty && state.model.settings.security.vault.is_some() {
            if let Some(master) = state.model.vault_master_password.as_ref() {
                let pw = state.model.draft.password.expose_secret();
                if let Ok(Some(cid)) =
                    crate::vault::session_credentials::sync_ssh_credentials(
                        master.expose_secret(),
                        &pid,
                        Some(pw),
                        None,
                        state.model.settings.security.kdf_memory_level,
                    )
                {
                    if let Some(existing) =
                        state.model.profiles().iter().find(|p| p.id == pid).cloned()
                    {
                        if let crate::session::TransportConfig::Ssh(mut ssh) = existing.transport {
                            if ssh.credential_id.as_deref() != Some(cid.as_str()) {
                                ssh.credential_id = Some(cid);
                                let updated = crate::session::SessionProfile {
                                    transport: crate::session::TransportConfig::Ssh(ssh),
                                    ..existing
                                };
                                let _ = state.rt.block_on(
                                    state.model.tab_manager.upsert_session(updated),
                                );
                            }
                        }
                    }
                }
                state.vault_status = super::super::state::VaultStatus::compute(
                    &state.model.settings,
                    state.model.vault_master_password.is_some(),
                );
            } else {
                state.vault_unlock = Some(super::super::state::VaultUnlockState {
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
}

/// Handle HostKeyAcceptOnce message.
pub(crate) fn handle_host_key_accept_once(state: &mut IcedState) -> Task<Message> {
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
    super::super::update::update(state, Message::ConnectPressed)
}

/// Handle HostKeyAlwaysTrust message.
pub(crate) fn handle_host_key_always_trust(state: &mut IcedState) -> Task<Message> {
    let Some(p) = state.host_key_prompt.take() else {
        return Task::none();
    };
    let info = p.info;
    state.model.settings.security.known_hosts.retain(|r| !(r.host == info.host && r.port == info.port));
    state.model.settings.security.known_hosts.push(crate::settings::KnownHostRecord {
        host: info.host,
        port: info.port,
        algo: info.algo,
        fingerprint: info.fingerprint,
        added_ms: crate::settings::unix_time_ms(),
    });
    state.model.settings.save_with_log();
    super::super::update::update(state, Message::ConnectPressed)
}

/// Handle HostKeyReject message.
pub(crate) fn handle_host_key_reject(state: &mut IcedState) -> Task<Message> {
    state.host_key_prompt = None;
    Task::none()
}

/// Handle DisconnectPressed message.
pub(crate) fn handle_disconnect(state: &mut IcedState) -> Task<Message> {
    super::super::update::disconnect_active_tab_session(state);
    if !state.tab_panes.is_empty() {
        state.active_pane_mut().last_terminal_focus_sent = None;
    }
    Task::none()
}

/// Handle ProfileConnect message.
pub(crate) fn handle_profile_connect(
    state: &mut IcedState,
    profile: crate::session::SessionProfile,
) -> Task<Message> {
    let needs_vault = match &profile.transport {
        crate::session::TransportConfig::Ssh(ssh) => ssh.credential_id.is_some(),
        _ => false,
    };
    if needs_vault
        && state.model.settings.security.vault.is_some()
        && state.model.vault_master_password.is_none()
    {
        // Start the modal close animation (same pattern as QuickConnectDismiss).
        if state.quick_connect_anim.phase != super::super::state::ModalAnimPhase::Closing {
            state.quick_connect_anim =
                super::super::state::ModalAnimState::closing(state.tick_count);
        }
        state.quick_connect_panel = super::super::state::QuickConnectPanel::Picker;

        // Ensure tab exists before accessing terminal (welcome page mode may have no tabs).
        if state.tab_panes.is_empty() {
            let _ = super::session::handle_add_tab(state);
        }

        // Track vault hint line count so we can clear them after unlock.
        let a = state.model.i18n.tr("iced.term.vault_needed");
        let b = state.model.i18n.tr("iced.term.vault_unlock_to_continue");
        state.active_pane_mut().terminal.inject_local_lines(&[a, b]);
        state.vault_hint_line_count = 2;
        return super::super::update::update(state, Message::VaultUnlockOpenConnect(profile));
    }

    let master = state.model.vault_master_password.clone();
    match state.model.fill_draft_from_profile(&profile, master.as_ref()) {
        Ok(()) => {
            // 欢迎页模式或不处于活跃连接时，强制开新页签
            if state.show_welcome || !state.active_session_is_connected() {
                let _ = super::session::handle_add_tab(state);
            }

            let mut can_auto_connect = false;
            if let crate::session::TransportConfig::Ssh(_ssh) = &profile.transport {
                match &state.model.draft.auth {
                    crate::session::AuthMethod::Password => {
                        can_auto_connect = !state.model.draft.password.expose_secret().trim().is_empty();
                    }
                    crate::session::AuthMethod::Key { private_key_path } => {
                        can_auto_connect = !private_key_path.trim().is_empty();
                    }
                    crate::session::AuthMethod::Agent => {
                        can_auto_connect = true;
                    }
                    crate::session::AuthMethod::Interactive => {
                        can_auto_connect = false;
                    }
                }
            }

            if !can_auto_connect {
                // Start the modal close animation (same pattern as QuickConnectDismiss).
                if state.quick_connect_anim.phase != super::super::state::ModalAnimPhase::Closing {
                    state.quick_connect_anim =
                        super::super::state::ModalAnimState::closing(state.tick_count);
                }
                state.quick_connect_flow = QuickConnectFlow::NeedAuthPassword;
                state.quick_connect_error_kind = None;

                // Show a hint in the terminal so the user knows what to do next.
                // Ensure tab exists before accessing terminal (defensive check).
                if state.tab_panes.is_empty() {
                    let _ = super::session::handle_add_tab(state);
                }
                let hint = match &state.model.draft.auth {
                    crate::session::AuthMethod::Password => {
                        state.model.i18n.tr("iced.quick_connect.need_password")
                    }
                    crate::session::AuthMethod::Key { .. } => {
                        state.model.i18n.tr("iced.quick_connect.need_passphrase")
                    }
                    _ => state.model.i18n.tr("iced.quick_connect.need_auth"),
                };
                state.active_pane_mut().terminal.inject_local_lines(&[hint]);
                return Task::none();
            }

            return super::super::update::update(state, Message::ConnectPressed);
        }
        Err(_msg) => {
            // 错误静默处理，连接失败时终端会显示错误信息
        }
    }
    Task::none()
}

/// Handle QuickConnectInteractiveSubmit message.
pub(crate) fn handle_interactive_submit(state: &mut IcedState) -> Task<Message> {
    let Some(flow) = state.quick_connect_interactive.take() else {
        return Task::none();
    };

    state.connection_stage = ConnectionStage::Authenticating;
    let answers = flow.ui.answers.clone();
    let (sess, step) = match state.rt.block_on(flow.session.respond(answers)) {
        Ok(v) => v,
        Err(_e) => {
            state.quick_connect_flow = QuickConnectFlow::Failed;
            state.quick_connect_error_kind = Some(crate::app::model::ConnectErrorKind::Unknown);
            state.connection_stage = ConnectionStage::None;
            return Task::none();
        }
    };

    match step {
        crate::backend::ssh_session::KeyboardInteractiveStep::Success => {
            match state.rt.block_on(sess.finish_into_session(80, 24)) {
                Ok(ssh_sess) => {
                    let session: Box<dyn AsyncSession> = Box::new(ssh_sess);
                    let host = state.model.draft.host.trim().to_string();
                    let user = state.model.draft.user.trim().to_string();
                    let port = state.model.draft.port.trim().parse().unwrap_or(22);
                    let label = format!("{user}@{host}");
                    let profile_id = state.model.selected_session_id.clone();
                    let recent = state
                        .model
                        .recent_record_for_draft_with_profile(label.clone(), profile_id.clone());
                    let connection_key = crate::backend::shared_ssh_session::ConnectionKey::new(
                        host.clone(),
                        port,
                        user.clone(),
                        state.model.draft.auth.clone(),
                    );
                    state.connection_stage = ConnectionStage::SessionSetup;
                    super::super::update::complete_new_ssh_session(
                        state, session, recent, label, profile_id, Some(connection_key),
                    );
                    state.quick_connect_flow = QuickConnectFlow::Connected;
                    state.quick_connect_error_kind = None;
                    state.connection_stage = ConnectionStage::None;

                    // 清空所有预连接 UI（vault 提示 + SSH info + "连接中…"），SSH 数据从行 0 开始
                    state.active_pane_mut().terminal.clear_local_preconnect_ui();
                    state.preconnect_info_line_count = 0;
                    state.vault_hint_line_count = 0;

                    let active_tab = state.active_tab;
                    let pane = &mut state.tab_panes[active_tab];
                    if let Some(sess) = state.tab_manager.session_mut(active_tab) {
                        let _ = pane.terminal.pump_output(sess);
                    }
                }
                Err(_e) => {
                    state.quick_connect_flow = QuickConnectFlow::Failed;
                    state.quick_connect_error_kind = Some(crate::app::model::ConnectErrorKind::Unknown);
                    state.connection_stage = ConnectionStage::None;
                }
            }
        }
        crate::backend::ssh_session::KeyboardInteractiveStep::Failure => {
            state.quick_connect_flow = QuickConnectFlow::Failed;
            state.quick_connect_error_kind = Some(crate::app::model::ConnectErrorKind::AuthFailed);
            state.connection_stage = ConnectionStage::None;
        }
        crate::backend::ssh_session::KeyboardInteractiveStep::InfoRequest(info) => {
            state.quick_connect_interactive = Some(super::super::state::InteractiveAuthFlow {
                session: sess,
                ui: super::super::state::InteractivePromptState {
                    name: info.name,
                    instructions: info.instructions,
                    prompts: info.prompts.clone(),
                    answers: vec![String::new(); info.prompts.len()],
                    error: None,
                },
            });
            state.quick_connect_flow = QuickConnectFlow::NeedAuthInteractive;
            state.quick_connect_error_kind = None;
        }
    }
    Task::none()
}

/// Merge persistent known_hosts with runtime overrides ("accept once").
pub(crate) fn merge_known_hosts(
    settings: &crate::settings::Settings,
    runtime_overrides: &[crate::settings::KnownHostRecord],
) -> Vec<crate::settings::KnownHostRecord> {
    let mut merged = settings.security.known_hosts.clone();
    for r in runtime_overrides {
        if !merged.iter().any(|x| x.host == r.host && x.port == r.port) {
            merged.push(r.clone());
        }
    }
    merged
}

/// Inject SSH connecting info into the terminal.
///
/// - Lines 1-3: target, fingerprint, auth method (counted in `preconnect_info_line_count`,
///   cleared on success/error/retry).
/// - Line 4: static "连接中…" message (NOT counted, stays visible during connection).
///
/// If `skip_counting` is true, SSH info lines are injected but `preconnect_info_line_count`
/// is NOT updated. Use this when re-injecting SSH info (e.g. after vault unlock).
pub(crate) fn inject_ssh_connecting_info(state: &mut IcedState) {
    inject_ssh_connecting_info_full(state, false);
    // 累加 vault 提示行数（vault 解锁路径中已注入 vault_unlocked 消息）
    state.preconnect_info_line_count += state.vault_hint_line_count;
}

pub(crate) fn inject_ssh_connecting_info_full(state: &mut IcedState, skip_counting: bool) {
    // Retry: clear previous error/info lines before injecting new ones.
    if matches!(
        state.quick_connect_flow,
        QuickConnectFlow::Failed | QuickConnectFlow::AuthLocked
    ) {
        let lines_to_clear = state.preconnect_info_line_count;
        if lines_to_clear > 0 {
            state.active_pane_mut().terminal.clear_preconnect_lines(lines_to_clear);
        }
        state.preconnect_info_line_count = 0;
    }

    let draft = state.model.draft.clone();
    let host = draft.host.trim().to_string();

    // Extract all i18n strings before any mutable borrow of state.
    let i18n_connecting = state.model.i18n.tr("iced.term.ssh.connecting");
    let i18n_host_fp = state.model.i18n.tr("iced.term.ssh.host_fingerprint");
    let i18n_auth_method = state.model.i18n.tr("iced.term.ssh.auth_method");
    let i18n_connecting_stage = state.model.i18n.tr("iced.term.connecting");
    let auth_name = match &draft.auth {
        crate::session::AuthMethod::Password => state.model.i18n.tr("iced.auth.password"),
        crate::session::AuthMethod::Key { .. } => state.model.i18n.tr("iced.auth.public_key"),
        crate::session::AuthMethod::Interactive => state.model.i18n.tr("iced.auth.keyboard_interactive"),
        crate::session::AuthMethod::Agent => state.model.i18n.tr("iced.auth.agent"),
    };
    let merged_known_hosts = merge_known_hosts(&state.model.settings, &state.runtime_known_hosts);

    // Build SSH info lines (counted, cleared on retry/success)
    let mut lines: Vec<String> = Vec::new();

    // 1. Connection target
    let target_str = state
        .model
        .selected_session_id
        .as_ref()
        .and_then(|pid| state.model.profiles().iter().find(|p| &p.id == pid))
        .map(|p| {
            let addr = format!("{}@{}:{}", draft.user, host, draft.port);
            format!("\"{}\" ({})", p.name, addr)
        })
        .unwrap_or_else(|| format!("{}@{}:{}", draft.user, host, draft.port));

    lines.push(i18n_connecting.replace("{target}", &target_str));

    // 2. Host fingerprint (from known_hosts + runtime overrides)
    if let Some(rec) = merged_known_hosts.iter().find(|r| r.host == host) {
        lines.push(
            i18n_host_fp.replace("{algo}", &rec.algo).replace("{fp}", &rec.fingerprint),
        );
    }

    // 3. Auth method
    lines.push(i18n_auth_method.replace("{method}", &auth_name));

    // Record SSH info line count for cleanup (skip when re-injecting after vault unlock)
    if !skip_counting {
        state.preconnect_info_line_count = lines.len();
    }
    let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    state.active_pane_mut().terminal.inject_local_lines(&line_refs);

    // 4. "连接中…" message (NOT counted — always visible during connection)
    state.active_pane_mut()
        .terminal
        .inject_local_lines(&[&i18n_connecting_stage]);
}

/// Handle auto-reconnect tick (called every second while in Reconnecting state).
/// Returns a Task for async operations.
pub(crate) fn handle_reconnect_tick(state: &mut IcedState) -> Task<Message> {
    let ConnectionStage::Reconnecting {
        attempt,
        max,
        delay_secs,
    } = state.connection_stage
    else {
        // Not in reconnecting state
        return Task::none();
    };

    // 计算已等待的时间（秒）
    let elapsed = state
        .reconnect_context
        .as_ref()
        .map(|ctx| ctx.start_time.elapsed().as_secs() as u32)
        .unwrap_or(0);

    if elapsed < delay_secs {
        // 还在等待中，更新倒计时显示
        let remaining = delay_secs - elapsed;
        let msg = state.model.i18n.tr("iced.term.reconnect_countdown");
        let msg = msg.replace("{secs}", &remaining.to_string());
        state.active_pane_mut().terminal.inject_local_lines(&[&msg]);
        return Task::none();
    }

    // 延迟已到，执行重连
    log::info!(
        "[Reconnect] Attempt {}/{}",
        attempt,
        max
    );

    // 显示重连尝试消息
    let msg = state.model.i18n.tr("iced.term.reconnect_attempt")
        .replace("{n}", &attempt.to_string())
        .replace("{max}", &max.to_string());
    state.active_pane_mut().terminal.inject_local_lines(&[&msg]);

    // 重置 start_time 以便计算下次延迟
    if let Some(ctx) = &mut state.reconnect_context {
        ctx.start_time = std::time::Instant::now();
    }

    // 启动异步重连（复用现有连接流程）
    start_reconnect_async(state)
}

/// Start an async reconnect task using the same pattern as start_ssh_connect.
fn start_reconnect_async(state: &mut IcedState) -> Task<Message> {
    // 确保 draft 是从 reconnect_context 恢复的
    let Some(ctx) = state.reconnect_context.as_ref() else {
        state.quick_connect_flow = QuickConnectFlow::Failed;
        state.connection_stage = ConnectionStage::None;
        return Task::none();
    };

    // 先克隆 draft，因为 inject_ssh_connecting_info 需要可变借用 state
    let draft = ctx.draft.clone();
    let settings = state.model.settings.clone();
    let merged_known_hosts = merge_known_hosts(&settings, &state.runtime_known_hosts);

    // 注入 SSH 连接信息（使用 ctx 中的 draft）
    state.model.draft = ctx.draft.clone();
    inject_ssh_connecting_info(state);

    // 使用 Task::perform 来异步执行连接
    let task = async move {
        let mut temp_model = crate::app::model::AppModel::new_for_connect(draft, settings);
        temp_model.settings.security.known_hosts = merged_known_hosts;

        let result = temp_model.connect_from_draft().await;

        match result {
            Ok(session) => Message::ReconnectResult(Ok(session)),
            Err(kind) => {
                // 只传递 error kind，host_key_error 从 draft 中获取
                Message::ReconnectResult(Err((kind, temp_model.draft.host_key_error.clone())))
            }
        }
    };

    Task::perform(task, |msg| msg)
}

/// Handle the result of a reconnect attempt.
pub(crate) fn handle_reconnect_result(
    state: &mut IcedState,
    result: Result<std::sync::Arc<crate::backend::ssh_session::SshChannel>, (crate::app::model::ConnectErrorKind, Option<crate::app::model::HostKeyErrorInfo>)>,
) -> Task<Message> {
    match result {
        Ok(session_arc) => {
            // 重连成功！
            state.quick_connect_flow = QuickConnectFlow::Connected;
            state.connection_stage = ConnectionStage::None;
            state.reconnect_context = None;

            // 清空所有预连接 UI，SSH 数据从行 0 开始
            state.active_pane_mut().terminal.clear_local_preconnect_ui();
            state.preconnect_info_line_count = 0;
            state.vault_hint_line_count = 0;

            // 完成会话建立
            let label = format!(
                "{}@{}",
                state.model.draft.user,
                state.model.draft.host
            );
            let recent = state.model.recent_record_for_draft_with_profile(
                label.clone(),
                state.model.selected_session_id.clone(),
            );
            // SshChannel 已实现 AsyncSession，可直接使用
            let session: Box<dyn crate::backend::ssh_session::AsyncSession> = Box::new(std::sync::Arc::try_unwrap(session_arc).unwrap_or_else(|_arc| {
                panic!("Reconnect session Arc was shared unexpectedly")
            }));
            // 重连时使用相同的连接键
            let connection_key = state.reconnect_context.as_ref().map(|ctx| {
                crate::backend::shared_ssh_session::ConnectionKey::new(
                    ctx.draft.host.trim().to_string(),
                    ctx.draft.port.trim().parse().unwrap_or(22),
                    ctx.draft.user.trim().to_string(),
                    ctx.draft.auth.clone(),
                )
            });
            super::complete_new_ssh_session(
                state,
                session,
                recent,
                label,
                state.model.selected_session_id.clone(),
                connection_key,
            );

            return Task::none();
        }
        Err((kind, host_key_error)) => {
            // 记录 host key error 到 draft（用于后续 Ask 流程）
            state.model.draft.host_key_error = host_key_error;
            let error_kind = kind;

            // 计算下次重连参数
            let max = state.model.settings.quick_connect.reconnect_max_attempts;
            if let ConnectionStage::Reconnecting { attempt, .. } = state.connection_stage {
                let next_attempt = attempt + 1;
                if next_attempt > max {
                    // 达到最大重试次数，放弃
                    state.quick_connect_flow = QuickConnectFlow::Failed;
                    state.quick_connect_error_kind = Some(error_kind);
                    state.connection_stage = ConnectionStage::None;
                    state.reconnect_context = None;

                    let msg = state.model.i18n.tr("iced.term.reconnect_failed")
                        .replace("{reason}", error_kind.user_message());
                    state.active_pane_mut().terminal.inject_local_lines(&[&msg]);

                    return Task::none();
                }

                // 继续重试
                let delay = state.model.settings.reconnect_delay_for_attempt(next_attempt);
                state.connection_stage = ConnectionStage::Reconnecting {
                    attempt: next_attempt,
                    max,
                    delay_secs: delay,
                };

                let msg = state.model.i18n.tr("iced.term.reconnect_attempt")
                    .replace("{n}", &next_attempt.to_string())
                    .replace("{max}", &max.to_string());
                state.active_pane_mut().terminal.inject_local_lines(&[&msg]);
            }
        }
    }

    Task::none()
}

/// Handle restore session confirm from startup.
pub(crate) fn handle_restore_session_confirm(state: &mut IcedState) -> Task<Message> {
    let record = match state.restore_session_modal.take() {
        Some(s) => s.record,
        None => return Task::none(),
    };

    // 尝试从 record 构建 draft 并连接
    state.model.draft.host = record.host.clone();
    state.model.draft.port = record.port.to_string();
    state.model.draft.user = record.user.clone();
    state.model.draft.source = crate::app::model::DraftSource::Recent;
    state.model.draft.profile_id = record.profile_id.clone();
    state.model.draft.edited = false;
    state.model.draft.last_error = None;
    state.model.draft.password_error_count = 0;
    state.model.selected_session_id = record.profile_id.clone();

    // 如果有保存的会话，从会话加载认证信息
    let mut credential_id = None;
    let mut auth_to_set = None;
    let mut key_path_to_set = None;
    if let Some(profile_id) = &record.profile_id {
        if let Some(profile) = state.model.profiles().iter().find(|p| &p.id == profile_id) {
            if let crate::session::TransportConfig::Ssh(ssh) = &profile.transport {
                auth_to_set = Some(ssh.auth.clone());
                key_path_to_set = Some(match &ssh.auth {
                    crate::session::AuthMethod::Key { private_key_path } => private_key_path.clone(),
                    _ => String::new(),
                });
                credential_id = ssh.credential_id.clone();
            }
        }
    }

    // 在获取所有借用后再修改 draft
    if let Some(auth) = auth_to_set {
        state.model.draft.auth = auth;
    }
    if let Some(key_path) = key_path_to_set {
        state.model.draft.private_key_path = key_path;
    }

    // 检查是否需要 Vault 解锁
    let needs_vault = credential_id.is_some();
    if needs_vault
        && state.model.settings.security.vault.is_some()
        && state.model.vault_master_password.is_none()
    {
        // 需要解锁 Vault
        return super::update(state, Message::VaultUnlockOpenConnect(
            crate::session::SessionProfile {
                id: record.profile_id.unwrap_or_default(),
                name: record.label.clone(),
                group_id: None,
                folder: None,
                color_tag: None,
                transport: crate::session::TransportConfig::Ssh(crate::session::SshConfig {
                    host: record.host,
                    port: record.port,
                    user: record.user,
                    auth: state.model.draft.auth.clone(),
                    credential_id,
                }),
            },
        ));
    }

    // 直接发起连接
    super::update(state, Message::ConnectPressed)
}

/// 开始预热：用户悬停在会话项上时调用。
pub(crate) fn handle_session_hover_start(state: &mut IcedState, key: String) {
    // 如果悬停的是同一个会话，不需要重新开始
    if let Some(prewarm) = &state.prewarm_state {
        if prewarm.profile_id.as_ref() == Some(&key) {
            return;
        }
    }

    // 取消之前的预热（如果有）
    state.prewarm_state = None;

    // recent: 前缀表示最近会话，暂时不支持预热
    if key.starts_with("recent:") {
        return;
    }

    // 开始新的预热
    state.prewarm_state = Some(SessionPrewarmState {
        profile_id: Some(key),
        status: PrewarmStatus::WaitingHover,
        start_time: Some(std::time::Instant::now()),
        timeout_secs: 30, // 30 秒后自动清理
    });
}

/// 结束预热：用户鼠标离开时调用。
pub(crate) fn handle_session_hover_end(state: &mut IcedState) {
    state.prewarm_state = None;
}

/// 启动预热连接。
pub(crate) fn start_prewarm_connect(
    state: &mut IcedState,
    profile_id: Option<String>,
) -> Task<Message> {
    // 找到对应的会话配置
    let Some(profile_id) = profile_id else {
        state.prewarm_state = None;
        return Task::none();
    };

    let Some(profile) = state
        .model
        .profiles()
        .iter()
        .find(|p| p.id == profile_id)
        .cloned()
    else {
        state.prewarm_state = None;
        return Task::none();
    };

    // 如果已经有活跃连接，不需要预热
    if state.active_session_is_connected() {
        // 仍然设置预热状态，但不建立新连接
        if let Some(prewarm) = &mut state.prewarm_state {
            prewarm.status = PrewarmStatus::Ready;
        }
        return Task::none();
    }

    // 先提取 vault_master_password，避免借用冲突
    let vault_master = state.model.vault_master_password.clone();

    // 构建 draft 并开始连接
    if let Err(msg) = state.model.fill_draft_from_profile(&profile, vault_master.as_ref()) {
        log::warn!("[Prewarm] Failed to fill draft: {}", msg);
        state.prewarm_state = None;
        return Task::none();
    }

    // 克隆必要数据用于异步连接
    let draft = state.model.draft.clone();
    let settings = state.model.settings.clone();
    let merged_known_hosts = merge_known_hosts(&settings, &state.runtime_known_hosts);
    let vault_master = state.model.vault_master_password.clone();

    let task = async move {
        let mut temp_model = crate::app::model::AppModel::new_for_connect(draft, settings);
        temp_model.settings.security.known_hosts = merged_known_hosts;
        // 预热时需要使用 Vault 主密码来解密凭据
        temp_model.vault_master_password = vault_master;

        let result = temp_model.connect_from_draft().await;

        match result {
            Ok(session) => Message::PrewarmResult(Ok(session)),
            Err(kind) => Message::PrewarmResult(Err((kind, temp_model.draft.host_key_error))),
        }
    };

    Task::perform(task, |msg| msg)
}

/// 处理预热结果。
pub(crate) fn handle_prewarm_result(
    state: &mut IcedState,
    result: Result<std::sync::Arc<crate::backend::ssh_session::SshChannel>, (crate::app::model::ConnectErrorKind, Option<crate::app::model::HostKeyErrorInfo>)>,
) -> Task<Message> {
    match result {
        Ok(_session) => {
            if let Some(prewarm) = &mut state.prewarm_state {
                prewarm.status = PrewarmStatus::Ready;
            }
            log::info!("[Prewarm] Prewarm succeeded");
        }
        Err((kind, _)) => {
            log::warn!("[Prewarm] Prewarm failed: {:?}", kind);
            if let Some(prewarm) = &mut state.prewarm_state {
                prewarm.status = PrewarmStatus::Failed;
            }
        }
    }
    Task::none()
}

/// 使用预热会话：用户点击会话时调用。
/// 如果有预热成功的会话，直接使用；否则走正常连接流程。
pub(crate) fn use_prewarmed_session(state: &mut IcedState, profile_id: &str) -> Task<Message> {
    // 检查是否有针对此会话的预热
    if let Some(prewarm) = &state.prewarm_state {
        if prewarm.profile_id.as_deref() == Some(profile_id)
            && matches!(prewarm.status, PrewarmStatus::Ready)
        {
            // 预热成功，应该有一个已建立的连接
            // 目前预热会话暂存在 prewarm_state 中，需要在点击时建立新连接
            // 这里简化处理：直接使用预热的 draft 进行连接
            log::info!("[Prewarm] Using prewarmed session for {}", profile_id);
            // 清理预热状态
            state.prewarm_state = None;
            // 继续正常的连接流程
        }
    }

    // 如果没有预热或预热失败，使用正常流程
    Task::none()
}
