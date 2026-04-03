use std::sync::Arc;

use anyhow::Result;
use secrecy::{ExposeSecret, SecretString};

use crate::backend::ssh_session::{AsyncSession, SshSession};
use crate::i18n::{I18n, Locale};
use crate::repository::JsonSessionStore;
use crate::session::{AuthMethod, SessionProfile, TransportConfig};
use crate::session_manager::SessionManager;
use crate::settings::{RecentConnectionRecord, Settings};
use crate::storage::StorageManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DraftSource {
    /// User typed/edited fields directly.
    #[default]
    DirectInput,
    /// Clicked from recent list.
    Recent,
    /// Clicked from a saved profile.
    SavedProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectErrorKind {
    AuthFailed,
    ResolveFailed,
    HostUnreachable,
    HostKeyUnknown,
    HostKeyChanged,
    KeyNotFound,
    KeyPermissionDenied,
    AgentUnsupported,
    AgentUnavailable,
    InvalidPort,
    MissingHostOrUser,
    Unknown,
}

impl ConnectErrorKind {
    pub fn user_message(self) -> &'static str {
        match self {
            ConnectErrorKind::AuthFailed => {
                "认证失败：请检查密码/密钥/交互认证输入，或切换认证方式后重试。"
            }
            ConnectErrorKind::ResolveFailed => {
                "无法解析主机名：请检查 host 是否正确、DNS/网络是否可用。"
            }
            ConnectErrorKind::HostUnreachable => {
                "无法连接到主机：请检查网络、防火墙、端口是否开放。"
            }
            ConnectErrorKind::HostKeyUnknown => "主机指纹未知：请核对指纹后再决定是否信任。",
            ConnectErrorKind::HostKeyChanged => {
                "主机指纹发生变化：存在中间人风险，请核对指纹后再决定是否继续。"
            }
            ConnectErrorKind::KeyNotFound => "找不到私钥文件：请检查私钥路径与权限。",
            ConnectErrorKind::KeyPermissionDenied => {
                "无法读取私钥文件：权限不足，请检查文件权限或改用其他认证方式。"
            }
            ConnectErrorKind::AgentUnsupported => "当前未接入 SSH Agent 认证：请切换认证方式。",
            ConnectErrorKind::AgentUnavailable => {
                "无法使用 SSH Agent：未检测到可用的 agent 或 agent 中没有可用的 key。"
            }
            ConnectErrorKind::InvalidPort => "端口无效：请输入 1–65535。",
            ConnectErrorKind::MissingHostOrUser => "Host/User 不能为空。",
            ConnectErrorKind::Unknown => "连接失败：请检查参数并重试。",
        }
    }
}
pub struct ConnectionDraft {
    pub host: String,
    pub port: String,
    pub user: String,
    pub auth: AuthMethod,
    pub password: SecretString,
    pub private_key_path: String,
    pub passphrase: SecretString,
    /// Where this draft came from; used for UX decisions and retry.
    pub source: DraftSource,
    /// The selected saved profile id (if any).
    pub profile_id: Option<String>,
    /// Recent record snapshot when draft was created from "recent" list.
    pub recent: Option<RecentConnectionRecord>,
    /// Whether user has edited fields after draft creation (preserve origin).
    pub edited: bool,
    /// Latest failure classification (kept for "retry" UX).
    pub last_error: Option<ConnectErrorKind>,
    /// Consecutive password auth failures within the current draft context.
    pub password_error_count: u8,
    /// Host key error details (for Ask/AcceptNew UX).
    pub host_key_error: Option<HostKeyErrorInfo>,
}

