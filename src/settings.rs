use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub general: GeneralSettings,
    pub terminal: TerminalSettings,
    pub security: SecuritySettings,
    pub quick_connect: QuickConnectSettings,
}

/// 快速连接「最近」列表中的一条记录（不含密码）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecentConnectionRecord {
    /// 若来自已保存会话则为 profile id；临时连接为 `None`。
    pub profile_id: Option<String>,
    pub label: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    #[serde(default)]
    pub last_connected_ms: i64,
    /// 标记是否为最后会话（用于重启后恢复）
    #[serde(default)]
    pub is_last_session: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QuickConnectSettings {
    /// 「最近」列表最大条数（默认 6，可后续接入设置 UI）。
    pub recent_max: usize,
    pub recent: Vec<RecentConnectionRecord>,
    /// When true (default): only one SSH session may be live; it is tied to the **current** tab, and
    /// switching tabs disconnects the previous tab's session (no title/session mismatch).
    /// When false: each tab may keep its own session and terminal buffer (independent tabs).
    #[serde(default = "default_true")]
    pub single_shared_session: bool,
    /// 连接断开时自动重连
    #[serde(default)]
    pub auto_reconnect: bool,
    /// 最大重连次数
    #[serde(default = "default_reconnect_max_attempts")]
    pub reconnect_max_attempts: u8,
    /// 重连基础延迟秒数（指数退避会以此为基数）
    #[serde(default = "default_reconnect_base_delay")]
    pub reconnect_base_delay_secs: u32,
    /// 是否使用指数退避重连
    #[serde(default = "default_true")]
    pub reconnect_exponential: bool,
    /// 启动时恢复上次会话
    #[serde(default)]
    pub restore_last_session: bool,
}

fn default_reconnect_max_attempts() -> u8 {
    5
}

fn default_reconnect_base_delay() -> u32 {
    2
}

fn default_true() -> bool {
    true
}

impl Default for QuickConnectSettings {
    fn default() -> Self {
        Self {
            recent_max: 6,
            recent: Vec::new(),
            single_shared_session: true,
            auto_reconnect: false,
            reconnect_max_attempts: 5,
            reconnect_base_delay_secs: 2,
            reconnect_exponential: true,
            restore_last_session: false,
        }
    }
}

/// 当前时间 Unix 毫秒。
pub fn unix_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralSettings {
    pub language: String,
    pub theme: String,
    pub accent_color: String,
    pub font_size: f32,
    pub window_opacity: f32,
    pub auto_check_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalSettings {
    pub target_fps: u32,
    pub font_family: String,
    /// Terminal text size (logical px). Drives Iced terminal widget and PTY `cols`/`rows` math.
    pub font_size: f32,
    pub line_height: f32,
    pub color_scheme: String,
    pub scrollback_limit: usize,
    pub right_click_paste: bool,
    /// When true and the remote terminal has enabled DEC mode 2004, wrap paste in `\e[200~` … `\e[201~`.
    /// Default off: the ESC prefix breaks many vim setups and non‑xterm clients; raw UTF‑8 paste is safer.
    pub bracketed_paste: bool,
    /// Whether to keep the selection highlight visible after mouse release.
    pub keep_selection_highlight: bool,
    /// Enable local command history search via Up/Down (supports `%` suffix query).
    pub history_search_enabled: bool,
    /// Enable local file/folder name completion on Tab.
    pub local_path_completion_enabled: bool,
    /// When true, `font_size` / `line_height` / `font_family` affect PTY grid sizing and Iced terminal rendering.
    /// When false, use fixed defaults (14px, 1.28 cell height ratio) to avoid frequent PTY resizes or bad metrics.
    pub apply_terminal_metrics: bool,
}

/// Vault KDF 内存级别（影响解锁速度和安全性）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KdfMemoryLevel {
    /// 平衡模式：16MiB 内存，快速解锁（约 1-2 秒）
    Balanced,
    /// 安全模式：64MiB 内存，更强防护（约 4-6 秒）
    Security,
}

