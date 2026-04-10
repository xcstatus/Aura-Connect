use iced::Task;

use secrecy::{ExposeSecret, SecretString};

use super::super::message::Message;
use super::super::state::{IcedState, IcedTab, ProxyType, SessionEditorState, TabPane, VaultStatus};

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
        .push(TabPane::new(&state.model.settings.terminal, &state.model.settings.color_scheme));
    state.tab_manager.add_tab();
    state.active_tab = state.tabs.len() - 1;
    // Start new tab width animation (expands from 0).
    state.tab_anims.push(crate::app::state::TabAnimEntry::new(crate::app::chrome::TAB_CHIP_WIDTH, state.tick_count));
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

    state.tab_manager.set_active_tab(i);
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
    // Also remove from session manager (returns the dropped session, if any).
    let _ = state.tab_manager.remove_tab(i);

    let len = state.tabs.len();
    if was_active {
        state.active_tab = state.active_tab.min(len.saturating_sub(1));
    } else if i < state.active_tab {
        state.active_tab -= 1;
    }
    state.tab_manager.set_active_tab(state.active_tab);

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

/// Handle TabScrollTo message: select tab and scroll it into view.
pub(crate) fn handle_tab_scroll_to(state: &mut IcedState, i: usize) -> Task<Message> {
    use crate::app::chrome::tab_strip_width;
    if i >= state.tabs.len() {
        return Task::none();
    }
    state.active_tab = i;
    let available_w = tab_strip_width(state.window_size.width);
    state.scroll_to_tab(i, available_w);
    Task::none()
}

/// Handle TabOverflowToggle message.
pub(crate) fn handle_tab_overflow_toggle(state: &mut IcedState) -> Task<Message> {
    state.tab_overflow_open = !state.tab_overflow_open;
    Task::none()
}