impl Default for ConnectionDraft {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: "22".to_string(),
            user: String::new(),
            auth: AuthMethod::Password,
            password: SecretString::from(String::new()),
            private_key_path: String::new(),
            passphrase: SecretString::from(String::new()),
            source: DraftSource::DirectInput,
            profile_id: None,
            recent: None,
            edited: false,
            last_error: None,
            password_error_count: 0,
            host_key_error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostKeyErrorInfo {
    pub host: String,
    pub port: u16,
    pub algo: String,
    pub fingerprint: String,
    pub old_fingerprint: Option<String>,
}

pub struct AppModel {
    pub settings: Settings,
    pub i18n: I18n,
    pub session_manager: SessionManager,
    pub terminal_capture_mode: bool,
    pub command_input: String,
    pub draft: ConnectionDraft,
    pub selected_session_id: Option<String>,
    pub status: String,
    /// Iced runtime-only: cached vault master password for unlocking credentials.
    pub vault_master_password: Option<SecretString>,
}

impl AppModel {
    pub fn load() -> Self {
        let settings = Settings::load();
        let i18n = I18n::new(Locale::from_language_code(&settings.general.language));
        let sessions_path = StorageManager::get_sessions_path()
            .unwrap_or_else(|| std::path::PathBuf::from("sessions.json"));
        let store = Arc::new(JsonSessionStore::new(sessions_path.clone()));
        let mut session_manager = SessionManager::new(store);

        if let Ok(content) = std::fs::read_to_string(&sessions_path) {
            if let Ok(library) = serde_json::from_str::<crate::session::SessionLibrary>(&content) {
                session_manager.library.sessions = library.sessions;
            }
        }

        Self {
            settings,
            i18n,
            session_manager,
            terminal_capture_mode: true,
            command_input: String::new(),
            draft: ConnectionDraft::default(),
            selected_session_id: None,
            status: "Ready".to_string(),
            vault_master_password: None,
        }
    }

    pub fn save_settings(&mut self) -> Result<()> {
        self.settings.save()?;
        Ok(())
    }

    pub fn profiles(&self) -> &[SessionProfile] {
        &self.session_manager.library.sessions
    }

    pub fn select_profile(&mut self, id: String) {
        self.selected_session_id = Some(id.clone());
        if let Some(profile) = self
            .session_manager
            .library
            .sessions
            .iter()
            .find(|s| s.id == id)
        {
            if let TransportConfig::Ssh(ssh) = &profile.transport {
                self.draft.host = ssh.host.clone();
                self.draft.port = ssh.port.to_string();
                self.draft.user = ssh.user.clone();
                self.draft.source = DraftSource::SavedProfile;
                self.draft.profile_id = Some(profile.id.clone());
                self.draft.recent = None;
                self.draft.edited = false;
                self.draft.last_error = None;
                self.draft.password_error_count = 0;
                self.status = format!("Selected profile: {}", profile.name);
            }
        }
    }

    /// 成功连接后写入「最近」列表（去重、截断、`settings` 落盘）。
    pub fn record_recent_connection(&mut self, mut rec: RecentConnectionRecord) {
        rec.last_connected_ms = crate::settings::unix_time_ms();
        let max = self.settings.quick_connect.recent_max.clamp(1, 64);
        let recent = &mut self.settings.quick_connect.recent;
        recent.retain(|r| {
            if let (Some(a), Some(b)) = (&r.profile_id, &rec.profile_id) {
                a != b
            } else if r.profile_id.is_none() && rec.profile_id.is_none() {
                !(r.host == rec.host && r.port == rec.port && r.user == rec.user)
            } else {
                true
            }
        });
        recent.insert(0, rec);
        recent.truncate(max);
        self.settings.save_with_log();
    }

    pub fn recent_connections(&self) -> &[RecentConnectionRecord] {
        &self.settings.quick_connect.recent
    }

    pub fn set_ui_language(&mut self, code: &str) {
        self.settings.general.language = code.to_string();
        self.i18n.set_locale(Locale::from_language_code(code));
    }

