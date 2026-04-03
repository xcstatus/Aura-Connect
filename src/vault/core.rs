use super::crypto;
use super::error::VaultError;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use zeroize::Zeroizing;

/// Internal structure serialized into the encrypted payload
#[derive(Serialize, Deserialize, Default)]
struct VaultPayload {
    /// Mapping of credential identifiers to their binary payload
    credentials: HashMap<String, Vec<u8>>,
}

/// The layout used when Vault is persisted to disk
#[derive(Serialize, Deserialize)]
struct VaultDiskFormat {
    salt: Vec<u8>,
    encrypted_data: Vec<u8>,
}

pub struct CredentialVault {
    encrypted_data: Vec<u8>,
    salt: [u8; 16],
    unlocked_key: Option<Zeroizing<[u8; 32]>>,
    pub unlocked: bool,
}

impl CredentialVault {
    /// Initialize a new Vault with a master password
    pub fn initialize(master_password: &SecretString) -> Result<Self, VaultError> {
        let salt = crypto::generate_salt();
        let key = crypto::derive_key(master_password, &salt)?;

        // Initial empty payload
        let payload = VaultPayload::default();
        let serialized = serde_json::to_vec(&payload)?;
        let encrypted_data = crypto::encrypt(&key, &serialized)?;

        Ok(Self {
            encrypted_data,
            salt,
            unlocked_key: Some(key),
            unlocked: true,
        })
    }

    /// Unlock an existing vault with the master password
    pub fn unlock(&mut self, master_password: &SecretString) -> Result<(), VaultError> {
        let key = crypto::derive_key(master_password, &self.salt)?;

        // Verify we can decrypt the payload with this key
        let _ =
            crypto::decrypt(&key, &self.encrypted_data).map_err(|_| VaultError::InvalidPassword)?;

        self.unlocked_key = Some(key);
        self.unlocked = true;
        Ok(())
    }

    /// Lock the vault and wipe the secret key from memory
    pub fn lock(&mut self) {
        self.unlocked_key = None;
        self.unlocked = false;
    }

    /// Re-encrypt the current vault payload with a new password.
    /// The vault must be unlocked before calling this method.
    pub fn rekey(&mut self, new_password: &SecretString) -> Result<(), VaultError> {
        let current_key = self.unlocked_key.as_ref().ok_or(VaultError::Locked)?;
        let decrypted = crypto::decrypt(current_key, &self.encrypted_data)?;

        let new_salt = crypto::generate_salt();
        let new_key = crypto::derive_key(new_password, &new_salt)?;
        let new_encrypted = crypto::encrypt(&new_key, &decrypted)?;

        self.salt = new_salt;
        self.encrypted_data = new_encrypted;
        self.unlocked_key = Some(new_key);
        self.unlocked = true;
        Ok(())
    }

    /// Get a stored credential, returning `None` if the credential doesn't exist
    pub fn get_credential(&self, key: &str) -> Result<Option<Vec<u8>>, VaultError> {
        let secret_key = self.unlocked_key.as_ref().ok_or(VaultError::Locked)?;

        let decrypted = crypto::decrypt(secret_key, &self.encrypted_data)?;
        let payload: VaultPayload = serde_json::from_slice(&decrypted)?;

        Ok(payload.credentials.get(key).cloned())
    }

    /// Set or update a credential
    pub fn set_credential(&mut self, key: &str, value: &[u8]) -> Result<(), VaultError> {
        let secret_key = self.unlocked_key.as_ref().ok_or(VaultError::Locked)?;

        let decrypted = crypto::decrypt(secret_key, &self.encrypted_data)?;
        let mut payload: VaultPayload = serde_json::from_slice(&decrypted)?;

        payload.credentials.insert(key.to_string(), value.to_vec());

        let serialized = serde_json::to_vec(&payload)?;
        self.encrypted_data = crypto::encrypt(secret_key, &serialized)?;

        Ok(())
    }

    /// Delete a credential from the vault
    pub fn delete_credential(&mut self, key: &str) -> Result<(), VaultError> {
        let secret_key = self.unlocked_key.as_ref().ok_or(VaultError::Locked)?;

        let decrypted = crypto::decrypt(secret_key, &self.encrypted_data)?;
        let mut payload: VaultPayload = serde_json::from_slice(&decrypted)?;

        if payload.credentials.remove(key).is_some() {
            let serialized = serde_json::to_vec(&payload)?;
            self.encrypted_data = crypto::encrypt(secret_key, &serialized)?;
        }

        Ok(())
    }