/// Handle OpenSessionEditor message.
pub(crate) fn handle_open_session_editor(
    state: &mut IcedState,
    profile_id: Option<String>,
) -> Task<Message> {
    let mut st = SessionEditorState {
        profile_id: profile_id.clone(),
        name: String::new(),
        group_id: None,
        host: String::new(),
        port: "22".to_string(),
        user: String::new(),
        auth: crate::session::AuthMethod::Password,
        password: SecretString::new("".into()),
        existing_credential_id: None,
        password_dirty: false,
        clear_saved_password: false,
        error: None,
        private_key_path: String::new(),
        passphrase: SecretString::new("".into()),
        test_connecting: false,
        keep_alive_interval: 60,
        connection_timeout: 10,
        proxy_type: ProxyType::None,
        proxy_host: String::new(),
        proxy_port: String::new(),
        heartbeat_enabled: false,
        heartbeat_interval: 30,
    };

    if let Some(pid) = profile_id {
        if let Some(p) = state.model.profiles().iter().find(|p| p.id == pid) {
            st.name = p.name.clone();
            st.group_id = p.group_id.clone();
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

/// Handle SessionEditorNameChanged message.
pub(crate) fn handle_session_editor_name(state: &mut IcedState, v: String) -> Task<Message> {
    if let Some(ed) = state.session_editor.as_mut() {
        ed.name = v;
        ed.error = None;
    }
    Task::none()
}

/// Handle SessionEditorGroupChanged message.
pub(crate) fn handle_session_editor_group(
    state: &mut IcedState,
    v: Option<String>,
) -> Task<Message> {
    if let Some(ed) = state.session_editor.as_mut() {
        ed.group_id = v;
        ed.error = None;
    }
    Task::none()
}

/// Handle SessionEditorPrivateKeyPathChanged message.
pub(crate) fn handle_session_editor_private_key_path(
    state: &mut IcedState,
    v: String,
) -> Task<Message> {
    if let Some(ed) = state.session_editor.as_mut() {
        ed.private_key_path = v;
        ed.error = None;
    }
    Task::none()
}

/// Handle SessionEditorPassphraseChanged message.
pub(crate) fn handle_session_editor_passphrase(
    state: &mut IcedState,
    v: String,
) -> Task<Message> {
    if let Some(ed) = state.session_editor.as_mut() {
        ed.passphrase = SecretString::from(v);
        ed.error = None;
    }
    Task::none()
}

/// Handle SessionEditorTestConnection message.
pub(crate) fn handle_session_editor_test_connection(
    state: &mut IcedState,
) -> Task<Message> {
    let Some(ed) = state.session_editor.as_ref() else {
        return Task::none();
    };

    // TODO: 实现测试连接逻辑
    // 目前只是设置测试连接状态
    if let Some(ed) = state.session_editor.as_mut() {
        ed.test_connecting = true;
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
    let mut credential_id = ed.existing_credential_id.clone();

    // Credential lifecycle policy:
    // - If user did NOT touch password and did NOT explicitly clear -> keep existing credential_id unchanged.
    // - If user explicitly clears -> delete credential (requires runtime unlock).
    // - If user edited password and provided non-empty -> write/overwrite credential (requires runtime unlock).
    let needs_vault_op = ed.clear_saved_password
        || (ed.password_dirty
            && matches!(auth, crate::session::AuthMethod::Password)
            && !ed.password.expose_secret().trim().is_empty());

    if needs_vault_op {
        if state.model.settings.security.vault.is_none() {
            ed.error = Some("请先在设置中初始化 Vault，再保存密码凭据".to_string());
            return Task::none();
        }
        if state.model.vault_master_password.is_none()
            && state.model.settings.security.vault.is_some()
        {
            return super::super::update::update(state, Message::VaultUnlockOpenSaveSession);
        }
        let Some(master) = state.model.vault_master_password.as_ref() else {
            ed.error = Some("Vault 未解锁".to_string());
            return Task::none();
        };
        if ed.clear_saved_password {
            if let Some(cid) = ed.existing_credential_id.as_deref() {
                if let Err(e) = crate::vault::session_credentials::delete_credential(
                    master.expose_secret(),
                    cid,
                    state.model.settings.security.kdf_memory_level,
                ) {
                    ed.error = Some(format!("清理凭据失败：{e}"));
                    return Task::none();
                }
            }
            credential_id = None;
        } else {
            let pw = ed.password.expose_secret().trim();
            match crate::vault::session_credentials::sync_ssh_credentials(
                master.expose_secret(),
                &id,
                Some(pw),
                None,
                state.model.settings.security.kdf_memory_level,
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
        name: if ed.name.is_empty() {
            format!("{}@{}", user, host)
        } else {
            ed.name.clone()
        },
        group_id: ed.group_id.clone(),
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

    let res = state.rt.block_on(state.model.tab_manager.upsert_session(profile));

    state.model.status = match res {
        Ok(()) => "Session saved".to_string(),
        Err(e) => format!("Save failed: {e}"),
    };
    state.session_editor = None;
    Task::none()
}

/// Handle QuickConnectSaveSession message.
/// Saves the Quick Connect draft as a new session.
pub(crate) fn handle_quick_connect_save_session(state: &mut IcedState) -> Task<Message> {
    let host = state.model.draft.host.trim().to_string();
    let user = state.model.draft.user.trim().to_string();
    let port_ok = state
        .model
        .draft
        .port
        .trim()
        .parse::<u32>()
        .ok()
        .is_some_and(|p| p >= 1 && p <= 65535);

    if host.is_empty() || user.is_empty() {
        state.model.status = "Host 和 User 为必填项".to_string();
        return Task::none();
    }
    if !port_ok {
        state.model.status = "端口范围 1–65535".to_string();
        return Task::none();
    }

    let port: u16 = state.model.draft.port.trim().parse().unwrap_or(22);
    let auth = state.model.draft.auth.clone();
    let password = state.model.draft.password.expose_secret().to_string();
    let profile_id = uuid::Uuid::new_v4().to_string();

    // Check if password needs to be saved to vault
    let password_non_empty = !password.trim().is_empty();
    let needs_vault = matches!(auth, crate::session::AuthMethod::Password) && password_non_empty;

    if needs_vault {
        if state.model.settings.security.vault.is_none() {
            state.model.status = "请先在设置中初始化 Vault，再保存密码凭据".to_string();
            return Task::none();
        }
        if state.model.vault_master_password.is_none() {
            // Store pending operation and open vault unlock dialog
            state.vault_unlock = Some(super::super::state::VaultUnlockState {
                pending_connect: None,
                pending_delete_profile_id: None,
                pending_save_session: false,
                pending_save_credentials_profile_id: Some(profile_id.clone()),
                password: secrecy::SecretString::from(String::new()),
                error: None,
            });
            state.model.status = "需要解锁 Vault 以保存密码".to_string();
            return Task::none();
        }
        // Save password to vault
        let master = state.model.vault_master_password.as_ref().unwrap();
        match crate::vault::session_credentials::sync_ssh_credentials(
            master.expose_secret(),
            &profile_id,
            Some(&*password),
            None,
            state.model.settings.security.kdf_memory_level,
        ) {
            Ok(_) => {
                state.vault_status = VaultStatus::compute(
                    &state.model.settings,
                    true,
                );
            }
            Err(e) => {
                state.model.status = format!("保存密码失败：{e}");
                return Task::none();
            }
        }
    }

    let profile = crate::session::SessionProfile {
        id: profile_id.clone(),
        name: format!("{}@{}", user, host),
        group_id: None,
        folder: None,
        color_tag: None,
        transport: crate::session::TransportConfig::Ssh(crate::session::SshConfig {
            host,
            port,
            user,
            auth,
            credential_id: if needs_vault { Some(profile_id.clone()) } else { None },
        }),
    };

    let res = state.rt.block_on(state.model.tab_manager.upsert_session(profile));
    state.model.status = match res {
        Ok(()) => "Session saved".to_string(),
        Err(e) => format!("Save failed: {e}"),
    };
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
                if let Some(ref master) = state.model.vault_master_password {
                    if let Err(e) = crate::vault::session_credentials::delete_credential(
                        master.expose_secret(),
                        cid,
                        state.model.settings.security.kdf_memory_level,
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

    let res = state.rt.block_on(state.model.tab_manager.delete_session(&id));

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
