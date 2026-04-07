# Tabby Vault 密码保存逻辑分析

## 一、概述

Tabby 的 Vault（保险箱）功能用于安全存储 SSH 密码、私钥密码和密钥文件等敏感信息。主要涉及两类密码：

- **主密码 (Master Password)**: 用于加密整个 Vault 的密码
- **会话密码 (Session Passwords)**: 存储的 SSH 密码、私钥密码等

---

## 二、核心文件

| 文件路径 | 说明 |
|---------|------|
| `tabby-core/src/services/vault.service.ts` | **核心实现** - Vault 加密/解密服务 |
| `tabby-ssh/src/services/passwordStorage.service.ts` | SSH 密码存储服务 |
| `tabby-core/src/services/config.service.ts` | 配置管理服务（含加密配置逻辑） |
| `tabby-settings/src/components/vaultSettingsTab.component.ts` | Vault 设置 UI 组件 |

---

## 三、数据结构

### 3.1 Vault 存储结构 (StoredVault)

```typescript
interface StoredVault {
    version: number   // 格式版本（当前为1）
    contents: string  // Base64 编码的加密内容
    keySalt: string   // PBKDF2 盐值（十六进制，64字节 = 8字节×2）
    iv: string        // AES-CBC 初始向量（十六进制，32字节 = 16字节×2）
}
```

### 3.2 Vault 内容结构 (Vault)

```typescript
interface Vault {
    config: any              // Vault 配置
    secrets: VaultSecret[]   // 存储的密码/密钥数组
}

interface VaultSecret {
    type: string              // 秘密类型
    key: VaultSecretKey       // 查找键
    value: string             // 秘密值
}

// 类型常量
const VAULT_SECRET_TYPE_PASSWORD = 'ssh:password'           // SSH 密码
const VAULT_SECRET_TYPE_PASSPHRASE = 'ssh:key-passphrase'  // 私钥密码
const VAULT_SECRET_TYPE_FILE = 'file'                       // 文件
```

---

## 四、加密算法参数

| 参数 | 值 |
|-----|-----|
| PBKDF2 迭代次数 | 100,000 |
| PBKDF2 哈希算法 | SHA-512 |
| 密钥长度 | 32 字节 (256位) |
| IV 长度 | 16 字节 (128位) |
| 对称加密算法 | AES-256-CBC |

---

## 五、主密码加密/解密流程

### 5.1 密钥派生

```
用户输入密码 + 随机 Salt → PBKDF2 → 32字节密钥
```

```typescript
async function deriveVaultKey(passphrase: string, salt: Buffer): Promise<Buffer> {
    return crypto.pbkdf2(
        Buffer.from(passphrase),
        salt,
        PBKDF_ITERATIONS,    // 100000
        CRYPT_KEY_LENGTH,     // 32
        PBKDF_DIGEST          // sha512
    )
}
```

### 5.2 加密流程

```
┌─────────────────┐
│   用户输入密码    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 生成随机 Salt     │ 8 字节
│ 生成随机 IV       │ 16 字节
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ PBKDF2 密钥派生   │ 密码 + Salt → 密钥
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ AES-256-CBC 加密 │ 明文 → 密文
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────────┐
│ StoredVault                         │
│   version: 1                        │
│   contents: Base64(密文)            │
│   keySalt: hex(Salt)                │
│   iv: hex(IV)                       │
└─────────────────────────────────────┘
```

### 5.3 解密流程

```
��─────────────────────────────────────┐
│ StoredVault                         │
│   contents: Base64(密文)            │
│   keySalt: hex(Salt)                │
│   iv: hex(IV)                       │
└────────┬────────────────────────────┘
         │
         ▼
┌─────────────────┐
│   用户输入密码    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ PBKDF2 密钥派生   │ 密码 + Salt → 密钥
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ AES-256-CBC 解密 │ 密文 → 明文
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  JSON.parse     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Vault 对象      │
└─────────────────┘
```

---

## 六、会话密码保存逻辑

### 6.1 PasswordStorageService

根据 Vault 是否启用，选择不同的存储后端：

