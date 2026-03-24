use rust_ssh::vault::VaultManager;
use secrecy::SecretString;

#[test]
fn test_vault_setup_and_verify() {
    let password = SecretString::from("correct_password".to_string());
    let wrong_password = SecretString::from("wrong_password".to_string());
    
    // 1. Setup
    let meta = VaultManager::setup_vault(&password).unwrap();
    assert!(!meta.salt.is_empty());
    assert!(meta.verifier_hash.contains("$argon2id$"));

    // 2. Verify Correct
    assert!(VaultManager::verify_password(&password, &meta));

    // 3. Verify Wrong
    assert!(!VaultManager::verify_password(&wrong_password, &meta));
}

#[test]
fn test_master_key_derivation() {
    let password = SecretString::from("my_secure_password".to_string());
    let salt = "fixed_salt_for_testing";
    
    // 1. Derive once
    let key1 = VaultManager::derive_master_key(&password, salt).unwrap();
    
    // 2. Derive again (should be same)
    let key2 = VaultManager::derive_master_key(&password, salt).unwrap();
    
    assert_eq!(key1, key2);
    assert!(key1.iter().any(|&b| b != 0)); // Not all zeros
    
    // 3. Different password -> different key
    let other_pwd = SecretString::from("other_password".to_string());
    let key3 = VaultManager::derive_master_key(&other_pwd, salt).unwrap();
    assert_ne!(key1, key3);
}