    pub(crate) fn classify_connect_error(e: &anyhow::Error) -> ConnectErrorKind {
        if let Some(se) = e.downcast_ref::<crate::backend::ssh_session::SshConnectError>() {
            return match se {
                crate::backend::ssh_session::SshConnectError::AuthFailed => {
                    ConnectErrorKind::AuthFailed
                }
                crate::backend::ssh_session::SshConnectError::AgentUnsupported => {
                    ConnectErrorKind::AgentUnsupported
                }
                crate::backend::ssh_session::SshConnectError::AgentUnavailable => {
                    ConnectErrorKind::AgentUnavailable
                }
                crate::backend::ssh_session::SshConnectError::KeyPathEmpty
                | crate::backend::ssh_session::SshConnectError::KeyNotFound => {
                    ConnectErrorKind::KeyNotFound
                }
                crate::backend::ssh_session::SshConnectError::KeyPermissionDenied => {
                    ConnectErrorKind::KeyPermissionDenied
                }
                crate::backend::ssh_session::SshConnectError::HostKeyUnknown { .. } => {
                    ConnectErrorKind::HostKeyUnknown
                }
                crate::backend::ssh_session::SshConnectError::HostKeyMismatch { .. } => {
                    ConnectErrorKind::HostKeyChanged
                }
            };
        }
        // Native IO errors (connect / DNS resolution) should map without string matching.
        if let Some(ioe) = e.downcast_ref::<std::io::Error>() {
            use std::io::ErrorKind;
            match ioe.kind() {
                ErrorKind::NotFound => return ConnectErrorKind::KeyNotFound,
                ErrorKind::PermissionDenied => return ConnectErrorKind::KeyPermissionDenied,
                ErrorKind::AddrNotAvailable
                | ErrorKind::AddrInUse
                | ErrorKind::ConnectionRefused
                | ErrorKind::ConnectionAborted
                | ErrorKind::ConnectionReset
                | ErrorKind::TimedOut
                | ErrorKind::NetworkUnreachable
                | ErrorKind::HostUnreachable => return ConnectErrorKind::HostUnreachable,
                ErrorKind::InvalidInput => return ConnectErrorKind::Unknown,
                _ => {}
            }
        }
        // Prefer stable "sentinel" strings we control.
        for s in e.chain().map(|x| x.to_string()) {
            if s.contains("AUTH_FAILED") {
                return ConnectErrorKind::AuthFailed;
            }
            if s.contains("无法解析主机") || s.contains("Name or service not known") {
                return ConnectErrorKind::ResolveFailed;
            }
            if s.contains("私钥路径为空") || s.contains("No such file") {
                return ConnectErrorKind::KeyNotFound;
            }
            if s.contains("SSH Agent 认证暂未接入") {
                return ConnectErrorKind::AgentUnsupported;
            }
        }
        ConnectErrorKind::Unknown
    }

    pub async fn connect_from_draft(
        &mut self,
    ) -> std::result::Result<Box<dyn AsyncSession>, ConnectErrorKind> {
        let host = self.draft.host.trim().to_string();
        let user = self.draft.user.trim().to_string();
        let port: u16 = self
            .draft
            .port
            .trim()
            .parse()
            .map_err(|_| ConnectErrorKind::InvalidPort)?;
        let pwd = self.draft.password.expose_secret().to_string();

        if host.is_empty() || user.is_empty() {
            self.draft.last_error = Some(ConnectErrorKind::MissingHostOrUser);
            return Err(ConnectErrorKind::MissingHostOrUser);
        }
        let mut session = SshSession::new();
        match session
            .connect_with_auth(
                &host,
                port,
                &user,
                self.draft.auth.clone(),
                &pwd,
                self.draft.private_key_path.trim(),
                self.draft.passphrase.expose_secret(),
                &self.settings.security.known_hosts,
            )
            .await
        {
            Ok(()) => {
                self.draft.last_error = None;
                self.draft.password_error_count = 0;
                self.draft.host_key_error = None;
                Ok(Box::new(session))
            }
            Err(e) => {
                let kind = Self::classify_connect_error(&e);
                self.draft.last_error = Some(kind);
                self.draft.host_key_error = e
                    .downcast_ref::<crate::backend::ssh_session::SshConnectError>()
                    .and_then(|se| match se {
                        crate::backend::ssh_session::SshConnectError::HostKeyUnknown {
                            host,
                            port,
                            algo,
                            fingerprint,
                        } => Some(HostKeyErrorInfo {
                            host: host.clone(),
                            port: *port,
                            algo: algo.clone(),
                            fingerprint: fingerprint.clone(),
                            old_fingerprint: None,
                        }),
                        crate::backend::ssh_session::SshConnectError::HostKeyMismatch {
                            host,
                            port,
                            algo,
                            old_fingerprint,
                            new_fingerprint,
                        } => Some(HostKeyErrorInfo {
                            host: host.clone(),
                            port: *port,
                            algo: algo.clone(),
                            fingerprint: new_fingerprint.clone(),
                            old_fingerprint: Some(old_fingerprint.clone()),
                        }),
                        _ => None,
                    });
                Err(kind)
            }
        }
    }