```typescript
async savePassword(profile: SSHProfile, password: string, username?: string): Promise<void> {
    const account = username ?? profile.options.user

    if (this.vault.isEnabled()) {
        // 使用 Vault 存储
        const key = this.getVaultKeyForConnection(profile, account)
        this.vault.addSecret({
            type: VAULT_SECRET_TYPE_PASSWORD,
            key,
            value: password
        })
    } else {
        // 使用系统密钥链 (keytar)
        const key = this.getKeytarKeyForConnection(profile)
        keytar.setPassword(key, account, password)
    }
}
```

### 6.2 密钥构造方式

**SSH 连接密码的键：**
```typescript
{
    user: username ?? profile.options.user,  // 用户名
    host: profile.options.host,              // 主机地址
    port: profile.options.port,              // 端口
}
```

**私钥密码的键：**
```typescript
{
    hash: id  // 私钥文件的哈希 ID
}
```

---

## 七、主密码缓存机制

Vault 服务使用模块级变量缓存解密后的主密码：

```typescript
let _rememberedPassphrase: string | null = null

async getPassphrase(): Promise<string> {
    if (!_rememberedPassphrase) {
        // 弹出解锁对话框
        const modal = this.ngbModal.open(UnlockVaultModalComponent)
        const { passphrase, rememberFor } = await modal.result

        // 设置超时后自动清除
        setTimeout(() => {
            _rememberedPassphrase = null
        }, Math.max(1000, rememberFor * 60000))

        _rememberedPassphrase = passphrase
    }
    return _rememberedPassphrase!
}
```

**记住时长选项：** 1分钟、5分钟、15分钟、1小时、1天、1周

---

## 八、数据持久化

### 8.1 存储位置

Vault 数据存储在配置文件的 `vault` 字段中：

```yaml
# ~/.config/tabby/config.yml (Linux)
# %APPDATA%\tabby\config.yml (Windows)
# ~/Library/Application Support/tabby/config.yml (macOS)

vault:
  version: 1
  contents: "base64加密内容..."
  keySalt: "hex盐值..."
  iv: "hex向量..."
```

### 8.2 存储时机

Vault 内容变化时自动保存：

```typescript
vault.contentChanged$.subscribe(() => {
    this.store.vault = vault.store  // 同步到配置存储
    this.save()                      // 触发配置保存
})
```

---

## 九、整体架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                         Tabby 应用                               │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                   PasswordStorageService                        │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │           Vault 启用?                                     │   │
│  │          是                    否                         │   │
│  │          ▼                     ▼                          │   │
│  │   ┌─────────────┐      ┌─────────────┐                   │   │
│  │   │ VaultService│      │  keytar     │                   │   │
│  │   │ (加密存储)  │      │ (系统密钥链) │                   │   │
│  │   └─────────────┘      └─────────────┘                   │   │
│  └──────────────────────────────────���───────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      VaultService                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐    │
│  │ getPassphrase() │ │  PBKDF2    │  │  AES-256-CBC        │    │
│  │ (主密码输入)  │  │ 密钥派生     │  │ 加密/解密            │    │
│  └─────────────┘  └─────────────┘  └─────────────────────┘    │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Vault 对象                                 │   │
│  │  config: {...}                                         │   │
│  │  secrets: [                                             │   │
│  │    { type: 'ssh:password', key: {...}, value: '...' },  │   │
│  │    { type: 'ssh:key-passphrase', key: {...}, value: '...' } │
│  │  ]                                                      │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      ConfigService                              │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                     config.yml                           │   │
│  │  vault: { version, contents, keySalt, iv }              │   │
│  │  ...                                                    │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 十、安全特性总结

| 特性 | 实现方式 |
|-----|---------|
| **密码学基元** | PBKDF2-SHA512 + AES-256-CBC |
| **主密码派生** | 100,000 次迭代，防止暴力破解 |
| **盐值管理** | 每 Vault 随机生成，不可预测 |
| **IV 管理** | 每加密操作随机生成，防止重放攻击 |
| **会话密码存储** | 加密的 Vault 内，明文存储（已加密保护） |
| **密码查找** | user + host + port 组合键 |
| **缓存超时** | 可配置的记住时长，超时自动清除 |
| **持久化** | YAML 配置内嵌加密数据 |