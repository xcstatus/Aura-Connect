use iced::Task;

use secrecy::{ExposeSecret, SecretString};

use super::super::message::Message;
use super::super::state::{IcedState, VaultFlowMode, VaultFlowState, VaultStatus};

/// Handle VaultOpen message.
pub(crate) fn handle_vault_open(state: &mut IcedState) -> Task<Message> {
    let mode = if state.model.settings.security.vault.is_some() {
        VaultFlowMode::ChangePassword
    } else {
        VaultFlowMode::Initialize
    };
    state.vault_flow = Some(VaultFlowState::new(mode));
    Task::none()
}

/// Handle VaultClose message.
pub(crate) fn handle_vault_close(state: &mut IcedState) -> Task<Message> {
    state.vault_flow = None;
    Task::none()
}

/// Handle VaultOldPasswordChanged message.
pub(crate) fn handle_vault_old_password(state: &mut IcedState, v: String) -> Task<Message> {
    if let Some(flow) = state.vault_flow.as_mut() {
        flow.old_password = SecretString::from(v);
        flow.error = None;
    }
    Task::none()
}

/// Handle VaultNewPasswordChanged message.
pub(crate) fn handle_vault_new_password(state: &mut IcedState, v: String) -> Task<Message> {
    if let Some(flow) = state.vault_flow.as_mut() {
        flow.new_password = SecretString::from(v);
        flow.error = None;
    }
    Task::none()
}

/// Handle VaultConfirmPasswordChanged message.
pub(crate) fn handle_vault_confirm_password(state: &mut IcedState, v: String) -> Task<Message> {
    if let Some(flow) = state.vault_flow.as_mut() {
        flow.confirm_password = SecretString::from(v);
        flow.error = None;
    }
    Task::none()
}

/// Handle VaultSubmit message.
pub(crate) fn handle_vault_submit(state: &mut IcedState) -> Task<Message> {
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
        let old_pwd = SecretString::from(flow.old_password.expose_secret().to_string());
        if !crate::vault::VaultManager::verify_password(&old_pwd, old_meta) {
            flow.error = Some("旧密码校验失败".to_string());
            return Task::none();
        }
    }

    let new_password = SecretString::from(new_pwd);
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

    let new_secret = SecretString::from(new_meta.verifier_hash.clone());
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
            let old_secret = SecretString::from(old_meta.verifier_hash.clone());
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
    state.model.settings.save_with_log();
    // Unlock with the new derived secret for this runtime.
    state.model.vault_master_password = Some(SecretString::from(
        state
            .model
            .settings
            .security
            .vault
            .as_ref()
            .unwrap()
            .verifier_hash
            .clone(),
    ));
    state.vault_status = VaultStatus::compute(
        &state.model.settings,
        state.model.vault_master_password.is_some(),
    );

    state.vault_flow = None;
    state.model.status = "Vault 已更新".to_string();
    Task::none()
}

/// Handle VaultUnlockOpenConnect message.
pub(crate) fn handle_vault_unlock_open_connect(
    state: &mut IcedState,
    profile: crate::session::SessionProfile,
) -> Task<Message> {
    state.vault_unlock = Some(super::super::state::VaultUnlockState {
        pending_connect: Some(profile),
        pending_delete_profile_id: None,
        pending_save_session: false,
        pending_save_credentials_profile_id: None,
        password: SecretString::from(String::new()),
        error: None,
    });
    Task::none()
}

/// Handle VaultUnlockOpenDelete message.
pub(crate) fn handle_vault_unlock_open_delete(state: &mut IcedState, id: String) -> Task<Message> {
    state.vault_unlock = Some(super::super::state::VaultUnlockState {
        pending_connect: None,
        pending_delete_profile_id: Some(id),
        pending_save_session: false,
        pending_save_credentials_profile_id: None,
        password: SecretString::from(String::new()),
        error: None,
    });
    Task::none()
}

/// Handle VaultUnlockOpenSaveSession message.
pub(crate) fn handle_vault_unlock_open_save_session(state: &mut IcedState) -> Task<Message> {
    state.vault_unlock = Some(super::super::state::VaultUnlockState {
        pending_connect: None,
        pending_delete_profile_id: None,
        pending_save_session: true,
        pending_save_credentials_profile_id: None,
        password: SecretString::from(String::new()),
        error: None,
    });
    Task::none()
}

/// Handle VaultUnlockClose message.
pub(crate) fn handle_vault_unlock_close(state: &mut IcedState) -> Task<Message> {
    state.vault_unlock = None;
    Task::none()
}

/// Handle VaultUnlockPasswordChanged message.
pub(crate) fn handle_vault_unlock_password(state: &mut IcedState, v: String) -> Task<Message> {
    if let Some(u) = state.vault_unlock.as_mut() {
        u.password = SecretString::from(v);
        u.error = None;
    }
    Task::none()
}

/// Handle VaultUnlockSubmit message.
pub(crate) fn handle_vault_unlock_submit(state: &mut IcedState) -> Task<Message> {
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
    state.model.vault_master_password = Some(SecretString::from(meta.verifier_hash.clone()));

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
        return super::super::update::update(state, Message::SessionEditorSave);
    }

    if let Some(id) = pending_delete {
        return super::super::update::update(state, Message::DeleteSessionProfile(id));
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
                if let Some(existing) = state.model.profiles().iter().find(|p| p.id == pid).cloned()
                {
                    if let crate::session::TransportConfig::Ssh(mut ssh) = existing.transport {
                        ssh.credential_id = Some(cid);
                        let updated = crate::session::SessionProfile {
                            transport: crate::session::TransportConfig::Ssh(ssh),
                            ..existing
                        };
                        let _ = state.rt.block_on(state.model.session_manager.upsert_session(updated));
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
        // 关闭弹窗，直接在终端显示连接信息
        state.quick_connect_open = false;
        state.vault_unlock = None;

        // 清除终端并显示解锁消息
        let a = state.model.i18n.tr("iced.term.vault_unlocked");
        let b = state.model.i18n.tr("iced.term.connecting");
        {
            let pane = state.active_pane_mut();
            pane.terminal.clear_local_preconnect_ui();
            pane.terminal.inject_local_lines(&[a, b]);
        }

        // 填充 draft 并直接触发连接（不打开弹窗）
        let master = state.model.vault_master_password.clone();
        if state.model.fill_draft_from_profile(&prof, master.as_ref()).is_ok() {
            return super::super::update::update(state, Message::ConnectPressed);
        }
    }

    Task::none()
}
