use super::error::VaultError;
use super::manager::kdf_argon2id_for_level;
use crate::settings::KdfMemoryLevel;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use rand::RngCore;
use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroizing;

const SALT_SIZE: usize = 16;
const NONCE_SIZE: usize = 12;

pub fn generate_salt() -> [u8; SALT_SIZE] {
    let mut salt = [0u8; SALT_SIZE];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

/// 使用 Argon2id 派生密钥，并使用 Zeroizing 包裹以防内存残留。
/// KDF 参数统一在 `manager::kdf_argon2id_for_level()` 中管理。
/// 使用默认内存级别。
pub fn derive_key(password: &SecretString, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>, VaultError> {
    derive_key_with_level(password, salt, KdfMemoryLevel::default())
}

/// 使用 Argon2id 派生密钥（指定内存级别）
pub fn derive_key_with_level(
    password: &SecretString,
    salt: &[u8],
    level: KdfMemoryLevel,
) -> Result<Zeroizing<[u8; 32]>, VaultError> {
    let argon2 = kdf_argon2id_for_level(level);
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password.expose_secret().as_bytes(), salt, &mut key)
        .map_err(|_| VaultError::DerivationFailed)?;

    Ok(Zeroizing::new(key))
}

pub fn generate_nonce() -> [u8; NONCE_SIZE] {
    let mut nonce = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut nonce);
    nonce
}

pub fn encrypt(key: &Zeroizing<[u8; 32]>, plaintext: &[u8]) -> Result<Vec<u8>, VaultError> {
    let nonce_bytes = generate_nonce();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let cipher = Aes256Gcm::new(key.as_slice().into());

    let mut ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| VaultError::EncryptionFailed(e.to_string()))?;

    let mut result = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.append(&mut ciphertext);
    Ok(result)
}

pub fn decrypt(key: &Zeroizing<[u8; 32]>, encrypted_data: &[u8]) -> Result<Vec<u8>, VaultError> {
    if encrypted_data.len() < NONCE_SIZE {
        return Err(VaultError::DecryptionFailed("Data too short".into()));
    }

    let (nonce_bytes, ciphertext) = encrypted_data.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);
    let cipher = Aes256Gcm::new(key.as_slice().into());

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| VaultError::DecryptionFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    #[test]
    fn test_encryption_decryption() {
        let key = Zeroizing::new([42u8; 32]);
        let plaintext = b"Hello, secret world!";

        let encrypted = encrypt(&key, plaintext).unwrap();
        assert_ne!(plaintext.as_slice(), encrypted.as_slice());

        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_derive_key() {
        let salt = generate_salt();
        let password = SecretString::from("password123".to_string());
        let key1 = derive_key(&password, &salt).unwrap();
        let key2 = derive_key(&password, &salt).unwrap();

        let diff_password = SecretString::from("Password123".to_string());
        let key3 = derive_key(&diff_password, &salt).unwrap();

        assert_eq!(*key1, *key2);
        assert_ne!(*key1, *key3);
    }
}
