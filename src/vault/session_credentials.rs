use anyhow::{Result, anyhow};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::settings::KdfMemoryLevel;
use crate::utils::StorageManager;
use crate::vault::core::CredentialVault;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SshCredentialPayload {
    pub password: Option<Zeroizing<String>>,
    pub passphrase: Option<Zeroizing<String>>,
}

/// 加载并解锁 vault（使用显式 master_password 和 KDF 内存级别）。
/// 若 vault 文件不存在但 settings 中存在 vault 元数据，则自动初始化一个新的空 vault。
pub(crate) fn load_unlocked_vault(
    master_password: &SecretString,
    kdf_level: KdfMemoryLevel,
) -> Result<CredentialVault> {
    let path = StorageManager::get_vault_path().ok_or_else(|| anyhow!("无法定位 vault 路径"))?;

    if path.exists() {
        let mut vault = CredentialVault::load_from_file(&path)?;
        vault.unlock_with_level(master_password, kdf_level)?;
        Ok(vault)
    } else {
        // Vault 文件不存在时，生成随机 salt 并初始化。
        let salt = crate::vault::crypto::generate_salt();
        let vault = CredentialVault::initialize_with_level(
            master_password,
            salt,
            kdf_level,
        )?;
        vault.save_to_file(&path)?;
        Ok(vault)
    }
}

/// 保存 SSH 会话凭据（密码和私钥口令）。
///
/// - 当 password / passphrase 至少有一个非空时，加密写入 vault 并返回 Some(credential_id)。
/// - 当两者都为空时，删除已存在的凭据并返回 None。
///
/// 需要 vault 处于解锁态（即调用方持有有效的 master_password）。
pub fn save_ssh_credentials(
    master_password: &SecretString,
    session_id: &str,
    password: Option<&str>,
    passphrase: Option<&str>,
    kdf_level: KdfMemoryLevel,
) -> Result<Option<String>> {
    let pwd = password
        .filter(|s| !s.trim().is_empty())
        .map(|s| Zeroizing::new(s.trim().to_owned()));
    let pph = passphrase
        .filter(|s| !s.trim().is_empty())
        .map(|s| Zeroizing::new(s.trim().to_owned()));

    let mut vault = load_unlocked_vault(master_password, kdf_level)?;
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

/// 同步 SSH 会话凭据。与 save_ssh_credentials 行为一致。
///
/// 需要 vault 处于解锁态。
pub fn sync_ssh_credentials(
    master_password: &SecretString,
    session_id: &str,
    password: Option<&str>,
    passphrase: Option<&str>,
    kdf_level: KdfMemoryLevel,
) -> Result<Option<String>> {
    save_ssh_credentials(master_password, session_id, password, passphrase, kdf_level)
}

/// 加载 SSH 会话凭据。
///
/// 需要 vault 处于解锁态（即调用方持有有效的 master_password）。
pub fn load_ssh_credentials(
    master_password: &SecretString,
    credential_id: &str,
    kdf_level: KdfMemoryLevel,
) -> Result<Option<SshCredentialPayload>> {
    let vault = load_unlocked_vault(master_password, kdf_level)?;
    let raw = vault.get_credential(credential_id)?;
    let Some(raw) = raw else {
        return Ok(None);
    };
    let payload: SshCredentialPayload = serde_json::from_slice(&raw)?;
    Ok(Some(payload))
}

/// 删除指定凭据。
///
/// 需要 vault 处于解锁态。
pub fn delete_credential(
    master_password: &SecretString,
    credential_id: &str,
    kdf_level: KdfMemoryLevel,
) -> Result<()> {
    let mut vault = load_unlocked_vault(master_password, kdf_level)?;
    let path = StorageManager::get_vault_path().ok_or_else(|| anyhow!("无法定位 vault 路径"))?;
    vault.delete_credential(credential_id)?;
    vault.save_to_file(&path)?;
    Ok(())
}

// ============================================================================
// 兼容性别名（deprecated），指向新的统一 API
// ============================================================================

/// [已废弃] 请使用 [`save_ssh_credentials`]。
#[deprecated(since = "1.0", note = "使用 save_ssh_credentials 替代")]
pub fn save_ssh_credentials_with_master(
    _settings: &crate::settings::Settings,
    master_password: &SecretString,
    session_id: &str,
    password: Option<&SecretString>,
    passphrase: Option<&SecretString>,
) -> Result<Option<String>> {
    save_ssh_credentials(
        master_password,
        session_id,
        password.map(|s| s.expose_secret()),
        passphrase.map(|s| s.expose_secret()),
        KdfMemoryLevel::default(),
    )
}

/// [已废弃] 请使用 [`sync_ssh_credentials`]。
#[deprecated(since = "1.0", note = "使用 sync_ssh_credentials 替代")]
pub fn sync_ssh_credentials_with_master(
    _settings: &crate::settings::Settings,
    master_password: &SecretString,
    session_id: &str,
    password: Option<&SecretString>,
    passphrase: Option<&SecretString>,
) -> Result<Option<String>> {
    sync_ssh_credentials(
        master_password,
        session_id,
        password.map(|s| s.expose_secret()),
        passphrase.map(|s| s.expose_secret()),
        KdfMemoryLevel::default(),
    )
}

/// [已废弃] 请使用 [`load_ssh_credentials`]。
#[deprecated(since = "1.0", note = "使用 load_ssh_credentials 替代")]
pub fn load_ssh_credentials_with_master(
    _settings: &crate::settings::Settings,
    master_password: &SecretString,
    credential_id: &str,
) -> Result<Option<SshCredentialPayload>> {
    load_ssh_credentials(
        master_password,
        credential_id,
        KdfMemoryLevel::default(),
    )
}

/// [已废弃] 请使用 [`delete_credential`]。
#[deprecated(since = "1.0", note = "使用 delete_credential 替代")]
pub fn delete_credential_with_master(
    _settings: &crate::settings::Settings,
    master_password: &SecretString,
    credential_id: &str,
) -> Result<()> {
    delete_credential(
        master_password,
        credential_id,
        KdfMemoryLevel::default(),
    )
}
