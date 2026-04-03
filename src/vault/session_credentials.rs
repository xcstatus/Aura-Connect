use anyhow::{Result, anyhow};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::settings::Settings;
use crate::storage::StorageManager;
use crate::vault::core::CredentialVault;
use crate::vault::manager::VaultManager;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SshCredentialPayload {
    pub password: Option<String>,
    pub passphrase: Option<String>,
}

/// 派生独立的加密密钥（通过 encryption_salt），用于会话凭据加密。
/// 此函数保留作为设计说明：当前会话凭据存储在 CredentialVault 内部，
/// 密钥派生由 CredentialVault::initialize/unlock 处理（使用 auth salt）。
/// 当需要认证密钥和加密密钥完全分离时，可使用此函数并重构 CredentialVault。
fn derive_credential_key(settings: &Settings) -> Result<Zeroizing<[u8; 32]>> {
    let meta = settings
        .security
        .vault
        .as_ref()
        .ok_or_else(|| anyhow!("Vault 未初始化"))?;
    let password = SecretString::from(meta.verifier_hash.clone());
    VaultManager::derive_encryption_key(&password, meta)
}

// 加载 vault 并解锁（用于存储/检索会话凭据）
// 使用与 vault 文件相同的密钥派生方式（从 auth salt 派生）
fn load_unlocked_vault_for_credentials(settings: &Settings) -> Result<CredentialVault> {
    let password = SecretString::from(
        settings
            .security
            .vault
            .as_ref()
            .ok_or_else(|| anyhow!("Vault 未初始化"))?
            .verifier_hash
            .clone(),
    );
    let path = StorageManager::get_vault_path().ok_or_else(|| anyhow!("无法定位 vault 路径"))?;

    if path.exists() {
        let mut vault = CredentialVault::load_from_file(&path)?;
        vault.unlock(&password)?;
        Ok(vault)
    } else {
        // 不自动初始化——凭据 vault 依赖于 settings 中的 vault 元数据已存在
        Err(anyhow!("Vault 文件不存在，请先在安全设置中初始化 Vault"))
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

    let mut vault = load_unlocked_vault_for_credentials(settings)?;
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

    let mut vault = load_unlocked_vault_for_credentials(settings)?;
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
    let vault = load_unlocked_vault_for_credentials(settings)?;
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
    let mut vault = load_unlocked_vault_for_credentials(settings)?;
    let path = StorageManager::get_vault_path().ok_or_else(|| anyhow!("无法定位 vault 路径"))?;
    vault.delete_credential(credential_id)?;
    vault.save_to_file(&path)?;
    Ok(())
}
