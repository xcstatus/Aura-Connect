pub mod crypto;
mod error;

pub mod core;
pub mod manager;
pub mod session_credentials;
pub mod utils;

pub use error::VaultError;
pub use manager::{VaultManager, VaultMeta};
pub use utils::check_password_strength;

/// Result type for vault unlock operations containing the verifier hash.
#[derive(Debug, Clone)]
pub struct VaultUnlockResult {
    pub verifier_hash: String,
}

/// Error types for vault unlock operations.
#[derive(Debug, Clone)]
pub enum VaultUnlockError {
    WrongPassword,
    VaultNotInitialized,
    VaultError(String),
}

impl VaultUnlockError {
    pub fn i18n_key(&self) -> &'static str {
        match self {
            VaultUnlockError::WrongPassword => "iced.vault_unlock.error.wrong_password",
            VaultUnlockError::VaultNotInitialized => "iced.vault_unlock.error.vault_not_initialized",
            VaultUnlockError::VaultError(_) => "iced.vault_unlock.error.unknown",
        }
    }
}

impl std::fmt::Display for VaultUnlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.i18n_key())
    }
}
