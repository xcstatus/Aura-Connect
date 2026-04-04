# 保险箱（Vault）- 技术规格文档

> 模板版本：v1.0
> 维护人：
> 最后更新：2026-04-04

---

## 一、架构概览

### 1.1 模块职责

Vault 模块是 RustSSH 的安全核心，提供 SSH 凭据（密码、私钥口令）的加密存储和运行时访问控制。所有凭据以 AES-256-GCM 加密形式持久化存储在磁盘上，密钥由用户主密码通过 Argon2id 密钥派生函数（KDF）衍生而来。

### 1.2 目录结构

```
src/vault/
├── mod.rs                  # 公共 API 导出
│                          # VaultUnlockResult / VaultUnlockError 类型定义
├── manager.rs              # VaultMeta 管理、密码验证、密钥派生
├── core.rs                 # CredentialVault 核心：加密/解密/rekey/文件持久化
├── session_credentials.rs   # SSH 会话凭据的存取封装
├── crypto.rs               # 底层加密原语：Argon2id + AES-256-GCM
└── error.rs                # VaultError 枚举
```

### 1.3 核心数据结构

| 结构 | 文件 | 用途 |
| :--- | :--- | :--- |
| `VaultMeta` | `manager.rs` | 写入 `settings.json` 的元数据：salt、verifier_hash、encryption_salt |
| `CredentialVault` | `core.rs` | 运行时保险箱实例，持有一对 salt 和加密后的 blob，提供加解密操作 |
| `SshCredentialPayload` | `session_credentials.rs` | 凭据载荷：`{ password: Option<String>, passphrase: Option<String> }` |
| `VaultDiskFormat` | `core.rs` | 磁盘序列化格式：`{ salt: Vec<u8>, encrypted_data: Vec<u8> }` |
| `VaultPayload` | `core.rs` | 内存中序列化内容：`{ credentials: HashMap<String, Vec<u8>> }` |
| `VaultUnlockState` | `iced_app/state.rs` | 解锁弹窗状态机，包含 pending 操作路由 |
| `VaultFlowState` | `iced_app/state.rs` | 初始化/改密流程状态机 |

---

## 二、公开 API

### 2.1 VaultManager（manager.rs）

```rust
/// 初始化保险箱元数据（生成 Salt + verifier hash）
pub fn setup_vault(password: &SecretString) -> anyhow::Result<VaultMeta>

/// 验证主密码是否正确（Argon2id 验证）
pub fn verify_password(password: &SecretString, meta: &VaultMeta) -> bool

/// 从主密码派生出 32-byte 加密密钥（用于 session_credentials）
pub fn derive_master_key(password: &SecretString, salt: &str) -> anyhow::Result<Zeroizing<[u8; 32]>>

/// 从独立 encryption_salt 派生加密密钥（当前未使用，保留扩展）
pub fn derive_encryption_key(password: &SecretString, meta: &VaultMeta) -> anyhow::Result<Zeroizing<[u8; 32]>>
```

### 2.2 CredentialVault（core.rs）

```rust
/// 用主密码初始化新保险箱（生成 salt + 加密空 payload）
pub fn initialize(master_password: &SecretString) -> Result<Self, VaultError>

/// 用主密码解锁已有保险箱（派生密钥并尝试验证明文）
pub fn unlock(&mut self, master_password: &SecretString) -> Result<(), VaultError>

/// 锁定保险箱，清除内存中的密钥
pub fn lock(&mut self)

/// 用新密码重新加密（rekey）：解密 → 重新派生 → 重新加密
pub fn rekey(&mut self, new_password: &SecretString) -> Result<(), VaultError>

/// 读取一条凭据
pub fn get_credential(&self, key: &str) -> Result<Option<Vec<u8>>, VaultError>

/// 写入一条凭据（整体 payload 解密 → 修改 → 重新加密）
pub fn set_credential(&mut self, key: &str, value: &[u8]) -> Result<(), VaultError>

/// 删除一条凭据
pub fn delete_credential(&mut self, key: &str) -> Result<(), VaultError>

/// 持久化到磁盘（pretty JSON 格式）
pub fn save_to_file(&self, path: &Path) -> Result<(), VaultError>

/// 从磁盘加载（不解锁）
pub fn load_from_file(path: &Path) -> Result<Self, VaultError>
```

### 2.3 Session Credentials（session_credentials.rs）

