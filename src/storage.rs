use directories::ProjectDirs;
use std::path::PathBuf;

pub struct StorageManager;

impl StorageManager {
    pub fn get_config_dir() -> Option<PathBuf> {
        ProjectDirs::from("com", "rustssh", "RustSsh")
            .map(|proj_dirs| proj_dirs.config_dir().to_path_buf())
    }

    pub fn get_sessions_path() -> Option<PathBuf> {
        Self::get_config_dir().map(|mut p| {
            p.push("sessions.json");
            p
        })
    }

    pub fn get_vault_path() -> Option<PathBuf> {
        Self::get_config_dir().map(|mut p| {
            p.push("vault.json");
            p
        })
    }
}
