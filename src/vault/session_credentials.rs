use anyhow::{Result, anyhow};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::settings::Settings;
use crate::storage::StorageManager;
use crate::vault::core::CredentialVault;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SshCredentialPayload {
    pub password: Option<String>,
    pub passphrase: Option<String>,
}

fn vault_password_from_settings(settings: &Settings) -> Result<SecretString> {
    let meta = settings
        .security
        .vault
        .as_ref()
        .ok_or_else(|| anyhow!("Vault 未初始化，请先在安全设置中设置主密码"))?;
    // Current project only persists verifier hash; use it as a deterministic
    // secret for local vault encryption key derivation.
    Ok(SecretString::from(meta.verifier_hash.clone()))
}

fn load_or_init_unlocked_vault(settings: &Settings) -> Result<CredentialVault> {
    let password = vault_password_from_settings(settings)?;
    let path = StorageManager::get_vault_path().ok_or_else(|| anyhow!("无法定位 vault 路径"))?;

    if path.exists() {
        let mut vault = CredentialVault::load_from_file(&path)?;
        vault.unlock(&password)?;
        Ok(vault)
    } else {
        let vault = CredentialVault::initialize(&password)?;
        vault.save_to_file(&path)?;
        Ok(vault)
    }
}

fn load_or_init_unlocked_vault_with_master(
    _settings: &Settings,
    master_password: &SecretString,
) -> Result<CredentialVault> {
    let path = StorageManager::get_vault_path().ok_or_else(|| anyhow!("无法定位 vault 路径"))?;
    if path.exists() {
        let mut vault = CredentialVault::load_from_file(&path)?;
        vault.unlock(master_password)?;
        Ok(vault)
    } else {
        let vault = CredentialVault::initialize(master_password)?;
        vault.save_to_file(&path)?;
        Ok(vault)
    }
}

pub fn save_ssh_credentials(
    settings: &Settings,
    session_id: &str,
    password: Option<&SecretString>,
    passphrase: Option<&SecretString>,
) -> Result<Option<String>> {
    let pwd = password
        .map(|s| s.expose_secret().trim().to_string())
        .filter(|s| !s.is_empty());
    let pph = passphrase
        .map(|s| s.expose_secret().trim().to_string())
        .filter(|s| !s.is_empty());

    if pwd.is_none() && pph.is_none() {
        return Ok(None);
    }

    let mut vault = load_or_init_unlocked_vault(settings)?;
    let path = StorageManager::get_vault_path().ok_or_else(|| anyhow!("无法定位 vault 路径"))?;
    let credential_id = format!("ssh:{}", session_id);
    let payload = SshCredentialPayload {
        password: pwd,
        passphrase: pph,
    };
    let bytes = serde_json::to_vec(&payload)?;
    vault.set_credential(&credential_id, &bytes)?;
    vault.save_to_file(&path)?;
    Ok(Some(credential_id))
}

/// Upsert SSH credentials for a session.
///
/// - When at least one of `password` / `passphrase` is provided (non-empty), writes to vault and returns `Some(credential_id)`.
/// - When both are `None`/empty, deletes any existing `ssh:{session_id}` credential and returns `None`.
pub fn sync_ssh_credentials(
    settings: &Settings,
    session_id: &str,
    password: Option<&SecretString>,
    passphrase: Option<&SecretString>,
) -> Result<Option<String>> {
    let pwd = password
        .map(|s| s.expose_secret().trim().to_string())
        .filter(|s| !s.is_empty());
    let pph = passphrase
        .map(|s| s.expose_secret().trim().to_string())
        .filter(|s| !s.is_empty());

    let mut vault = load_or_init_unlocked_vault(settings)?;
    let path = StorageManager::get_vault_path().ok_or_else(|| anyhow!("无法定位 vault 路径"))?;
    let credential_id = format!("ssh:{}", session_id);

    if pwd.is_none() && pph.is_none() {
        vault.delete_credential(&credential_id)?;
        vault.save_to_file(&path)?;
        return Ok(None);
    }

    let payload = SshCredentialPayload {
        password: pwd,
        passphrase: pph,
    };
    let bytes = serde_json::to_vec(&payload)?;
    vault.set_credential(&credential_id, &bytes)?;
    vault.save_to_file(&path)?;
    Ok(Some(credential_id))
}