```rust
/// 保存凭据（使用 settings 中的 verifier_hash 作为密钥）
pub fn save_ssh_credentials(
    settings: &Settings, session_id: &str,
    password: Option<&SecretString>, passphrase: Option<&SecretString>,
) -> Result<Option<String>>

/// 同步凭据（有值则写入，为空则删除）
pub fn sync_ssh_credentials(
    settings: &Settings, session_id: &str,
    password: Option<&SecretString>, passphrase: Option<&SecretString>,
) -> Result<Option<String>>

/// 加载凭据
pub fn load_ssh_credentials(
    settings: &Settings, credential_id: &str,
) -> Result<Option<SshCredentialPayload>>

/// 加载凭据（使用显式 master_password）
pub fn load_ssh_credentials_with_master(
    settings: &Settings, master_password: &SecretString, credential_id: &str,
) -> Result<Option<SshCredentialPayload>>

/// 保存凭据（使用显式 master_password）
pub fn save_ssh_credentials_with_master(
    settings: &Settings, master_password: &SecretString, session_id: &str,
    password: Option<&SecretString>, passphrase: Option<&SecretString>,
) -> Result<Option<String>>

/// 同步凭据（使用显式 master_password）
pub fn sync_ssh_credentials_with_master(
    settings: &Settings, master_password: &SecretString, session_id: &str,
    password: Option<&SecretString>, passphrase: Option<&SecretString>,
) -> Result<Option<String>>

/// 删除凭据（使用显式 master_password）
pub fn delete_credential_with_master(
    settings: &Settings, master_password: &SecretString, credential_id: &str,
) -> Result<()>
```

### 2.4 类型与枚举

| 类型 | 定义位置 | 说明 |
| :--- | :--- | :--- |
| `VaultMeta` | `manager.rs` | 保险箱元数据，序列化到 settings |
| `VaultError` | `error.rs` | 错误枚举：Locked / InvalidPassword / EncryptionFailed / DecryptionFailed / SerializationError / DerivationFailed |
| `VaultUnlockError` | `mod.rs` | 解锁错误：WrongPassword / VaultNotInitialized / VaultError |
| `VaultUnlockResult` | `mod.rs` | 解锁成功结果：verifier_hash |
| `VaultStatus` | `iced_app/state.rs` | 运行状态：Uninitialized / Locked / Unlocked / Unavailable |
| `VaultFlowMode` | `iced_app/state.rs` | 设置流程模式：Initialize / ChangePassword |
| `VaultFlowState` | `iced_app/state.rs` | 设置流程状态机 |
| `VaultUnlockState` | `iced_app/state.rs` | 解锁弹窗状态机，含 pending 路由 |

---

## 三、设计决策

### 3.1 加密方案选型

- **KDF**：Argon2id（推荐用于口令派生的 Argon2 变体）
- **对称加密**：AES-256-GCM（带认证的 AEAD 加密）
- **理由**：Argon2id 在侧信道攻击防护和 GPU/ASIC 破解难度上有最佳平衡；AES-256-GCM 提供加密和完整性验证双重保护

### 3.2 认证与加密密钥分离策略

`VaultMeta` 包含两个独立的盐值：

- **`salt`**：用于派生出 `verifier_hash`（PHC 格式的 Argon2id 哈希），仅用于密码验证
- **`encryption_salt`**：用于派生出独立的加密密钥，理论上可与认证密钥分离

当前实现中，vault 文件的加密和解密都使用 `verifier_hash` 作为密码输入，实际派生时只用 `salt`（即 `CredentialVault` 自身的 salt）。`encryption_salt` 字段存在但未被 `core.rs` 使用，仅在 `session_credentials.rs` 的 `derive_credential_key` 注释中保留为设计说明。

> 理想状态应该是：`verifier_hash` 仅用于验证；验证通过后用 `encryption_salt` 派生的独立密钥来加密 vault 数据。这样即使用户密码被暴力破解得到 `verifier_hash`，也无法直接用于解密凭据。

### 3.3 VaultMeta 嵌入 Settings 的理由

将 `VaultMeta`（salt + hash）嵌入 `settings.json` 而非单独文件，优势在于：

1. **原子性**：`settings.json` 的写入本身有原子性保证（rename），VaultMeta 和其他设置同步更新
2. **简化管理**：无需额外跟踪 vault 相关文件的存在性
3. **风险**：settings 文件损坏可能导致所有凭据不可用；建议配合备份策略

### 3.4 解锁链路中的 verifier_hash 作为密钥

`sync_ssh_credentials` 系列函数（不带 `_with_master` 后缀）从 `settings.security.vault.verifier_hash` 获取密码。这是因为在运行时，验证通过后内存中持有的是 `verifier_hash`（由 `vault_master_password` 持有）。这是一种**运行时传递机制**，而非直接暴露用户密码。

### 3.5 UI 异步化策略