    /// Save the encrypted Vault to a specified file path
    pub fn save_to_file(&self, path: &Path) -> Result<(), VaultError> {
        let storage = VaultDiskFormat {
            salt: self.salt.to_vec(),
            encrypted_data: self.encrypted_data.clone(),
        };
        let data = serde_json::to_string_pretty(&storage)?;
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path, data).map_err(|e| VaultError::EncryptionFailed(e.to_string()))?;
        Ok(())
    }

    /// Load the encrypted Vault from a file
    pub fn load_from_file(path: &Path) -> Result<Self, VaultError> {
        let data =
            fs::read_to_string(path).map_err(|e| VaultError::DecryptionFailed(e.to_string()))?;
        let storage: VaultDiskFormat = serde_json::from_str(&data)?;

        let mut salt = [0u8; 16];
        if storage.salt.len() == 16 {
            salt.copy_from_slice(&storage.salt);
        } else {
            return Err(VaultError::DecryptionFailed("Invalid salt length".into()));
        }

        Ok(Self {
            encrypted_data: storage.encrypted_data,
            salt,
            unlocked_key: None,
            unlocked: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    #[test]
    fn test_vault_lifecycle() {
        let password = SecretString::from("SuperSecret!23".to_string());
        // Initialization
        let mut vault = CredentialVault::initialize(&password).unwrap();
        assert!(vault.unlocked);

        // Write credential
        vault
            .set_credential("my_ssh_key", b"-----BEGIN OPENSSH PRIVATE KEY-----")
            .unwrap();

        // Read credential
        let cred = vault.get_credential("my_ssh_key").unwrap().unwrap();
        assert_eq!(cred, b"-----BEGIN OPENSSH PRIVATE KEY-----");

        // Lock
        vault.lock();
        assert!(!vault.unlocked);
        assert!(vault.get_credential("my_ssh_key").is_err());

        // Unlock with wrong password
        assert!(
            vault
                .unlock(&SecretString::from("WrongPass".to_string()))
                .is_err()
        );

        // Unlock with correct password
        vault.unlock(&password).unwrap();
        assert!(vault.unlocked);

        let cred2 = vault.get_credential("my_ssh_key").unwrap().unwrap();
        assert_eq!(cred2, b"-----BEGIN OPENSSH PRIVATE KEY-----");

        // Delete credential
        vault.delete_credential("my_ssh_key").unwrap();
        assert!(vault.get_credential("my_ssh_key").unwrap().is_none());
    }

    #[test]
    fn test_vault_persistence() {
        let mut file_path = std::env::temp_dir();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        file_path.push(format!("vault_test_{}.json", timestamp));

        let password = SecretString::from("Pass123".to_string());
        let mut vault = CredentialVault::initialize(&password).unwrap();
        vault.set_credential("api_token", b"abcdef123456").unwrap();

        // Save to disk
        vault.save_to_file(&file_path).unwrap();

        // Load into new instance
        let mut loaded_vault = CredentialVault::load_from_file(&file_path).unwrap();
        assert!(!loaded_vault.unlocked);
        assert!(loaded_vault.get_credential("api_token").is_err());

        // Unlock new instance
        loaded_vault.unlock(&password).unwrap();

        // Verify credential
        let cred = loaded_vault.get_credential("api_token").unwrap().unwrap();
        assert_eq!(cred, b"abcdef123456");

        // Cleanup
        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_vault_rekey() {
        let old_pwd = SecretString::from("old-pass".to_string());
        let new_pwd = SecretString::from("new-pass".to_string());
        let mut vault = CredentialVault::initialize(&old_pwd).unwrap();
        vault.set_credential("k", b"v").unwrap();

        vault.rekey(&new_pwd).unwrap();
        vault.lock();
        assert!(vault.unlock(&old_pwd).is_err());
        vault.unlock(&new_pwd).unwrap();
        assert_eq!(vault.get_credential("k").unwrap().unwrap(), b"v");
    }
}
