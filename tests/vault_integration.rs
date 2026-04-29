use proptest::prelude::*;
use rust_ssh::vault::core::CredentialVault;
use rust_ssh::vault::crypto;
use secrecy::SecretString;

// 1) 基于属性的随机口令测试（Property-based Testing）
//
// 测试内容：
// - 用 proptest 生成大量不同形态的主密码（包括空格、符号、Unicode 等非控制字符集合）
// - 对每个主密码执行一条最常见的用户路径：初始化 -> 写入凭据 -> 锁定 -> 解锁 -> 读取回放
//
// 测试原因：
// - Vault 的主密码输入来自真实用户，字符分布不可控；属性测试能覆盖大量“人想不到”的边界情况，
//   比少量固定用例更容易发现编码、序列化、KDF 输入处理等细节回归。
// - 这里强调“锁定后必须再次解锁才能读取”，防止未来优化把锁定状态绕过（安全回归）。
proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32,
        max_shrink_iters: 0,
        .. ProptestConfig::default()
    })]
    #[test]
    fn test_vault_with_random_passwords(s in "\\PC*") {
        // proptest 可能生成空字符串；如果产品语义允许空主密码，这里应显式覆盖。
        // 当前选择跳过：避免测试把“空口令是否允许”这个产品决策绑定死。
        if s.is_empty() { return Ok(()); }

        let secret = SecretString::new(s.clone().into());
        let mut vault = CredentialVault::initialize(&secret, crypto::generate_salt()).unwrap();
        assert!(vault.is_unlocked());

        let key = "test_key";
        let val = b"secret_data";
        vault.set_credential(key, val).unwrap();

        // 关键链路：锁定后应不可读；重新解锁后应可读且数据一致。
        vault.lock();
        assert!(!vault.is_unlocked());

        vault.unlock(&secret).unwrap();
        let retrieved = vault.get_credential(key).unwrap().unwrap();
        assert_eq!(retrieved, val);
    }
}

// 2) 锁定状态访问控制（安全边界）
//
// 测试内容：
// - 初始化并写入一条凭据后执行 `lock`
// - 在锁定状态下调用 `get_credential`，期望返回错误而不是返回数据
//
// 测试原因：
// - 这条断言是 Vault 的核心安全语义：锁定即“拒绝读取敏感信息”。
// - 若未来重构把锁状态判断漏掉，可能导致 UI/后台逻辑在未解锁情况下仍能读取明文数据（严重安全回归）。
#[test]
fn test_vault_memory_lock_wipe() {
    let secret = SecretString::new("master_pass".into());
    let mut vault = CredentialVault::initialize(&secret, crypto::generate_salt()).unwrap();
    vault.set_credential("k", b"v").unwrap();

    assert!(vault.is_unlocked());
    vault.lock();

    // 尝试越权访问：锁定状态下必须拒绝读取。
    let res = vault.get_credential("k");
    assert!(res.is_err(), "应在锁定状态下拒绝访问");
}

// 3) 损坏数据鲁棒性（故障可诊断性）
//
// 测试内容：
// - 先将一个可正常保存的 vault 写入文件
// - 再人为把文件内容改成无效 JSON
// - 调用 `load_from_file` 期望返回错误
//
// 测试原因：
// - 真实世界里文件可能因手动编辑、磁盘故障、版本切换、写入被中断等原因损坏。
// - 对损坏输入“明确失败并可上报/提示”，比 silent fallback 或 panic 更可控；
//   该测试确保读取路径对坏数据是可预期的失败模式。
#[test]
fn test_corrupted_data_recovery() {
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join("corrupted_vault.json");

    let secret = SecretString::new("pass".into());
    let vault = CredentialVault::initialize(&secret, crypto::generate_salt()).unwrap();
    vault.save_to_file(&path).unwrap();

    // 人为破坏文件内容
    std::fs::write(&path, "not a valid json").unwrap();

    let loaded = CredentialVault::load_from_file(&path);
    assert!(loaded.is_err(), "读取损坏文件时应当报错");

    let _ = std::fs::remove_file(path);
}

// 4) 错误密码失败后的状态不应被污染
//
// 测试内容：
// - 创建 vault 并写入数据后先 lock
// - 使用错误密码 unlock（期望失败）
// - 失败后再次读取应仍然被拒绝
// - 使用正确密码 unlock 后数据仍可读
//
// 测试原因：
// - 错误口令尝试是高频真实场景；失败路径必须“只失败，不改变状态”。
// - 若失败后仍把 vault 标成 unlocked，或者污染了解密密钥缓存，会形成严重安全漏洞。
#[test]
fn test_unlock_with_wrong_password_does_not_unlock_or_corrupt_state() {
    let correct = SecretString::new("correct-pass".into());
    let wrong = SecretString::new("wrong-pass".into());

    let mut vault = CredentialVault::initialize(&correct, crypto::generate_salt()).unwrap();
    vault.set_credential("token", b"abc123").unwrap();
    vault.lock();

    assert!(vault.unlock(&wrong).is_err(), "错误密码解锁应返回错误");
    assert!(!vault.is_unlocked(), "错误密码失败后不应变为 unlocked");
    assert!(
        vault.get_credential("token").is_err(),
        "错误密码失败后仍应拒绝读取"
    );

    vault.unlock(&correct).unwrap();
    let v = vault.get_credential("token").unwrap().unwrap();
    assert_eq!(v, b"abc123");
}

// 5) rekey 后旧密码必须失效（密码轮换安全性）
//
// 测试内容：
// - 使用旧密码初始化并写入数据
// - 调用 rekey 切换到新密码
// - lock 后验证旧密码解锁失败、新密码解锁成功且数据完整
//
// 测试原因：
// - 密码轮换是安全合规常见要求；若旧密码仍可解锁，等同轮换失效。
// - 该测试锁定“旧口令失效 + 新口令生效 + 数据不丢失”三项关键语义。
#[test]
fn test_rekey_invalidates_old_password_and_preserves_data() {
    let old_pwd = SecretString::new("old-master".into());
    let new_pwd = SecretString::new("new-master".into());

    let mut vault = CredentialVault::initialize(&old_pwd, crypto::generate_salt()).unwrap();
    vault
        .set_credential("ssh_key", b"PRIVATE_KEY_BYTES")
        .unwrap();

    vault
        .rekey_with_salt(&new_pwd, crypto::generate_salt())
        .unwrap();
    vault.lock();

    assert!(vault.unlock(&old_pwd).is_err(), "rekey 后旧密码必须失效");
    assert!(!vault.is_unlocked(), "旧密码失败后不应变为 unlocked");

    vault.unlock(&new_pwd).unwrap();
    let val = vault.get_credential("ssh_key").unwrap().unwrap();
    assert_eq!(val, b"PRIVATE_KEY_BYTES");
}