    /// Fill quick-connect draft from a saved profile and resolve password from vault when applicable.
    /// Sets [`AppModel::status`] with next-step hints. Returns `Err` only when this flow cannot proceed.
    pub fn fill_draft_from_profile(
        &mut self,
        profile: &SessionProfile,
        vault_master_password: Option<&SecretString>,
    ) -> Result<(), String> {
        let TransportConfig::Ssh(ssh) = &profile.transport else {
            return Err(
                "Only SSH sessions can be opened from this quick-connect flow.".to_string(),
            );
        };

        self.selected_session_id = Some(profile.id.clone());
        self.draft.source = DraftSource::SavedProfile;
        self.draft.profile_id = Some(profile.id.clone());
        self.draft.recent = None;
        self.draft.edited = false;
        self.draft.last_error = None;
        self.draft.password_error_count = 0;
        self.draft.host = ssh.host.clone();
        self.draft.port = ssh.port.to_string();
        self.draft.user = ssh.user.clone();
        self.draft.password = SecretString::from(String::new());
        self.draft.auth = ssh.auth.clone();
        self.draft.private_key_path = match &ssh.auth {
            AuthMethod::Key { private_key_path } => private_key_path.clone(),
            _ => String::new(),
        };
        self.draft.passphrase = SecretString::from(String::new());

        match &ssh.auth {
            AuthMethod::Password => {
                let mut loaded_from_vault = false;
                if let Some(cid) = ssh.credential_id.as_deref() {
                    let loaded = if let Some(master) = vault_master_password {
                        crate::vault::session_credentials::load_ssh_credentials_with_master(
                            &self.settings,
                            master,
                            cid,
                        )
                    } else {
                        crate::vault::session_credentials::load_ssh_credentials(&self.settings, cid)
                    };
                    match loaded {
                        Ok(Some(payload)) => {
                            if let Some(pw) = payload.password.filter(|s| !s.is_empty()) {
                                self.draft.password = SecretString::from(pw);
                                loaded_from_vault = true;
                            }
                        }
                        Ok(None) => {}
                        Err(e) => {
                            self.status = format!(
                                "Loaded \"{}\" but could not read saved password: {}. Enter the password manually, then click Connect.",
                                profile.name, e
                            );
                            return Ok(());
                        }
                    }
                }
                if loaded_from_vault {
                    self.status = format!(
                        "Loaded \"{}\". Password was filled from saved credentials — click Connect when ready.",
                        profile.name
                    );
                } else if self.draft.password.expose_secret().is_empty() {
                    self.status = format!(
                        "Loaded \"{}\" (password authentication). Enter the password and click Connect.",
                        profile.name
                    );
                } else {
                    self.status = format!(
                        "Loaded \"{}\". Review the fields and click Connect.",
                        profile.name
                    );
                }
            }
            AuthMethod::Interactive => {
                self.status = format!(
                    "Loaded \"{}\" (keyboard-interactive auth). Click Connect; the server will prompt for credentials.",
                    profile.name
                );
            }
            AuthMethod::Key { .. } => {
                // Passphrase may be stored in vault.
                if let Some(cid) = ssh.credential_id.as_deref() {
                    let loaded = if let Some(master) = vault_master_password {
                        crate::vault::session_credentials::load_ssh_credentials_with_master(
                            &self.settings,
                            master,
                            cid,
                        )
                    } else {
                        crate::vault::session_credentials::load_ssh_credentials(&self.settings, cid)
                    };
                    if let Ok(Some(payload)) = loaded {
                        if let Some(pph) = payload.passphrase.filter(|s| !s.is_empty()) {
                            self.draft.passphrase = SecretString::from(pph);
                        }
                    }
                }
                self.status = format!(
                    "Loaded \"{}\" (public-key auth). Review key path/passphrase and click Connect.",
                    profile.name
                );
            }
            AuthMethod::Agent => {
                return Err(
                    "This profile uses SSH agent authentication, which is not wired in the Iced quick-connect flow yet."
                        .to_string(),
                );
            }
        }
        Ok(())
    }

    pub fn recent_record_for_draft(&self, label: String) -> RecentConnectionRecord {
        self.recent_record_for_draft_with_profile(label, None)
    }

    pub fn recent_record_for_draft_with_profile(
        &self,
        label: String,
        profile_id: Option<String>,
    ) -> RecentConnectionRecord {
        let port = self.draft.port.trim().parse().unwrap_or(22);
        RecentConnectionRecord {
            profile_id,
            label,
            host: self.draft.host.trim().to_string(),
            port,
            user: self.draft.user.trim().to_string(),
            last_connected_ms: 0,
        }
    }

    pub fn recent_record_for_profile(profile: &SessionProfile) -> Option<RecentConnectionRecord> {
        let TransportConfig::Ssh(ssh) = &profile.transport else {
            return None;
        };
        Some(RecentConnectionRecord {
            profile_id: Some(profile.id.clone()),
            label: profile.name.clone(),
            host: ssh.host.clone(),
            port: ssh.port,
            user: ssh.user.clone(),
            last_connected_ms: 0,
        })
    }
}
