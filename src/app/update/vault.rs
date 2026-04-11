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
        flow.error = Some(state.model.i18n.tr("iced.vault.error.password_mismatch").to_string());
        return Task::none();
    }

    if let VaultFlowMode::ChangePassword = flow.mode {
        let Some(old_meta) = state.model.settings.security.vault.as_ref() else {
            flow.error = Some(state.model.i18n.tr("iced.vault_unlock.error.vault_not_initialized").to_string());
            return Task::none();
        };
        let old_pwd = SecretString::from(flow.old_password.expose_secret().to_string());
        if !crate::vault::VaultManager::verify_password(&old_pwd, old_meta) {
            flow.error = Some(state.model.i18n.tr("iced.vault.error.old_password_failed").to_string());
            return Task::none();
        }
    }

    let new_password = SecretString::from(new_pwd);
    let new_meta = match crate::vault::VaultManager::setup_vault(&new_password) {
        Ok(m) => m,
        Err(e) => {
            flow.error = Some(format!("{}: {}", state.model.i18n.tr("iced.vault.error.init_failed"), e));
            return Task::none();
        }
    };

    let Some(vault_path) = crate::utils::StorageManager::get_vault_path() else {
        flow.error = Some(state.model.i18n.tr("iced.vault.error.vault_path_not_found").to_string());
        return Task::none();
    };

    // Derive encryption salt from VaultMeta and initialize vault with user's password.
    let enc_salt_str = new_meta.encryption_salt.as_deref().unwrap_or_default();
    let enc_salt_bytes = enc_salt_str.as_bytes();
    let mut enc_salt_arr = [0u8; 16];
    let salt_len = enc_salt_bytes.len().min(16);
    enc_salt_arr[..salt_len].copy_from_slice(&enc_salt_bytes[..salt_len]);

    // IMPORTANT: do NOT overwrite settings meta unless vault file operation succeeds.
    let vault_ok = match flow.mode {
        VaultFlowMode::Initialize => crate::vault::core::CredentialVault::initialize(&new_password, enc_salt_arr)
            .and_then(|v| v.save_to_file(&vault_path))
            .is_ok(),
        VaultFlowMode::ChangePassword => {
            let Some(old_meta) = state.model.settings.security.vault.as_ref() else {
                flow.error = Some(state.model.i18n.tr("iced.vault_unlock.error.vault_not_initialized").to_string());
                return Task::none();
            };
            if !vault_path.exists() {
                flow.error = Some(state.model.i18n.tr("iced.vault.error.file_lost").to_string());
                return Task::none();
            }
            let old_enc_salt_str = old_meta.encryption_salt.as_deref().unwrap_or_default();
            let old_enc_salt_bytes = old_enc_salt_str.as_bytes();
            let mut old_enc_salt_arr = [0u8; 16];
            let old_salt_len = old_enc_salt_bytes.len().min(16);
            old_enc_salt_arr[..old_salt_len].copy_from_slice(&old_enc_salt_bytes[..old_salt_len]);
            let old_pwd_secret = SecretString::from(flow.old_password.expose_secret().to_string());
            crate::vault::core::CredentialVault::load_from_file(&vault_path)
                .and_then(|mut v| {
                    v.unlock(&old_pwd_secret)?;
                    v.rekey_with_salt(&new_password, enc_salt_arr)?;
                    v.save_to_file(&vault_path)?;
                    Ok(())
                })
                .is_ok()
        }
    };

    if !vault_ok {
        flow.error = Some(state.model.i18n.tr("iced.vault.error.save_failed").to_string());
        return Task::none();
    }

    state.model.settings.security.vault = Some(new_meta);
    state.model.settings.save_with_log();
    // Store the user's actual password for this runtime (not verifier_hash).
    state.model.vault_master_password = Some(new_password);
    state.vault_status = VaultStatus::compute(
        &state.model.settings,
        state.model.vault_master_password.is_some(),
    );

    state.vault_flow = None;
    state.model.status = state.model.i18n.tr("iced.term.vault_unlocked").to_string();
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
/// This now runs vault KDF operations asynchronously to avoid blocking the UI thread.
pub(crate) fn handle_vault_unlock_submit(state: &mut IcedState) -> Task<Message> {
    let Some(u) = state.vault_unlock.take() else {
        return Task::none();
    };

    let Some(meta) = state.model.settings.security.vault.clone() else {
        return Task::none();
    };

    // Close the vault unlock modal immediately for responsive UI
    state.vault_unlock = None;

    // Extract pending credential_id synchronously (SessionProfile cannot cross into spawn_blocking easily)
    let pending_credential_id: Option<String> = u.pending_connect.as_ref().and_then(|prof| {
        if let crate::session::TransportConfig::Ssh(ssh) = &prof.transport {
            ssh.credential_id.clone()
        } else {
            None
        }
    });

    // Store pending context for use after async completion (pending_save_session/delete/save_creds need it)
    state.pending_vault_unlock = Some(super::super::state::PendingVaultUnlock {
        verifier_hash: meta.verifier_hash.clone(),
        pending_connect: u.pending_connect.clone(),
        pending_delete_profile_id: u.pending_delete_profile_id.clone(),
        pending_save_session: u.pending_save_session,
        pending_save_credentials_profile_id: u.pending_save_credentials_profile_id.clone(),
    });

    // Close the vault unlock modal immediately for responsive UI
    state.vault_unlock = None;

    // Capture password and meta for async task
    let password = u.password.expose_secret().to_string();
    let verifier_hash = meta.verifier_hash.clone();
    let meta_salt = meta.salt.clone();
    let enc_salt_str = meta.encryption_salt.clone();

    // Get KDF memory level from settings
    let kdf_level = state.model.settings.security.kdf_memory_level;

    // Run KDF + vault load + credential preload in background to avoid blocking UI
    // We use spawn_blocking to run CPU-intensive KDF operations, and process results in the async block
    let task = async move {
        // Step 1: Verify password using spawn_blocking (CPU-intensive KDF)
        let password_for_verify = password.clone();
        let meta_salt_for_verify = meta_salt.clone();
        let verifier_hash_for_verify = verifier_hash.clone();
        let enc_salt_str_for_verify = enc_salt_str.clone();
        let pending_cred_id = pending_credential_id.clone();

        let verified = tokio::task::spawn_blocking(move || {
            crate::vault::VaultManager::verify_password_with_level(
                &SecretString::from(password_for_verify),
                &crate::vault::VaultMeta {
                    salt: meta_salt_for_verify,
                    verifier_hash: verifier_hash_for_verify,
                    encryption_salt: enc_salt_str_for_verify,
                },
                kdf_level,
            )
        }).await;

        let verified = match verified {
            Ok(v) => v,
            Err(_) => return Message::VaultUnlockComplete(Err(crate::vault::VaultUnlockError::VaultError("Task cancelled".into()))),
        };

        if !verified {
            return Message::VaultUnlockComplete(Err(crate::vault::VaultUnlockError::WrongPassword));
        }

        // Step 2: Load vault file (fast, no KDF)
        let vault_path = match crate::utils::StorageManager::get_vault_path() {
            Some(p) => p,
            None => return Message::VaultUnlockComplete(Err(crate::vault::VaultUnlockError::VaultError("无法定位 vault 路径".into()))),
        };
        let vault_exists = vault_path.exists();

        // Step 3: Unlock vault using spawn_blocking (CPU-intensive KDF)
        let enc_salt_str = enc_salt_str.clone().unwrap_or_default();
        let enc_salt_bytes = enc_salt_str.as_bytes();
        let mut enc_salt_arr = [0u8; 16];
        let salt_len = enc_salt_bytes.len().min(16);
        enc_salt_arr[..salt_len].copy_from_slice(&enc_salt_bytes[..salt_len]);

        // Prepare password for vault unlock (clone before spawn_blocking consumes it)
        let password_for_vault = password.clone();

        let vault_result = tokio::task::spawn_blocking(move || {
            if vault_exists {
                let mut vault = crate::vault::core::CredentialVault::load_from_file(&vault_path)
                    .map_err(|e| crate::vault::VaultUnlockError::VaultError(e.to_string()))?;
                vault.unlock_with_level(&SecretString::from(password_for_vault), kdf_level)
                    .map_err(|e| crate::vault::VaultUnlockError::VaultError(e.to_string()))?;
                Ok(vault)
            } else {
                let vault = crate::vault::core::CredentialVault::initialize_with_level(
                    &SecretString::from(password_for_vault),
                    enc_salt_arr,
                    kdf_level,
                )
                .map_err(|e| crate::vault::VaultUnlockError::VaultError(e.to_string()))?;
                vault.save_to_file(&vault_path)
                    .map_err(|e| crate::vault::VaultUnlockError::VaultError(e.to_string()))?;
                Ok(vault)
            }
        }).await;

        let vault = match vault_result {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => return Message::VaultUnlockComplete(Err(e)),
            Err(_) => return Message::VaultUnlockComplete(Err(crate::vault::VaultUnlockError::VaultError("Task cancelled".into()))),
        };

        // Step 4: Preload SSH credentials for the pending session
        let preloaded = if let Some(ref cid) = pending_cred_id {
            let credential_id = format!("ssh:{}", cid);
            if let Ok(Some(raw)) = vault.get_credential(&credential_id) {
                if let Ok(payload) =
                    serde_json::from_slice::<crate::vault::session_credentials::SshCredentialPayload>(&raw)
                {
                    Some((payload.password, payload.passphrase))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Return master password and preloaded credentials
        let preloaded = preloaded.map(|(pw, pp)| (pw.unwrap_or_default(), pp));
        Message::VaultUnlockComplete(Ok((password, preloaded)))
    };

    Task::perform(task, |msg| msg)
}

/// Handle async vault unlock completion (after background KDF + credential preload).
pub(crate) fn handle_vault_unlock_complete(
    state: &mut IcedState,
    result: Result<(String, Option<(String, Option<String>)>), crate::vault::VaultUnlockError>,
) -> Task<Message> {
    // Take pending context from state
    let pending = match result {
        Ok((master_password, preloaded)) => {
            // Construct PendingVaultUnlock from preloaded data
            let pending = state.pending_vault_unlock.take().expect("pending context missing");
            (pending, master_password, preloaded)
        }
        Err(e) => {
            // Error: reopen vault unlock modal with error (translate to current language)
            let error_msg = match &e {
                crate::vault::VaultUnlockError::WrongPassword => {
                    state.model.i18n.tr("iced.vault_unlock.error.wrong_password").to_string()
                }
                crate::vault::VaultUnlockError::VaultNotInitialized => {
                    state.model.i18n.tr("iced.vault_unlock.error.vault_not_initialized").to_string()
                }
                crate::vault::VaultUnlockError::VaultError(_) => {
                    state.model.i18n.tr("iced.vault_unlock.error.unknown").to_string()
                }
            };
            state.vault_unlock = Some(super::super::state::VaultUnlockState {
                pending_connect: None,
                pending_delete_profile_id: None,
                pending_save_session: false,
                pending_save_credentials_profile_id: None,
                password: SecretString::from(String::new()),
                error: Some(error_msg),
            });
            return Task::none();
        }
    };

    let (pending, master_password, preloaded) = pending;

    // Set vault master password
    state.model.vault_master_password = Some(SecretString::from(master_password));
    state.vault_status = VaultStatus::compute(&state.model.settings, true);

    // Route to pending operations
    if pending.pending_save_session {
        return super::super::update::update(state, Message::SessionEditorSave);
    }

    if let Some(id) = pending.pending_delete_profile_id {
        return super::super::update::update(state, Message::DeleteSessionProfile(id));
    }

    if let Some(pid) = pending.pending_save_credentials_profile_id {
        // Save password entered in current draft after successful connect.
        let Some(master) = state.model.vault_master_password.as_ref() else {
            return Task::none();
        };
        let pw_ok = !state.model.draft.password.expose_secret().trim().is_empty();
        if pw_ok {
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
                if let Some(existing) = state.model.profiles().iter().find(|p| p.id == pid).cloned()
                {
                    if let crate::session::TransportConfig::Ssh(mut ssh) = existing.transport {
                        ssh.credential_id = Some(cid);
                        let updated = crate::session::SessionProfile {
                            transport: crate::session::TransportConfig::Ssh(ssh),
                            ..existing
                        };
                        let _ = state.rt.block_on(state.model.tab_manager.upsert_session(updated));
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

    if let Some(prof) = pending.pending_connect {
        // Apply preloaded credentials to draft (avoids second KDF round-trip)
        if let Some((pw, pp)) = preloaded {
            state.model.draft.password = SecretString::from(pw);
            state.model.draft.passphrase = SecretString::from(pp.unwrap_or_default());
        }

        // Extract values before any mutable borrow.
        let vault_unlocked_msg = state.model.i18n.tr("iced.term.vault_unlocked");

        // Fill draft with profile host/user/auth.
        let master = state.model.vault_master_password.clone();
        if state.model.fill_draft_from_profile(&prof, master.as_ref()).is_err() {
            return Task::none();
        }

        // Clear the entire terminal viewport before showing vault_unlocked.
        // SSH info (target + fingerprint + auth method + "连接中…") will be injected
        // by start_ssh_connect once ConnectPressed is dispatched.
        state.active_pane_mut().terminal.clear_local_preconnect_ui();
        state.active_pane_mut()
            .terminal
            .inject_local_lines(&[&vault_unlocked_msg]);
        // 记录 vault 提示行数，等待 handle_connect() 注入 SSH info 后一并计数
        state.vault_hint_line_count = 1;

        return super::super::update::update(state, Message::ConnectPressed);
    }

    Task::none()
}
