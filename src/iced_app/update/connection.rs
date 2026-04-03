use iced::Task;
use secrecy::ExposeSecret;

use crate::backend::ssh_session::AsyncSession;

use super::super::message::Message;
use super::super::state::{ConnectionStage, IcedState, QuickConnectFlow};

/// Handle ConnectPressed message - core connection logic.
pub(crate) fn handle_connect(state: &mut IcedState) -> Task<Message> {
    // User required gate (NeedUser).
    if state.model.draft.host.trim().is_empty() || state.model.draft.user.trim().is_empty() {
        state.quick_connect_flow = QuickConnectFlow::NeedUser;
        state.quick_connect_error_kind =
            Some(crate::app_model::ConnectErrorKind::MissingHostOrUser);
        return Task::none();
    }

    // Password lockout: stop repeated wrong passwords within the same draft context.
    if matches!(state.model.draft.auth, crate::session::AuthMethod::Password)
        && state.model.draft.password_error_count >= 3
    {
        state.quick_connect_flow = QuickConnectFlow::AuthLocked;
        state.quick_connect_error_kind = Some(crate::app_model::ConnectErrorKind::AuthFailed);
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

/// Handle interactive auth (synchronous - requires multi-step state).
fn handle_interactive_auth(state: &mut IcedState) -> Task<Message> {
    let draft = &state.model.draft;
    let host = draft.host.trim();
    let port = draft.port.trim();
    let user = draft.user.trim();

    // 收集连接信息行
    let mut lines: Vec<String> = Vec::new();
    let connecting_msg = state.model.i18n.tr("iced.term.ssh.connecting")
        .replace("{user}", user)
        .replace("{host}", host)
        .replace("{port}", port);
    lines.push(connecting_msg);

    if let Some(rec) = state.model.settings.security.known_hosts.iter().find(|r| r.host == host) {
        let fp_msg = state.model.i18n.tr("iced.term.ssh.host_fingerprint")
            .replace("{algo}", &rec.algo)
            .replace("{fp}", &rec.fingerprint);
        lines.push(fp_msg);
    }

    let auth_name = state.model.i18n.tr("iced.auth.keyboard_interactive");
    let auth_msg = state.model.i18n.tr("iced.term.ssh.auth_method")
        .replace("{method}", auth_name);
    lines.push(auth_msg);

    let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    state.active_pane_mut().terminal.inject_local_lines(&line_refs);
    state.preconnect_info_line_count = lines.len();

    state.quick_connect_flow = QuickConnectFlow::Connecting;
    state.quick_connect_error_kind = None;
    state.connection_stage = ConnectionStage::SshConnecting;

    let host = state.model.draft.host.trim().to_string();
    let user = state.model.draft.user.trim().to_string();
    let port: u16 = state.model.draft.port.trim().parse().unwrap_or(22);
    let known_hosts = state.model.settings.security.known_hosts.clone();

    let sess = match state.rt.block_on(
        crate::backend::ssh_session::InteractiveAuthSession::connect(&host, port, &user, &known_hosts)
    ) {
        Ok(s) => s,
        Err(_e) => {
            state.quick_connect_flow = QuickConnectFlow::Failed;
            state.quick_connect_error_kind = Some(crate::app_model::ConnectErrorKind::HostUnreachable);
            state.connection_stage = ConnectionStage::None;
            return Task::none();
        }
    };

    state.connection_stage = ConnectionStage::Authenticating;
    let (sess, step) = match state.rt.block_on(sess.start()) {
        Ok(v) => v,
        Err(_e) => {
            state.quick_connect_flow = QuickConnectFlow::Failed;
            state.quick_connect_error_kind = Some(crate::app_model::ConnectErrorKind::Unknown);
            state.connection_stage = ConnectionStage::None;
            return Task::none();
        }
    };

    match step {
        crate::backend::ssh_session::KeyboardInteractiveStep::Success => {
            match state.rt.block_on(sess.finish_into_session()) {
                Ok(ssh_sess) => {
                    let session: Box<dyn AsyncSession> = Box::new(ssh_sess);
                    handle_connect_success(state, session);
                    Task::none()
                }
                Err(_e) => {
                    state.quick_connect_flow = QuickConnectFlow::Failed;
                    state.quick_connect_error_kind = Some(crate::app_model::ConnectErrorKind::Unknown);
                    state.connection_stage = ConnectionStage::None;
                    Task::none()
                }
            }
        }
        crate::backend::ssh_session::KeyboardInteractiveStep::Failure => {
            state.quick_connect_flow = QuickConnectFlow::Failed;
            state.quick_connect_error_kind = Some(crate::app_model::ConnectErrorKind::AuthFailed);
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
    let draft = state.model.draft.clone();
    let host = draft.host.trim().to_string();
    let settings = state.model.settings.clone();
    let i18n = state.model.i18n.clone();

    // Merge persisted known hosts and runtime overrides ("accept once").
    let mut merged_known_hosts = settings.security.known_hosts.clone();
    for r in &state.runtime_known_hosts {
        if !merged_known_hosts
            .iter()
            .any(|x| x.host == r.host && x.port == r.port)
        {
            merged_known_hosts.push(r.clone());
        }
    }

    // Collect connection info lines for display
    let mut lines: Vec<String> = Vec::new();

    // 1. Connection target
    let connecting_msg = i18n
        .tr("iced.term.ssh.connecting")
        .replace("{user}", &draft.user)
        .replace("{host}", &host)
        .replace("{port}", &draft.port);
    lines.push(connecting_msg);

    // 2. Host fingerprint (if known)
    if let Some(rec) = merged_known_hosts.iter().find(|r| r.host == host) {
        let fp_msg = i18n
            .tr("iced.term.ssh.host_fingerprint")
            .replace("{algo}", &rec.algo)
            .replace("{fp}", &rec.fingerprint);
        lines.push(fp_msg);
    }

    // 3. Auth method
    let auth_name = match &draft.auth {
        crate::session::AuthMethod::Password => i18n.tr("iced.auth.password"),
        crate::session::AuthMethod::Key { .. } => i18n.tr("iced.auth.public_key"),
        crate::session::AuthMethod::Interactive => i18n.tr("iced.auth.keyboard_interactive"),
        crate::session::AuthMethod::Agent => i18n.tr("iced.auth.agent"),
    };
    let auth_msg = i18n
        .tr("iced.term.ssh.auth_method")
        .replace("{method}", auth_name);
    lines.push(auth_msg);

    // Record line count for cleanup
    let line_count = lines.len();
    let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();

    // Display connection info in terminal
    state.active_pane_mut().terminal.inject_local_lines(&line_refs);
    state.preconnect_info_line_count = line_count;
    state.quick_connect_flow = QuickConnectFlow::Connecting;
    state.quick_connect_error_kind = None;

    // Set initial connection stage
    state.connection_stage = if state.model.vault_master_password.is_some() {
        ConnectionStage::VaultLoading
    } else {
        ConnectionStage::SshConnecting
    };

    // Build async task - this will run without blocking the UI thread
    let task = async move {
        // Create a temporary AppModel for the async connection
        let mut temp_model = crate::app_model::AppModel::new_for_connect(draft, settings);
        temp_model.settings.security.known_hosts = merged_known_hosts;

        // Perform the connection
        let result = temp_model.connect_from_draft().await;

        match result {
            Ok(session) => Ok(std::sync::Arc::new(session)),
            Err(kind) => Err(kind),
        }
    };

    Task::perform(task, Message::ConnectResult)
}

/// Internal error handler (returns Task::none).
pub(crate) fn internal_handle_connect_error(state: &mut IcedState, e: crate::app_model::ConnectErrorKind) {
    state.connection_stage = ConnectionStage::None;
    if e == crate::app_model::ConnectErrorKind::AuthFailed
        && matches!(state.model.draft.auth, crate::session::AuthMethod::Password)
    {
        state.model.draft.password_error_count =
            state.model.draft.password_error_count.saturating_add(1);
        if state.model.draft.password_error_count >= 3 {
            state.quick_connect_flow = QuickConnectFlow::AuthLocked;
            state.quick_connect_error_kind = Some(e);
            return;
        }
    }

    if e == crate::app_model::ConnectErrorKind::AuthFailed
        && matches!(state.model.draft.auth, crate::session::AuthMethod::Password)
    {
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
fn internal_handle_host_key_error(state: &mut IcedState, e: &crate::app_model::ConnectErrorKind) {
    if matches!(
        e,
        crate::app_model::ConnectErrorKind::HostKeyUnknown
            | crate::app_model::ConnectErrorKind::HostKeyChanged
    ) {
        if let Some(info) = state.model.draft.host_key_error.clone() {
            match state.model.settings.security.host_key_policy {
                crate::settings::HostKeyPolicy::AcceptNew
                    if e == &crate::app_model::ConnectErrorKind::HostKeyUnknown =>
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
            crate::app_model::DraftSource::DirectInput
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

    super::super::update::complete_new_ssh_session(state, session, recent, label, profile_id);
    state.quick_connect_flow = QuickConnectFlow::Connected;
    state.quick_connect_error_kind = None;
    state.connection_stage = ConnectionStage::None;

    // 显示连接成功信息
    let msg = state.model.i18n.tr("iced.term.connected");
    state.active_pane_mut().terminal.inject_local_lines(&[msg]);

    // 清理预连接信息行
    let lines_to_clear = state.preconnect_info_line_count;
    if lines_to_clear > 0 {
        state.active_pane_mut().terminal.clear_preconnect_lines(lines_to_clear);
    }
    state.preconnect_info_line_count = 0;

    // 立即 pump 一次，读取 MOTD
    let active_tab = state.active_tab;
    let pane = &mut state.tab_panes[active_tab];
    if let Some(sess) = state.session_manager.session_mut(active_tab) {
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
                if let Ok(Some(cid)) =
                    crate::vault::session_credentials::sync_ssh_credentials_with_master(
                        &state.model.settings,
                        master,
                        &pid,
                        Some(&state.model.draft.password),
                        None,
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
                                    state.model.session_manager.upsert_session(updated),
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
    state.active_pane_mut().last_terminal_focus_sent = None;
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
        state.quick_connect_open = false;
        state.quick_connect_panel = super::super::state::QuickConnectPanel::Picker;
        let a = state.model.i18n.tr("iced.term.vault_needed");
        let b = state.model.i18n.tr("iced.term.vault_unlock_to_continue");
        state.active_pane_mut().terminal.inject_local_lines(&[a, b]);
        return super::super::update::update(state, Message::VaultUnlockOpenConnect(profile));
    }

    let master = state.model.vault_master_password.clone();
    match state.model.fill_draft_from_profile(&profile, master.as_ref()) {
        Ok(()) => {
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

            if can_auto_connect {
                // 直接连接，不打开弹窗
                return super::super::update::update(state, Message::ConnectPressed);
            }

            // 需要用户输入时才打开弹窗
            state.quick_connect_open = true;
            state.quick_connect_panel = super::super::state::QuickConnectPanel::NewConnection;
        }
        Err(msg) => {
            state.model.status = msg;
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
            state.quick_connect_error_kind = Some(crate::app_model::ConnectErrorKind::Unknown);
            state.connection_stage = ConnectionStage::None;
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
                    let label = format!("{user}@{host}");
                    let profile_id = state.model.selected_session_id.clone();
                    let recent = state
                        .model
                        .recent_record_for_draft_with_profile(label.clone(), profile_id.clone());
                    state.connection_stage = ConnectionStage::SessionSetup;
                    super::super::update::complete_new_ssh_session(
                        state, session, recent, label, profile_id,
                    );
                    state.quick_connect_flow = QuickConnectFlow::Connected;
                    state.quick_connect_error_kind = None;
                    state.connection_stage = ConnectionStage::None;

                    // 显示连接成功并清理预连接信息
                    let msg = state.model.i18n.tr("iced.term.connected");
                    state.active_pane_mut().terminal.inject_local_lines(&[msg]);
                    let lines_to_clear = state.preconnect_info_line_count;
                    if lines_to_clear > 0 {
                        state.active_pane_mut().terminal.clear_preconnect_lines(lines_to_clear);
                    }
                    state.preconnect_info_line_count = 0;

                    let active_tab = state.active_tab;
                    let pane = &mut state.tab_panes[active_tab];
                    if let Some(sess) = state.session_manager.session_mut(active_tab) {
                        let _ = pane.terminal.pump_output(sess);
                    }
                }
                Err(_e) => {
                    state.quick_connect_flow = QuickConnectFlow::Failed;
                    state.quick_connect_error_kind = Some(crate::app_model::ConnectErrorKind::Unknown);
                    state.connection_stage = ConnectionStage::None;
                }
            }
        }
        crate::backend::ssh_session::KeyboardInteractiveStep::Failure => {
            state.quick_connect_flow = QuickConnectFlow::Failed;
            state.quick_connect_error_kind = Some(crate::app_model::ConnectErrorKind::AuthFailed);
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