impl Default for KdfMemoryLevel {
    fn default() -> Self {
        Self::Balanced
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecuritySettings {
    pub auto_lock: bool,
    pub lock_on_sleep: bool,
    pub idle_timeout_mins: u32,
    pub use_biometrics: bool,
    pub host_key_policy: HostKeyPolicy,
    /// SSH known hosts (fingerprints). Used by host key policy.
    pub known_hosts: Vec<KnownHostRecord>,
    /// Consent policy for automatic auth probing (SSH Agent / local key).
    pub auto_probe_consent: AutoProbeConsent,
    pub vault: Option<crate::vault::VaultMeta>,
    /// Vault KDF 内存级别
    pub kdf_memory_level: KdfMemoryLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnownHostRecord {
    pub host: String,
    pub port: u16,
    /// Key algorithm name (e.g. `ssh-ed25519`, `rsa-sha2-256`)
    pub algo: String,
    /// SHA256 fingerprint (base64, no padding).
    pub fingerprint: String,
    #[serde(default)]
    pub added_ms: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutoProbeConsent {
    /// Ask the first time before trying agent/key.
    Ask,
    /// Always allow automatic probing.
    AlwaysAllow,
}

impl Default for AutoProbeConsent {
    fn default() -> Self {
        AutoProbeConsent::Ask
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HostKeyPolicy {
    Strict,
    Ask,
    AcceptNew,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language: "zh-CN".to_string(),
            theme: "Dark".to_string(),
            accent_color: "#00FF80".to_string(),
            font_size: 14.0,
            window_opacity: 0.95,
            auto_check_update: true,
        }
    }
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            target_fps: 60,
            font_family: "JetBrains Mono".to_string(),
            font_size: 14.0,
            line_height: 1.4,
            color_scheme: "Default".to_string(),
            scrollback_limit: 10000,
            right_click_paste: true,
            bracketed_paste: false,
            keep_selection_highlight: true,
            history_search_enabled: true,
            local_path_completion_enabled: true,
            apply_terminal_metrics: true,
        }
    }
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            auto_lock: true,
            lock_on_sleep: true,
            idle_timeout_mins: 15,
            use_biometrics: false,
            host_key_policy: HostKeyPolicy::Strict,
            known_hosts: Vec::new(),
            auto_probe_consent: AutoProbeConsent::default(),
            vault: None,
            kdf_memory_level: KdfMemoryLevel::default(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            general: GeneralSettings::default(),
            terminal: TerminalSettings::default(),
            security: SecuritySettings::default(),
            quick_connect: QuickConnectSettings::default(),
        }
    }
}

impl Settings {
    /// 计算第 attempt 次重连的延迟秒数（1-indexed）。
    /// 如果 reconnect_exponential 为 true，使用指数退避：base * 2^(attempt-1)
    /// 最大延迟封顶为 30 秒。
    pub fn reconnect_delay_for_attempt(&self, attempt: u8) -> u32 {
        let base = self.quick_connect.reconnect_base_delay_secs;
        let delay = if self.quick_connect.reconnect_exponential {
            base * (2_u32.pow(attempt.saturating_sub(1) as u32))
        } else {
            base
        };
        delay.min(30)
    }

    pub fn load() -> Self {
        let path = Self::get_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str(&content) {
                    return settings;
                }
            }
        }
        let default = Self::default();
        if let Err(e) = default.save() {
            log::warn!("Failed to create default settings file: {}", e);
        }
        default
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::get_path();
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                log::error!("Failed to create config directory: {}", e);
                return Err(e);
            }
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Atomic write: write to temp file, then rename.
        // This prevents corruption if the process crashes mid-write.
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &content)?;

        // Sync the temp file to disk for durability, then rename atomically.
        #[cfg(unix)]
        {
            // Sync the temp file to disk for durability before rename.
            if let Ok(file) = std::fs::File::open(&tmp_path) {
                let _ = file.sync_all();
            }
        }
        #[cfg(not(unix))]
        {
            // On non-Unix, sync is not available; rename still provides atomicity.
        }

        std::fs::rename(&tmp_path, &path)?;
        Ok(())
    }
    pub fn save_with_log(&self) -> bool {
        if let Err(e) = self.save() {
            log::error!("Failed to save settings: {}", e);
            false
        } else {
            true
        }
    }

    fn get_path() -> PathBuf {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "rustssh", "RustSsh") {
            let mut path = proj_dirs.config_dir().to_path_buf();
            path.push("settings.json");
            path
        } else {
            PathBuf::from("settings.json")
        }
    }
}
