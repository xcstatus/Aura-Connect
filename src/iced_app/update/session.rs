use iced::Task;

use secrecy::{ExposeSecret, SecretString};

use super::super::message::Message;
use super::super::state::{IcedState, IcedTab, SessionEditorState, TabPane, VaultStatus};

/// Handle TopAddTab message.
pub(crate) fn handle_add_tab(state: &mut IcedState) -> Task<Message> {
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

/// Handle TabSelected message.
pub(crate) fn handle_tab_selected(state: &mut IcedState, i: usize) -> Task<Message> {
    if i >= state.tabs.len() {
        return Task::none();
    }

    let old = state.active_tab;

    // Explicit DEC 1004 transition when switching sessions.
    if old != i {
        super::super::terminal_host::TerminalHost::sync_focus_report_for_tab(state, old, false);
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
        state.model.status = "Disconnected previous tab (single-session mode)".to_string();
    }

    state.active_tab = i;

    if let Some(pid) = state.tabs[i].profile_id.clone() {
        state.model.select_profile(pid);
    }

    super::super::update::sync_terminal_grid_to_session(state);
    super::super::terminal_host::TerminalHost::sync_focus_report(state);
    Task::none()
}

/// Handle TabClose message.
pub(crate) fn handle_tab_close(state: &mut IcedState, i: usize) -> Task<Message> {
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

    super::super::update::sync_terminal_grid_to_session(state);
    super::super::terminal_host::TerminalHost::sync_focus_report(state);
    Task::none()
}

/// Handle TabChipHover message.
pub(crate) fn handle_tab_chip_hover(state: &mut IcedState, ix: Option<usize>) -> Task<Message> {
    state.tab_hover_index = ix;
    Task::none()
}

/// Handle OpenSessionEditor message.
pub(crate) fn handle_open_session_editor(
    state: &mut IcedState,
    profile_id: Option<String>,
) -> Task<Message> {
    let mut st = SessionEditorState {
        profile_id: profile_id.clone(),
        host: String::new(),
        port: "22".to_string(),
        user: String::new(),
        auth: crate::session::AuthMethod::Password,
        password: SecretString::new("".into()),
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

/// Handle SessionEditorClose message.
pub(crate) fn handle_session_editor_close(state: &mut IcedState) -> Task<Message> {
    state.session_editor = None;
    Task::none()
}

/// Handle SessionEditorHostChanged message.
pub(crate) fn handle_session_editor_host(state: &mut IcedState, v: String) -> Task<Message> {
    if let Some(ed) = state.session_editor.as_mut() {
        ed.host = v;
        ed.error = None;
    }
    Task::none()
}

/// Handle SessionEditorPortChanged message.
pub(crate) fn handle_session_editor_port(state: &mut IcedState, v: String) -> Task<Message> {
    if let Some(ed) = state.session_editor.as_mut() {
        ed.port = v;
        ed.error = None;
    }
    Task::none()
}

/// Handle SessionEditorUserChanged message.
pub(crate) fn handle_session_editor_user(state: &mut IcedState, v: String) -> Task<Message> {
    if let Some(ed) = state.session_editor.as_mut() {
        ed.user = v;
        ed.error = None;
    }
    Task::none()
}

/// Handle SessionEditorAuthChanged message.
pub(crate) fn handle_session_editor_auth(
    state: &mut IcedState,
    v: crate::session::AuthMethod,
) -> Task<Message> {
    if let Some(ed) = state.session_editor.as_mut() {
        ed.auth = v;
        ed.error = None;
    }
    Task::none()
}

/// Handle SessionEditorPasswordChanged message.
pub(crate) fn handle_session_editor_password(state: &mut IcedState, v: String) -> Task<Message> {
    if let Some(ed) = state.session_editor.as_mut() {
        ed.password = SecretString::from(v);
        ed.password_dirty = true;
        ed.error = None;
    }
    Task::none()
}

/// Handle SessionEditorClearPasswordToggled message.
pub(crate) fn handle_session_editor_clear_password(
    state: &mut IcedState,
    v: bool,
) -> Task<Message> {
    if let Some(ed) = state.session_editor.as_mut() {
        ed.clear_saved_password = v;
        ed.error = None;
    }
    Task::none()
}

/// Handle SessionEditorSave message.
pub(crate) fn handle_session_editor_save(state: &mut IcedState) -> Task<Message> {
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
            return super::super::update::update(state, Message::VaultUnlockOpenSaveSession);
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

    let res = state.rt.block_on(state.model.session_manager.upsert_session(profile));

    state.model.status = match res {
        Ok(()) => "Session saved".to_string(),
        Err(e) => format!("Save failed: {e}"),
    };
    state.session_editor = None;
    Task::none()
}

/// Handle DeleteSessionProfile message.
pub(crate) fn handle_delete_session(state: &mut IcedState, id: String) -> Task<Message> {
    // Cleanup for SSH credential stored in vault (requires runtime unlock when vault is initialized).
    let mut vault_cleanup_err: Option<String> = None;
    if let Some(p) = state.model.profiles().iter().find(|p| p.id == id) {
        if let crate::session::TransportConfig::Ssh(ssh) = &p.transport {
            if let Some(cid) = ssh.credential_id.as_deref() {
                if state.model.settings.security.vault.is_some()
                    && state.model.vault_master_password.is_none()
                {
                    return super::super::update::update(state, Message::VaultUnlockOpenDelete(id));
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

    let res = state.rt.block_on(state.model.session_manager.delete_session(&id));

    state.model.status = match res {
        Ok(()) => vault_cleanup_err.unwrap_or_else(|| {
            state
                .model
                .i18n
                .tr("iced.settings.conn.deleted")
                .to_string()
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