KDF 操作（Argon2id）的计算量较大（约 100-500ms），为避免 UI 冻结：

1. `handle_vault_unlock_submit` 立即关闭弹窗
2. 在 `Task::perform` 的 async 闭包中用 `tokio::task::spawn_blocking` 执行 KDF 验证 + vault 解密
3. 结果通过 `Message::VaultUnlockComplete` 回调回主状态机

---

## 四、关键实现细节

### 4.1 密钥派生流程

```
用户输入主密码
    ↓
VaultManager::verify_password()
    ├─ 从 VaultMeta.salt 构造 SaltString
    ├─ Argon2::default().hash_password() → PHC string (verifier_hash)
    └─ Argon2::default().verify_password() → bool

CredentialVault::unlock()
    ├─ 使用 CredentialVault 自身的 salt（≠ VaultMeta.salt）
    ├─ crypto::derive_key(password, self.salt) → Zeroizing<[u8; 32]>
    │   └─ Params::new(65536, 3, 1) + Argon2id + V0x13
    └─ 尝试验证解密 encrypted_data
```

### 4.2 凭据存取流程

**保存流程**：

```
save_ssh_credentials_with_master()
    ↓
load_or_init_unlocked_vault_with_master()
    ├─ CredentialVault::load_from_file(vault_path)
    └─ vault.unlock(master_password)
    ↓
vault.set_credential("ssh:{session_id}", payload_bytes)
    ├─ crypto::decrypt(current_key, encrypted_data)
    ├─ serde_json::from_slice::<VaultPayload>(decrypted)
    ├─ payload.credentials.insert(key, value)
    ├─ serde_json::to_vec(&payload)
    └─ crypto::encrypt(current_key, serialized)
    ↓
vault.save_to_file(vault_path)  ← 全量覆盖写入
```

**加载流程**：

```
load_ssh_credentials_with_master()
    ↓
load_or_init_unlocked_vault_with_master()
    ↓
vault.get_credential("ssh:{session_id}")
    ├─ crypto::decrypt(key, encrypted_data)
    ├─ serde_json::from_slice::<VaultPayload>(decrypted)
    └─ payload.credentials.get(key)
```

### 4.3 rekey 流程（改密）

```
CredentialVault::rekey(new_password)
    ↓
crypto::decrypt(current_key, self.encrypted_data)  ← 用旧密钥解密
    ↓
crypto::generate_salt()  ← 生成新 salt
    ↓
crypto::derive_key(new_password, new_salt)  ← 用新密码+新 salt 派生新密钥
    ↓
crypto::encrypt(new_key, decrypted)  ← 用新密钥加密旧数据
    ↓
self.salt = new_salt
self.encrypted_data = new_encrypted
self.unlocked_key = Some(new_key)
```

### 4.4 异步解锁消息流

```
Message::VaultUnlockSubmit
    ↓  [handle_vault_unlock_submit]
立即关闭弹窗 → Task::perform(background_task, Message::VaultUnlockComplete)
    ↓  [spawn_blocking 线程]
VaultManager::verify_password()  ← KDF 验证
    ↓
load_or_init_unlocked_vault_with_master()  ← 加载并解锁 vault
    ↓  [handle_vault_unlock_complete]
设置 vault_master_password → 路由到 pending 操作
    ├─ pending_connect → ConnectPressed
    ├─ pending_delete → DeleteSessionProfile
    ├─ pending_save → SessionEditorSave
    └─ pending_save_credentials → sync_ssh_credentials_with_master
```

### 4.5 线程安全约束

- `CredentialVault` 实例在单线程中使用（tokio 任务内）
- `Zeroizing<[u8; 32]>` 确保密钥在 drop 时被清零
- vault 文件通过文件系统锁防止并发写入（依赖 OS 层面保证）

---

## 五、安全考量

### 5.1 威胁模型

**已防御的威胁**：

- 磁盘数据泄露（vault 文件加密存储）
- 彩虹表攻击（使用随机 salt）
- 暴力破解（Argon2id 的计算成本）
- 内存残留（`Zeroizing` 包裹密钥）

**未防御的威胁**：

- 运行时内存扫描（`vault_master_password` 以 `SecretString` 存在，但 Rust 运行时内存保护有限）
- 侧信道攻击（Argon2id 的实现是否为恒定时间？）
- vault 文件篡改（缺少完整性校验）
- MITM 攻击（与 SSH 主机密钥验证分开）

### 5.2 内存安全

