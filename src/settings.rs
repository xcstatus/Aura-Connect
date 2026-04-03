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
    pub gpu_acceleration: bool,
    pub target_fps: u32,
    pub font_family: String,
    /// Terminal text size (logical px). Drives Iced terminal widget and PTY `cols`/`rows` math.
    pub font_size: f32,
    pub line_height: f32,
    pub color_scheme: String,
    pub scrollback_limit: usize,
    /// Optional override path for GPU glyph atlas font (TTF/TTC).
    /// If None, use platform defaults.
    pub gpu_font_path: Option<String>,
    /// Optional face index within TTC collection, used only when `gpu_font_path` is set.
    pub gpu_font_face_index: Option<u32>,
    /// When the GPU glyph atlas is under sustained eviction pressure, allow a best-effort reset to
    /// stabilize long-running sessions (may cause a brief re-rasterization burst).
    pub atlas_reset_on_pressure: bool,
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
            gpu_acceleration: true,
            target_fps: 60,
            font_family: "JetBrains Mono".to_string(),
            font_size: 14.0,
            line_height: 1.4,
            color_scheme: "Default".to_string(),
            scrollback_limit: 10000,
            gpu_font_path: None,
            gpu_font_face_index: None,
            atlas_reset_on_pressure: false,
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
        let _ = default.save();
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
        let content = serde_json::to_string_pretty(self).unwrap();
        std::fs::write(path, content)
    }

    fn get_path() -> PathBuf {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "rustssh", "rust-ssh") {
            let mut path = proj_dirs.config_dir().to_path_buf();
            path.push("settings.json");
            path
        } else {
            PathBuf::from("settings.json")
        }
    }
}
