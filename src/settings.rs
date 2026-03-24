use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub general: GeneralSettings,
    pub terminal: TerminalSettings,
    pub security: SecuritySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub language: String,
    pub theme: String,
    pub accent_color: String,
    pub font_size: f32,
    pub window_opacity: f32,
    pub auto_check_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSettings {
    pub gpu_acceleration: bool,
    pub target_fps: u32,
    pub font_family: String,
    pub line_height: f32,
    pub color_scheme: String,
    pub scrollback_limit: usize,
    pub right_click_paste: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    pub auto_lock: bool,
    pub lock_on_sleep: bool,
    pub idle_timeout_mins: u32,
    pub use_biometrics: bool,
    pub host_key_policy: HostKeyPolicy,
    pub vault: Option<crate::vault::VaultMeta>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HostKeyPolicy {
    Strict,
    Ask,
    AcceptNew,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            general: GeneralSettings {
                language: "zh-CN".to_string(),
                theme: "Dark".to_string(),
                accent_color: "#00FF80".to_string(),
                font_size: 14.0,
                window_opacity: 0.95,
                auto_check_update: true,
            },
            terminal: TerminalSettings {
                gpu_acceleration: true,
                target_fps: 60,
                font_family: "JetBrains Mono".to_string(),
                line_height: 1.4,
                color_scheme: "Default".to_string(),
                scrollback_limit: 10000,
                right_click_paste: true,
            },
            security: SecuritySettings {
                auto_lock: true,
                lock_on_sleep: true,
                idle_timeout_mins: 15,
                use_biometrics: false,
                host_key_policy: HostKeyPolicy::Strict,
                vault: None,
            },
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
                eprintln!("Failed to create config directory: {}", e);
                return Err(e);
            }
        }
        let content = serde_json::to_string_pretty(self).unwrap();
        std::fs::write(path, content)
    }

    fn get_path() -> PathBuf {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "rustssh",  "rust-ssh") {
            let mut path = proj_dirs.config_dir().to_path_buf();
            path.push("settings.json");
            path
        } else {
            PathBuf::from("settings.json")
        }
    }
}