| 字段 | 类型 | 清理策略 |
| :--- | :--- | :--- |
| `unlocked_key` | `Option<Zeroizing<[u8; 32]>>` | `Zeroizing` 自动清零 |
| `encrypted_data` | `Vec<u8>` | 普通 Rust drop，无主动清零 |
| `decrypted` / `serialized` 中间 buffer | `Vec<u8>` | 普通 Rust drop |
| `SshCredentialPayload.password` | `String` | 普通 drop |

`Zeroizing` 仅覆盖了密钥字段，payload 中的明文数据（`decrypted`、`serialized`、`SshCredentialPayload`）以普通 `Vec<u8>` 或 `String` 存在。虽然 Rust 的 drop 会在作用域结束时释放内存，但不会主动覆写。建议对高安全要求场景引入 `zeroize::Zeroizing` wrapper 包裹所有明文凭据。

### 5.3 已知安全局限

1. **无完整性校验**：vault 文件只校验 salt 长度，缺少整体 checksum 或 HMAC
2. **认证/加密密钥未真正分离**：`encryption_salt` 字段存在但未使用
3. **verifier_hash 作为运行时密钥**：虽然不是用户密码本身，但仍是一个高熵 secret，存储在内存中
4. **改密时的数据丢失**（见优化项）

---

## 六、与其他模块的边界

| 边界接口 | 协作模块 | 交互方式 |
| :--- | :--- | :--- |
| VaultMeta 持久化 | `settings` | 写入/读取 `settings.security.vault` |
| Vault 文件存储 | `storage` | `StorageManager::get_vault_path()` |
| 运行时密钥持有 | `iced_app/state.rs` | `model.vault_master_password: Option<SecretString>` |
| UI 层消息处理 | `iced_app/update/vault.rs` | Message 枚举 + handle_* 函数 |
| SSH 会话凭据关联 | `session` | `SessionProfile.transport` 中的 `credential_id` |

**本模块不做什么**：

- 不管理 SSH 私钥文件本身（只存储 passphrase）
- 不负责主机密钥验证（由 SSH backend 模块处理）
- 不提供加密数据传输通道（SSH 协议自身负责）

---

## 七、性能特征

### 7.1 主要操作的复杂度

| 操作 | 时间复杂度 | 说明 |
| :--- | :--- | :--- |
| `verify_password` | O(Argon2) ~100-500ms | KDF 计算，参数：m=65536, t=3, p=1 |
| `unlock`（不含验证） | O(Argon2) ~100-500ms | 同上，派生解密密钥 |
| `rekey` | O(2×Argon2 + AES) | 一次解密 + 一次派生 + 一次加密 |
| `set_credential` | O(AES + serde) | 全量解密 + 修改 + 全量加密 |
| `get_credential` | O(AES + serde) | 全量解密 + 查询 |
| `save_to_file` | O(n) | n = encrypted_data 字节数 |
| `load_from_file` | O(n) | 读取 + serde_json 解析 |

### 7.2 IO 模式

- **每次凭据操作（保存/删除）**：完整读取 vault 文件 → 全量解密 → 修改 → 全量加密 → 完整写入
- **文件格式**：pretty-printed JSON（`serde_json::to_string_pretty`），含缩进和换行
- **vault 文件大小**：固定为 salt(16B) + nonce(12B) + 加密数据

### 7.3 UI 阻塞风险

所有 KDF 操作通过 `spawn_blocking` 在后台线程执行，主 UI 线程不受影响。但用户点击"解锁"后到结果返回之间（约 100-500ms），弹窗处于关闭状态，用户无法取消操作。

---

## 八、测试策略

| 测试类型 | 覆盖内容 | 测试文件位置 |
| :--- | :--- | :--- |
| 单元测试 | Vault 生命周期（init/unlock/lock/rekey） | `src/vault/core.rs` 的 `mod tests` |
| 单元测试 | 持久化（save/load 往返） | `src/vault/core.rs` 的 `mod tests` |
| 单元测试 | 改密（rekey） | `src/vault/core.rs` 的 `mod tests` |
| 单元测试 | 加密/解密往返 | `src/vault/crypto.rs` 的 `mod tests` |
| 单元测试 | 密钥派生一致性 | `src/vault/crypto.rs` 的 `mod tests` |

当前测试覆盖了核心加密逻辑，但缺少：

- UI 层集成测试（`update/vault.rs` 中的异步流程）
- 多凭据场景下的 rekey 测试
- 并发访问 vault 文件的压力测试

---

## 九、变更历史

| 日期 | 版本 | 变更内容 | 作者 |
| :--- | :--- | :--- | :--- |
| 2026-04-04 | v1.0 | 初始文档编写，基于代码分析 | AI Assistant |
| 2026-04-04 | v1.1 | 增加性能特征与异步消息流章节 | AI Assistant |
