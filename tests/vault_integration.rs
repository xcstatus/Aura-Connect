use RustSsh::vault::core::CredentialVault;
use secrecy::SecretString;
use proptest::prelude::*;

// 1. 基于属性的随机口令测试 (Property-based Testing)
proptest! {
    #[test]
    fn test_vault_with_random_passwords(s in "\\PC*") {
        if s.is_empty() { return Ok(()); }
        
        let secret = SecretString::new(s.clone().into());
        let mut vault = CredentialVault::initialize(&secret).unwrap();
        assert!(vault.unlocked);
        
        let key = "test_key";
        let val = b"secret_data";
        vault.set_credential(key, val).unwrap();
        
        // 锁定并重新解锁
        vault.lock();
        assert!(!vault.unlocked);
        
        vault.unlock(&secret).unwrap();
        let retrieved = vault.get_credential(key).unwrap().unwrap();
        assert_eq!(retrieved, val);
    }
}

// 2. 内存安全验证
#[test]
fn test_vault_memory_lock_wipe() {
    let secret = SecretString::new("master_pass".into());
    let mut vault = CredentialVault::initialize(&secret).unwrap();
    vault.set_credential("k", b"v").unwrap();
    
    assert!(vault.unlocked);
    vault.lock();
    
    // 尝试越权访问（应报错）
    let res = vault.get_credential("k");
    assert!(res.is_err(), "应在锁定状态下拒绝访问");
}

// 3. 损坏数据鲁棒性测试
#[test]
fn test_corrupted_data_recovery() {
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join("corrupted_vault.json");
    
    let secret = SecretString::new("pass".into());
    let vault = CredentialVault::initialize(&secret).unwrap();
    vault.save_to_file(&path).unwrap();
    
    // 人为破坏文件内容
    std::fs::write(&path, "not a valid json").unwrap();
    
    let loaded = CredentialVault::load_from_file(&path);
    assert!(loaded.is_err(), "读取损坏文件时应当报错");
    
    let _ = std::fs::remove_file(path);
}
