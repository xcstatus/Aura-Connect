use thiserror::Error;

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("Vault is locked")]
    Locked,
    #[error("Invalid master password")]
    InvalidPassword,
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Key derivation failed")]
    DerivationFailed,
}