pub fn load_ssh_credentials(
    settings: &Settings,
    credential_id: &str,
) -> Result<Option<SshCredentialPayload>> {
    let vault = load_or_init_unlocked_vault(settings)?;
    let raw = vault.get_credential(credential_id)?;
    let Some(raw) = raw else {
        return Ok(None);
    };
    let payload: SshCredentialPayload = serde_json::from_slice(&raw)?;
    Ok(Some(payload))
}

pub fn load_ssh_credentials_with_master(
    settings: &Settings,
    master_password: &SecretString,
    credential_id: &str,
) -> Result<Option<SshCredentialPayload>> {
    let vault = load_or_init_unlocked_vault_with_master(settings, master_password)?;
    let raw = vault.get_credential(credential_id)?;
    let Some(raw) = raw else {
        return Ok(None);
    };
    let payload: SshCredentialPayload = serde_json::from_slice(&raw)?;
    Ok(Some(payload))
}

pub fn save_ssh_credentials_with_master(
    settings: &Settings,
    master_password: &SecretString,
    session_id: &str,
    password: Option<&SecretString>,
    passphrase: Option<&SecretString>,
) -> Result<Option<String>> {
    let pwd = password
        .map(|s| s.expose_secret().trim().to_string())
        .filter(|s| !s.is_empty());
    let pph = passphrase
        .map(|s| s.expose_secret().trim().to_string())
        .filter(|s| !s.is_empty());
    if pwd.is_none() && pph.is_none() {
        return Ok(None);
    }
    let mut vault = load_or_init_unlocked_vault_with_master(settings, master_password)?;
    let path = StorageManager::get_vault_path().ok_or_else(|| anyhow!("无法定位 vault 路径"))?;
    let credential_id = format!("ssh:{}", session_id);
    let payload = SshCredentialPayload {
        password: pwd,
        passphrase: pph,
    };
    let bytes = serde_json::to_vec(&payload)?;
    vault.set_credential(&credential_id, &bytes)?;
    vault.save_to_file(&path)?;
    Ok(Some(credential_id))
}

pub fn sync_ssh_credentials_with_master(
    settings: &Settings,
    master_password: &SecretString,
    session_id: &str,
    password: Option<&SecretString>,
    passphrase: Option<&SecretString>,
) -> Result<Option<String>> {
    let pwd = password
        .map(|s| s.expose_secret().trim().to_string())
        .filter(|s| !s.is_empty());
    let pph = passphrase
        .map(|s| s.expose_secret().trim().to_string())
        .filter(|s| !s.is_empty());
    let mut vault = load_or_init_unlocked_vault_with_master(settings, master_password)?;
    let path = StorageManager::get_vault_path().ok_or_else(|| anyhow!("无法定位 vault 路径"))?;
    let credential_id = format!("ssh:{}", session_id);
    if pwd.is_none() && pph.is_none() {
        vault.delete_credential(&credential_id)?;
        vault.save_to_file(&path)?;
        return Ok(None);
    }
    let payload = SshCredentialPayload {
        password: pwd,
        passphrase: pph,
    };
    let bytes = serde_json::to_vec(&payload)?;
    vault.set_credential(&credential_id, &bytes)?;
    vault.save_to_file(&path)?;
    Ok(Some(credential_id))
}

pub fn delete_credential_with_master(
    settings: &Settings,
    master_password: &SecretString,
    credential_id: &str,
) -> Result<()> {
    let mut vault = load_or_init_unlocked_vault_with_master(settings, master_password)?;
    let path = StorageManager::get_vault_path().ok_or_else(|| anyhow!("无法定位 vault 路径"))?;
    vault.delete_credential(credential_id)?;
    vault.save_to_file(&path)?;
    Ok(())
}

pub fn delete_credential(settings: &Settings, credential_id: &str) -> Result<()> {
    let mut vault = load_or_init_unlocked_vault(settings)?;
    let path = StorageManager::get_vault_path().ok_or_else(|| anyhow!("无法定位 vault 路径"))?;
    vault.delete_credential(credential_id)?;
    vault.save_to_file(&path)?;
    Ok(())
}
