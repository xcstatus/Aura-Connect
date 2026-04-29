use std::sync::{Mutex, OnceLock};

use secrecy::{ExposeSecret, SecretString};
use tempfile::tempdir;

use rust_ssh::session::{AuthMethod, SessionLibrary, SessionProfile, SshConfig, TransportConfig};
use rust_ssh::settings::{KdfMemoryLevel, Settings};
use rust_ssh::utils::StorageManager;
use rust_ssh::vault::VaultManager;

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// 在临时 HOME/XDG_CONFIG_HOME 环境中运行测试闭包。
///
/// 作用：
/// - 将配置文件、vault 文件、sessions 文件都重定向到临时目录，避免污染开发机真实数据。
/// - 让测试具备可重复性：每个用例都从“干净环境”启动。
///
/// 原理：
/// - 使用 `tempdir` 创建独立临时目录。
/// - 临时覆盖进程级环境变量 `HOME` / `XDG_CONFIG_HOME`，让 `ProjectDirs` 路径解析落到临时目录。
/// - 用全局 `ENV_LOCK` 串行化环境变量修改，规避并发读写环境变量导致的 UB 风险。
/// - 闭包执行后恢复原始环境变量，确保测试前后环境对称。
///
/// 针对问题：
/// - “本地跑一次测试把真实 settings/vault 改坏”的风险。
/// - 多个用例互相污染数据目录导致偶现失败（flaky）。
fn with_temp_home<T>(f: impl FnOnce() -> T) -> T {
    let _g = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let dir = tempdir().unwrap();

    let old_home = std::env::var("HOME").ok();
    let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    // SAFETY: modifying process environment is unsafe with concurrent readers/writers.
    // We serialize all env mutations in this test module via ENV_LOCK.
    unsafe {
        std::env::set_var("HOME", dir.path());
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
    }

    let out = f();

    match old_home {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    match old_xdg {
        Some(v) => unsafe { std::env::set_var("XDG_CONFIG_HOME", v) },
        None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
    }
    out
}

/// 用例：保存会话 -> 写入 vault -> 模拟重启 -> 自动回填密码。
///
/// 作用：
/// - 覆盖最关键的主链路：凭据落盘、会话落盘、重启后加载并回填。
///
/// 原理：
/// - 先调用 `VaultManager::setup_vault` 生成并持久化 `settings.security.vault` 元数据。
/// - 再通过 `save_ssh_credentials` 将密码写入 vault，并把 `credential_id` 绑定到 profile。
/// - 手工写入 `sessions.json` 模拟“应用重启后仅凭磁盘状态恢复”。
/// - 调用 `AppModel::load + fill_draft_from_profile`，断言密码成功从 vault 回填到 draft。
///
/// 针对问题：
/// - profile 与 vault 凭据关联断裂（有 `credential_id` 但读不到密码）。
/// - 重启后功能退化：保存时可用，重启后不可用。
#[test]
fn save_write_restart_prefill() {
    with_temp_home(|| {
        // 1) Initialize settings vault meta (persisted).
        let user_pwd = SecretString::new("master-pass".into());
        let meta = VaultManager::setup_vault(&user_pwd).unwrap();
        let mut settings = Settings::default();
        settings.security.vault = Some(meta.clone());
        settings.save().unwrap();

        // 2) Save a profile with password in vault (credential_id stored in profile).
        let sid = "sess-1";
        let pw = SecretString::new("ssh-pass".into());
        let runtime_master = SecretString::from(meta.verifier_hash.clone());
        let credential_id = rust_ssh::vault::session_credentials::save_ssh_credentials(
            &runtime_master,
            sid,
            Some(pw.expose_secret()),
            None,
            KdfMemoryLevel::default(),
        )
        .unwrap();

        let profile = SessionProfile {
            id: sid.to_string(),
            name: "u@h".to_string(),
            group_id: None,
            folder: None,
            color_tag: None,
            transport: TransportConfig::Ssh(SshConfig {
                host: "example.com".to_string(),
                port: 22,
                user: "u".to_string(),
                auth: AuthMethod::Password,
                credential_id,
            }),
        };

        // Persist sessions.json like a "restart".
        let sessions_path = StorageManager::get_sessions_path().unwrap();
        let lib = SessionLibrary {
            sessions: vec![profile.clone()],
            version: "1.1.0".to_string(),
        };
        std::fs::create_dir_all(sessions_path.parent().unwrap()).unwrap();
        std::fs::write(&sessions_path, serde_json::to_string_pretty(&lib).unwrap()).unwrap();

        // 3) Restart: load model and verify prefill loads password back from vault.
        let mut model = rust_ssh::app::AppModel::load();
        let derived_master = SecretString::from(meta.verifier_hash.clone());
        model
            .fill_draft_from_profile(&profile, Some(&derived_master))
            .unwrap();
        assert_eq!(model.draft.password.expose_secret(), "ssh-pass");
    });
}

/// 用例：`sync_ssh_credentials` 传入空凭据时，应删除已有 vault 条目。
///
/// 作用：
/// - 验证“显式清空凭据”语义，确保不会留下历史敏感信息。
///
/// 原理：
/// - 先写入一条密码凭据，拿到 `credential_id`。
/// - 再以 `password=None, passphrase=None` 调用 `sync_ssh_credentials`。
/// - 断言返回 `None` 且后续读取该 `credential_id` 为空。
///
/// 针对问题：
/// - 用户清空密码后，旧密码仍残留在 vault（安全与合规风险）。
/// - 业务层误把“空输入”当“保持不变”导致行为不一致。
#[test]
fn update_clear_credentials_deletes_vault_entry() {
    with_temp_home(|| {
        let user_pwd = SecretString::new("master-pass".into());
        let meta = VaultManager::setup_vault(&user_pwd).unwrap();
        let mut settings = Settings::default();
        settings.security.vault = Some(meta.clone());

        let sid = "sess-2";
        let pw = SecretString::new("ssh-pass".into());
        let runtime_master = SecretString::from(meta.verifier_hash.clone());
        let cid = rust_ssh::vault::session_credentials::sync_ssh_credentials(
            &runtime_master,
            sid,
            Some(pw.expose_secret()),
            None,
            KdfMemoryLevel::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(cid, "ssh:sess-2");

        // Clear: sync with no secrets should delete.
        let cleared = rust_ssh::vault::session_credentials::sync_ssh_credentials(
            &runtime_master,
            sid,
            None,
            None,
            KdfMemoryLevel::default(),
        )
        .unwrap();
        assert!(cleared.is_none());

        let loaded = rust_ssh::vault::session_credentials::load_ssh_credentials(
            &runtime_master,
            "ssh:sess-2",
            KdfMemoryLevel::default(),
        )
        .unwrap();
        assert!(loaded.is_none());
    });
}

/// 用例：模拟删除 profile 后，同步清理 vault 中对应凭据，不产生孤儿数据。
///
/// 作用：
/// - 校验删除路径的数据一致性：会话删除与凭据删除必须配套。
///
/// 原理：
/// - 先写入一条 session 对应凭据。
/// - 调用删除凭据 API（模拟删除 profile 时的清理动作）。
/// - 读取并断言凭据已不存在。
///
/// 针对问题：
/// - profile 已删除但 vault 仍残留明文相关密文条目（孤儿凭据）。
/// - 长期运行后 vault 累积脏数据，增加安全审计与维护成本。
#[test]
fn delete_profile_should_leave_no_orphan_credentials() {
    with_temp_home(|| {
        let user_pwd = SecretString::new("master-pass".into());
        let meta = VaultManager::setup_vault(&user_pwd).unwrap();
        let mut settings = Settings::default();
        settings.security.vault = Some(meta.clone());

        let sid = "sess-3";
        let pw = SecretString::new("ssh-pass".into());
        let runtime_master = SecretString::from(meta.verifier_hash.clone());
        let cid = rust_ssh::vault::session_credentials::sync_ssh_credentials(
            &runtime_master,
            sid,
            Some(pw.expose_secret()),
            None,
            KdfMemoryLevel::default(),
        )
        .unwrap()
        .unwrap();

        // Simulate "delete profile": delete the credential id, ensure it's gone.
        rust_ssh::vault::session_credentials::delete_credential(
            &runtime_master,
            &cid,
            KdfMemoryLevel::default(),
        )
        .unwrap();
        let loaded = rust_ssh::vault::session_credentials::load_ssh_credentials(
            &runtime_master,
            &cid,
            KdfMemoryLevel::default(),
        )
        .unwrap();
        assert!(loaded.is_none());
    });
}

/// 用例：`with_master` 路径下，错误运行时主密钥不能读取凭据。
///
/// 作用：
/// - 验证“运行时解锁门禁”是否生效，防止错误凭据解锁被绕过。
///
/// 原理：
/// - 先通过常规路径写入一条凭据。
/// - 使用错误 `master` 调用 `load_ssh_credentials_with_master`，预期返回错误。
/// - 再用正确 `master` 读取，预期成功并拿到原始密码。
///
/// 针对问题：
/// - UI 看似要求解锁，但底层 API 容错过宽导致错误密钥也能读到数据。
/// - 失败路径污染状态：一次错误解锁后意外可读。
#[test]
fn with_master_wrong_password_cannot_unlock_or_read_credentials() {
    with_temp_home(|| {
        let user_pwd = SecretString::new("master-pass".into());
        let meta = VaultManager::setup_vault(&user_pwd).unwrap();
        let mut settings = Settings::default();
        settings.security.vault = Some(meta.clone());

        // Save one credential using the new simplified API.
        let sid = "sess-master-check";
        let pw = SecretString::new("ssh-pass".into());
        let correct_master = SecretString::from(meta.verifier_hash.clone());
        let cid = rust_ssh::vault::session_credentials::save_ssh_credentials(
            &correct_master,
            sid,
            Some(pw.expose_secret()),
            None,
            KdfMemoryLevel::default(),
        )
        .unwrap()
        .unwrap();

        let wrong_master = SecretString::new("wrong-derived-secret".into());

        // Wrong runtime master must fail.
        let wrong = rust_ssh::vault::session_credentials::load_ssh_credentials_with_master(
            &settings,
            &wrong_master,
            &cid,
        );
        assert!(wrong.is_err());

        // Correct runtime master can read.
        let ok = rust_ssh::vault::session_credentials::load_ssh_credentials_with_master(
            &settings,
            &correct_master,
            &cid,
        )
        .unwrap()
        .unwrap();
        assert_eq!(ok.password.as_deref().map(|s| s.as_str()), Some("ssh-pass"));
    });
}

/// 用例：`delete_credential_with_master` 必须校验正确运行时主密钥。
///
/// 作用：
/// - 覆盖删除操作的鉴权边界，确保“删凭据”与“读凭据”同等级别受保护。
///
/// 原理：
/// - 先用正确 `master` 写入凭据。
/// - 用错误 `master` 删除：应失败，且凭据仍可被正确 `master` 读取。
/// - 再用正确 `master` 删除：应成功，读取结果为 `None`。
///
/// 针对问题：
/// - 删除接口鉴权缺失（误删或越权删）。
/// - 错误 `master` 失败后副作用不明确（误删/半删）。
#[test]
fn delete_credential_with_master_requires_correct_master() {
    with_temp_home(|| {
        let user_pwd = SecretString::new("master-pass".into());
        let meta = VaultManager::setup_vault(&user_pwd).unwrap();
        let mut settings = Settings::default();
        settings.security.vault = Some(meta.clone());

        let sid = "sess-delete-master";
        let correct_master = SecretString::from(meta.verifier_hash.clone());
        let wrong_master = SecretString::new("wrong-derived-secret".into());
        let pw = SecretString::new("ssh-pass".into());
        let cid = rust_ssh::vault::session_credentials::sync_ssh_credentials(
            &correct_master,
            sid,
            Some(pw.expose_secret()),
            None,
            KdfMemoryLevel::default(),
        )
        .unwrap()
        .unwrap();

        // Wrong master should fail and keep credential intact.
        let wrong_del = rust_ssh::vault::session_credentials::delete_credential(
            &wrong_master,
            &cid,
            KdfMemoryLevel::default(),
        );
        assert!(wrong_del.is_err());
        let still_there = rust_ssh::vault::session_credentials::load_ssh_credentials(
            &correct_master,
            &cid,
            KdfMemoryLevel::default(),
        )
        .unwrap();
        assert!(still_there.is_some());

        // Correct master can delete.
        rust_ssh::vault::session_credentials::delete_credential(
            &correct_master,
            &cid,
            KdfMemoryLevel::default(),
        )
        .unwrap();
        let gone = rust_ssh::vault::session_credentials::load_ssh_credentials(
            &correct_master,
            &cid,
            KdfMemoryLevel::default(),
        )
        .unwrap();
        assert!(gone.is_none());
    });
}

/// 用例：Key 认证 profile 的 passphrase 写入与回填闭环。
///
/// 作用：
/// - 确保不仅 Password，Key 模式的 `passphrase` 也能通过 vault 正确持久化与恢复。
///
/// 原理：
/// - 用 `sync_ssh_credentials_with_master` 写入 `passphrase`（密码字段为空）。
/// - 构造 `AuthMethod::Key` 的 profile（携带私钥路径和 `credential_id`）。
/// - 调用 `fill_draft_from_profile`，断言私钥路径与 `passphrase` 均被正确回填。
///
/// 针对问题：
/// - Key 模式功能“看起来支持，实际不回填 passphrase”。
/// - 多认证类型支持不均衡，导致连接体验回退。
#[test]
fn key_profile_passphrase_roundtrip_and_prefill() {
    with_temp_home(|| {
        let user_pwd = SecretString::new("master-pass".into());
        let meta = VaultManager::setup_vault(&user_pwd).unwrap();
        let mut settings = Settings::default();
        settings.security.vault = Some(meta.clone());
        settings.save().unwrap();

        let profile_id = "sess-key-1";
        let key_path = "/tmp/test_id_ed25519";
        let passphrase = SecretString::new("key-passphrase".into());
        let runtime_master = SecretString::from(meta.verifier_hash.clone());
        let credential_id = rust_ssh::vault::session_credentials::sync_ssh_credentials(
            &runtime_master,
            profile_id,
            None,
            Some(passphrase.expose_secret()),
            KdfMemoryLevel::default(),
        )
        .unwrap();

        let profile = SessionProfile {
            id: profile_id.to_string(),
            name: "key-profile".to_string(),
            group_id: None,
            folder: None,
            color_tag: None,
            transport: TransportConfig::Ssh(SshConfig {
                host: "example.com".to_string(),
                port: 22,
                user: "u".to_string(),
                auth: AuthMethod::Key {
                    private_key_path: key_path.to_string(),
                },
                credential_id,
            }),
        };

        let mut model = rust_ssh::app::AppModel::load();
        model
            .fill_draft_from_profile(&profile, Some(&runtime_master))
            .unwrap();
        assert_eq!(model.draft.private_key_path, key_path);
        assert_eq!(model.draft.passphrase.expose_secret(), "key-passphrase");
    });
}
