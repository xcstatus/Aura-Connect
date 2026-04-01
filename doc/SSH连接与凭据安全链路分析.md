# SSH 连接与凭据安全链路分析（当前实现）

本文档基于当前代码实现，梳理以下 4 个问题：

1. SSH 信息的保存逻辑  
2. 密码/密钥等敏感字段的存储逻辑  
3. 登录时 SSH 信息、密钥信息的获取与传递逻辑  
4. 与服务器交互中的加密、密钥交换、HMAC、主机密钥、Compression 逻辑  

---

## 1) SSH 信息的保存逻辑

### 1.1 保存入口

- UI 保存动作从 `session editor` 触发，最终进入 `save_session_or_toast()`  
  - 文件：`src/ui/session_editor/shared.rs`
- 保存前做基础校验：
  - `name` 非空
  - `ssh_host` 非空
  - `ssh_user` 非空
  - `ssh_port` 在 `1..=65535`

### 1.2 保存内容（非敏感字段）

保存为 `SessionProfile`（别名 `SessionConfig`）：

- 文件：`src/session.rs`
- SSH 相关结构：
  - `SshConfig { host, port, user, auth, credential_id }`

当前非敏感字段保存路径：

- 先更新内存态 `app.session_manager.library.sessions`
- 再异步 `sm.upsert_session(config)` 落盘
  - 文件：`src/session_manager.rs`
  - 存储实现：`JsonSessionStore`
  - 文件：`src/repository.rs`
  - 落盘位置：`sessions.json`（通过 `StorageManager::get_sessions_path()`）
  - 文件：`src/storage.rs`

---

## 2) 密码/密钥等敏感字段的存储逻辑

### 2.1 敏感字段范围（当前）

在 SSH 配置编辑中，敏感数据主要是：

- 密码认证下的 `ssh_password`
- 私钥认证下的 `ssh_passphrase`（私钥口令）

说明：

- `private_key_path` 是路径字符串，当前仍作为普通字段存储在 session 配置中（非加密）。
- 私钥文件内容本身不由当前应用托管（由用户本地文件系统管理）。

### 2.2 Vault 存储实现

新增了专用模块：

- 文件：`src/vault/session_credentials.rs`
- 核心接口：
  - `save_ssh_credentials(settings, session_id, password, passphrase) -> Option<credential_id>`
  - `load_ssh_credentials(settings, credential_id) -> Option<SshCredentialPayload>`

存储策略：

- `credential_id` 规则：`ssh:{session_id}`
- 凭据载荷结构：
  - `SshCredentialPayload { password: Option<String>, passphrase: Option<String> }`
- 载荷序列化后写入 `CredentialVault::set_credential(...)`
  - 文件：`src/vault/core.rs`
- Vault 文件位置：`vault.json`
  - 文件：`src/storage.rs`

### 2.3 加密机制（Vault 内部）

Vault 采用：

- KDF：Argon2id（派生 32-byte key）
- 对称加密：AES-256-GCM
- 每次加密使用随机 nonce
- 磁盘存储包含：
  - salt
  - encrypted_data

实现文件：

- `src/vault/crypto.rs`
- `src/vault/core.rs`

### 2.4 当前主密码衔接方式（重要现状）

当前 `session_credentials` 模块中，Vault 解锁口令来源为：

- `settings.security.vault.verifier_hash`
- 文件：`src/vault/session_credentials.rs` 的 `vault_password_from_settings()`

这是一种“可运行的过渡实现”，用于把 session 凭据先接入 Vault。  
但它不是典型的“用户输入主密码 -> 内存解锁态”模型（详见第 4 节的风险与改进）。

---

## 3) 登录时 SSH 信息、密钥信息的获取与传递逻辑

### 3.1 从会话列表发起连接

快速连接 / 会话列表点击 SSH 会话：

- 文件：`src/ui/components/modals.rs`
- 流程：
  1. 读取 `SessionProfile.transport = TransportConfig::Ssh(ssh)`
  2. 调用 `load_prefilled_password(app, ssh)`（新增）
     - 通过 `ssh.credential_id` 从 Vault 读取 `password`
  3. 组装 `ConnectionPromptState { host, port, user, password, ... }`

