use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Clone)]
pub struct VaultMeta {
    /// 唯一 Salt，用于 KDF
    pub salt: String,
    /// 派生出的验证哈希 (PHC 格式)
    pub verifier_hash: String,
}

pub struct VaultManager;

impl VaultManager {
    /// 初始化保险箱元数据
    /// 生成随机 Salt 并为初始密码计算哈希
    pub fn setup_vault(password: &SecretString) -> anyhow::Result<VaultMeta> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.expose_secret().as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("KDF init failed: {}", e))?
            .to_string();

        Ok(VaultMeta {
            salt: salt.to_string(),
            verifier_hash: password_hash,
        })
    }

    /// 验证主密码是否正确
    pub fn verify_password(password: &SecretString, meta: &VaultMeta) -> bool {
        use argon2::password_hash::PasswordHash;

        let Ok(parsed_hash) = PasswordHash::new(&meta.verifier_hash) else {
            return false;
        };

        Argon2::default()
            .verify_password(password.expose_secret().as_bytes(), &parsed_hash)
            .is_ok()
    }

    /// 派生出用于加密数据的根密钥 (Master Key)
    /// 建议使用不同的 Pepper 或者特定的子密钥派生
    pub fn derive_master_key(password: &SecretString, salt: &str) -> anyhow::Result<[u8; 32]> {
        let mut output_key = [0u8; 32];
        let salt_bytes = salt.as_bytes();

        let argon2 = Argon2::default();
        argon2
            .hash_password_into(
                password.expose_secret().as_bytes(),
                salt_bytes,
                &mut output_key,
            )
            .map_err(|e| anyhow::anyhow!("Key derivation failed: {}", e))?;

        Ok(output_key)
    }
}

impl fmt::Debug for VaultMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VaultMeta")
            .field("salt", &self.salt)
            .field("verifier_hash", &"[REDACTED]")
            .finish()
    }
}
