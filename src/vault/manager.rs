use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
    Params,
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroizing;

use crate::settings::KdfMemoryLevel;

/// Vault KDF 参数（默认/安全模式）
/// 统一在此处管理，所有密钥派生都应使用此参数集。
pub const KDF_ARGON2_MEMORY_KIB: u32 = 65536;
pub const KDF_ARGON2_T_COST: u32 = 3;
pub const KDF_ARGON2_P_COST: u32 = 1;

/// 根据内存级别构建 Argon2id 实例
pub fn kdf_argon2id_for_level(level: KdfMemoryLevel) -> Argon2<'static> {
    let (mem_kib, t) = match level {
        KdfMemoryLevel::Balanced => (16384u32, 2u32),  // 16MiB, t=2
        KdfMemoryLevel::Security => (65536u32, 3u32),    // 64MiB, t=3
    };
    let params = Params::new(mem_kib, t, KDF_ARGON2_T_COST, Some(32))
        .expect("KDF params are hardcoded and should always be valid");
    Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
}

/// 从 KDF 参数构建 Argon2id 实例（使用默认内存级别）
pub fn kdf_argon2id() -> Argon2<'static> {
    kdf_argon2id_for_level(KdfMemoryLevel::default())
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VaultMeta {
    /// 唯一 Salt，用于 KDF
    pub salt: String,
    /// 派生出的验证哈希 (PHC 格式)
    pub verifier_hash: String,
    /// 独立的加密 Salt，用于派生独立的加密密钥（与认证密钥分离）
    pub encryption_salt: Option<String>,
}

pub struct VaultManager;

impl VaultManager {
    /// 初始化保险箱元数据（使用默认内存级别）
    pub fn setup_vault(password: &SecretString) -> anyhow::Result<VaultMeta> {
        Self::setup_vault_with_level(password, KdfMemoryLevel::default())
    }

    /// 初始化保险箱元数据（指定内存级别）
    pub fn setup_vault_with_level(password: &SecretString, level: KdfMemoryLevel) -> anyhow::Result<VaultMeta> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = kdf_argon2id_for_level(level);

        let password_hash = argon2
            .hash_password(password.expose_secret().as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("KDF init failed: {}", e))?
            .to_string();

        let encryption_salt = SaltString::generate(&mut OsRng).to_string();

        Ok(VaultMeta {
            salt: salt.to_string(),
            verifier_hash: password_hash,
            encryption_salt: Some(encryption_salt),
        })
    }

    /// 验证主密码是否正确（使用默认内存级别）
    pub fn verify_password(password: &SecretString, meta: &VaultMeta) -> bool {
        Self::verify_password_with_level(password, meta, KdfMemoryLevel::default())
    }

    /// 验证主密码是否正确（指定内存级别）
    pub fn verify_password_with_level(password: &SecretString, meta: &VaultMeta, level: KdfMemoryLevel) -> bool {
        use argon2::password_hash::PasswordHash;

        let Ok(parsed_hash) = PasswordHash::new(&meta.verifier_hash) else {
            return false;
        };

        kdf_argon2id_for_level(level)
            .verify_password(password.expose_secret().as_bytes(), &parsed_hash)
            .is_ok()
    }

    /// 派生出用于加密数据的根密钥 (Master Key)（使用默认内存级别）
    pub fn derive_master_key(password: &SecretString, salt: &str) -> anyhow::Result<Zeroizing<[u8; 32]>> {
        Self::derive_master_key_with_level(password, salt, KdfMemoryLevel::default())
    }

    /// 派生出用于加密数据的根密钥 (Master Key)（指定内存级别）
    pub fn derive_master_key_with_level(password: &SecretString, salt: &str, level: KdfMemoryLevel) -> anyhow::Result<Zeroizing<[u8; 32]>> {
        let salt_bytes = salt.as_bytes();
        let argon2 = kdf_argon2id_for_level(level);
        let mut output_key = [0u8; 32];
        argon2
            .hash_password_into(
                password.expose_secret().as_bytes(),
                salt_bytes,
                &mut output_key,
            )
            .map_err(|e| anyhow::anyhow!("Key derivation failed: {}", e))?;

        Ok(Zeroizing::new(output_key))
    }

    /// 派生出独立的加密密钥（使用默认内存级别）
    pub fn derive_encryption_key(password: &SecretString, meta: &VaultMeta) -> anyhow::Result<Zeroizing<[u8; 32]>> {
        Self::derive_encryption_key_with_level(password, meta, KdfMemoryLevel::default())
    }

    /// 派生出独立的加密密钥（指定内存级别）
    pub fn derive_encryption_key_with_level(password: &SecretString, meta: &VaultMeta, level: KdfMemoryLevel) -> anyhow::Result<Zeroizing<[u8; 32]>> {
        let enc_salt = meta.encryption_salt.as_deref()
            .ok_or_else(|| anyhow::anyhow!("No encryption salt in vault metadata"))?;
        Self::derive_master_key_with_level(password, enc_salt, level)
    }
}

impl fmt::Debug for VaultMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VaultMeta")
            .field("salt", &self.salt)
            .field("verifier_hash", &"[REDACTED]")
            .field("encryption_salt", &self.encryption_salt.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}