当前行为：

- 仅自动回填 `password`
- `passphrase` 已可存储，但尚未用于实际连接参数传递

### 3.2 连接弹窗到 SSH 会话建立

- 连接弹窗：`render_connection_modal()`
  - 文件：`src/ui/components/modals.rs`
- 点击“连接”后：
  - 读取 `prompt.host/port/user/password`
  - 调用 `SshSession::connect(&host, port, &user, &password)`
  - 文件：`src/backend/ssh_session.rs`

### 3.3 SSH 建连后数据流

- 建立 russh client 连接
- `authenticate_password`
- 打开 session channel
- `request_pty("xterm-256color", 80, 24, ...)`
- `request_shell`
- 使用 mpsc 通道桥接：
  - UI -> SSH：`write_stream` -> `SessionCmd::Data`
  - SSH -> UI：`ChannelMsg::Data` -> `read_stream`

---

## 4) 与服务器交互式加密、密钥交换、HMAC、主机密钥、Compression 逻辑

本节区分“当前已实现行为”与“协议层实际控制点”。

### 4.1 当前已实现（代码层）

- SSH 底层库：`russh`
  - 文件：`src/backend/ssh_session.rs`
- 客户端配置使用：`client::Config::default()`
- 服务端主机密钥验证：
  - `check_server_key(...)` 当前直接 `Ok(true)`
  - 即：**默认信任所有主机密钥**

### 4.2 现状解读（加密/KEX/HMAC/Compression）

由于使用 `russh` 默认配置，以下参数由库默认协商：

- 密钥交换算法（KEX）
- 对称加密算法（cipher）
- MAC/HMAC
- 压缩（compression）

项目当前代码中 **没有显式策略配置**（例如 preferred algorithms 列表、压缩开关、HMAC 优先级等）。

### 4.3 与“交互式认证”的关系

`AuthMethod` 枚举里有：

- `Password`
- `Key { private_key_path }`
- `Agent`
- `Interactive`

但当前 `SshSession::connect(...)` 仅实现了：

- `authenticate_password`

也就是说：

- 私钥认证 / agent / keyboard-interactive 仅在 UI 数据模型层有定义，
- 尚未接到实际 SSH 认证流程。

### 4.4 主机密钥策略与设置项现状

项目设置里有主机密钥策略枚举：

- `HostKeyPolicy::{Strict, Ask, AcceptNew}`
- 文件：`src/settings.rs`

但当前 `check_server_key` 未使用该策略，实际行为仍是“全部接受”。

---

## 风险与建议（针对当前实现）

### A. 必须尽快收敛的安全风险

1. 主机密钥校验为全放行（MITM 风险）  
2. Vault 解锁链路未采用“用户主密码实时解锁态”模型  
3. SSH 认证方法与 UI 模型不一致（仅 password 真正生效）

### B. 建议的分阶段改造

1) **主机密钥策略落地**

- 在 `check_server_key` 中接入 known_hosts + `HostKeyPolicy`
- Strict / Ask / AcceptNew 行为与设置项一致

2) **Vault 运行时解锁态**

- 应用启动后不自动可读 Vault
- 用户输入主密码解锁，内存持有短生命周期密钥
- 自动锁定/后台锁定策略生效时清空解锁态

3) **认证方式打通**

- `AuthMethod::Key`：读取私钥文件 + 可选 passphrase
- `AuthMethod::Agent`：接 SSH agent
- `AuthMethod::Interactive`：接 keyboard-interactive 回调

4) **算法策略可配置**

- 在 `russh::client::Config` 上显式设定首选算法集（KEX/Cipher/MAC/Compression）
- 与“加密方法”UI 页签打通

---

## 一页结论

- **已完成**：SSH 会话配置保存、敏感字段入 Vault、连接时从 Vault 回填密码。  
- **未完成**：主机密钥严格校验、私钥/交互式认证链路、算法策略与设置页的真实联动。  
- **当前最紧急安全补丁**：把 `check_server_key = Ok(true)` 改造成基于 `HostKeyPolicy` 的严格校验流程。
