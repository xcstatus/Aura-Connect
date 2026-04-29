//! SSH session management: connection, authentication, and channel multiplexing.

use anyhow::Result;
use russh::*;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::mpsc;

mod channel;
mod traits;

pub use channel::{BaseSshConnection, SshChannel};
pub use traits::{AsyncSession, SessionCmd};

// ============================================================================
// Error types
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum SshConnectError {
    #[error("ssh auth failed")]
    AuthFailed,
    #[error("ssh agent unsupported")]
    AgentUnsupported,
    #[error("ssh agent unavailable")]
    AgentUnavailable,
    #[error("private key path is empty")]
    KeyPathEmpty,
    #[error("private key not found")]
    KeyNotFound,
    #[error("private key permission denied")]
    KeyPermissionDenied,
    #[error("host key unknown: {host}:{port} {algo} {fingerprint}")]
    HostKeyUnknown {
        host: String,
        port: u16,
        algo: String,
        fingerprint: String,
    },
    #[error("host key mismatch: {host}:{port} {algo} old={old_fingerprint} new={new_fingerprint}")]
    HostKeyMismatch {
        host: String,
        port: u16,
        algo: String,
        old_fingerprint: String,
        new_fingerprint: String,
    },
    #[error("connection disconnected")]
    Disconnected,
}

// ============================================================================
// DNS resolution and connection
// ============================================================================

/// 解析 `host:port`，**IPv4 优先**。macOS 上常见：`getaddrinfo` 先返回 IPv6，
/// 而本机到目标仅有 IPv4 路由时，`TcpStream::connect` 会得到 **errno 65 (EHOSTUNREACH)**，
/// 终端里 `ssh` 可能因解析顺序或配置不同而仍能连上。
async fn resolve_ssh_addrs(host: &str, port: u16) -> Result<Vec<SocketAddr>> {
    let host = host.trim();
    if host.is_empty() {
        anyhow::bail!("主机名为空");
    }
    let mut v4 = Vec::new();
    let mut v6 = Vec::new();
    for sa in tokio::net::lookup_host((host, port)).await? {
        match sa {
            SocketAddr::V4(_) => v4.push(sa),
            SocketAddr::V6(_) => v6.push(sa),
        }
    }
    if v4.is_empty() && v6.is_empty() {
        anyhow::bail!("无法解析主机: {host}");
    }
    Ok(v4.into_iter().chain(v6.into_iter()).collect())
}

async fn connect_russh_prefer_ipv4(
    config: Arc<client::Config>,
    host: &str,
    port: u16,
    known_hosts: Vec<crate::settings::KnownHostRecord>,
    host_key_policy: crate::settings::HostKeyPolicy,
) -> Result<client::Handle<Client>> {
    let addrs = resolve_ssh_addrs(host, port).await?;
    let mut last_err: Option<anyhow::Error> = None;
    for addr in addrs {
        log::debug!(
            "[SshSession] 尝试连接 {} (IPv{})",
            addr,
            if addr.is_ipv4() { 4 } else { 6 }
        );
        let handler = Client {
            tx: None,
            host: host.to_string(),
            port,
            known_hosts: known_hosts.clone(),
            host_key_policy,
        };
        match client::connect(config.clone(), addr, handler).await {
            Ok(handle) => return Ok(handle),
            Err(e) => {
                log::warn!("[SshSession] 连接 {} 失败: {}", addr, e);
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("连接失败")))
}

// ============================================================================
// SSH client handler
// ============================================================================

/// SSH 客户端回调处理
pub(crate) struct Client {
    tx: Option<mpsc::UnboundedSender<Vec<u8>>>,
    host: String,
    port: u16,
    known_hosts: Vec<crate::settings::KnownHostRecord>,
    host_key_policy: crate::settings::HostKeyPolicy,
}

impl client::Handler for Client {
    type Error = anyhow::Error;

    fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKey,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let algo = server_public_key.algorithm().to_string();
        let fp = server_public_key
            .fingerprint(russh::keys::HashAlg::Sha256)
            .to_string();
        let host = self.host.clone();
        let port = self.port;
        let known_hosts = self.known_hosts.clone();
        let host_key_policy = self.host_key_policy;

        async move {
            let known_record = known_hosts
                .iter()
                .find(|r| r.host == host && r.port == port);

            match known_record {
                Some(rec) if rec.fingerprint == fp => Ok(true),
                Some(rec) => Err(anyhow::anyhow!(SshConnectError::HostKeyMismatch {
                    host,
                    port,
                    algo,
                    old_fingerprint: rec.fingerprint.clone(),
                    new_fingerprint: fp,
                })),
                None => match host_key_policy {
                    crate::settings::HostKeyPolicy::Strict => {
                        Err(anyhow::anyhow!(SshConnectError::HostKeyUnknown {
                            host,
                            port,
                            algo,
                            fingerprint: fp,
                        }))
                    }
                    crate::settings::HostKeyPolicy::AcceptNew => Ok(true),
                    crate::settings::HostKeyPolicy::Ask => {
                        Err(anyhow::anyhow!(SshConnectError::HostKeyUnknown {
                            host,
                            port,
                            algo,
                            fingerprint: fp,
                        }))
                    }
                },
            }
        }
    }

    fn data(
        &mut self,
        _channel: ChannelId,
        data: &[u8],
        _session: &mut client::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let tx = self.tx.clone();
        async move {
            if let Some(ref tx) = tx {
                let _ = tx.send(data.to_vec());
            }
            Ok(())
        }
    }

    fn disconnected(
        &mut self,
        reason: client::DisconnectReason<Self::Error>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            log::warn!(
                "[Client] Connection to {}:{} disconnected: {:?}",
                self.host,
                self.port,
                reason
            );
            match reason {
                client::DisconnectReason::ReceivedDisconnect(_) => Ok(()),
                client::DisconnectReason::Error(e) => Err(e),
            }
        }
    }
}

// ============================================================================
// SshSession - high-level connection API
// ============================================================================

pub struct SshSession {}

impl SshSession {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn connect_with_auth(
        &self,
        host: &str,
        port: u16,
        user: &str,
        auth: crate::session::AuthMethod,
        password: &str,
        private_key_path: &str,
        passphrase: &str,
        known_hosts: &[crate::settings::KnownHostRecord],
        host_key_policy: crate::settings::HostKeyPolicy,
    ) -> Result<Arc<SshChannel>> {
        let config = Arc::new(client::Config::default());
        let mut handle =
            connect_russh_prefer_ipv4(config, host, port, known_hosts.to_vec(), host_key_policy)
                .await?;

        let authed = match auth {
            crate::session::AuthMethod::Password => {
                handle.authenticate_password(user, password).await?
            }
            crate::session::AuthMethod::Key { .. } => {
                let p = private_key_path.trim();
                if p.is_empty() {
                    anyhow::bail!(SshConnectError::KeyPathEmpty);
                }
                let private_key = match russh::keys::load_secret_key(
                    std::path::Path::new(p),
                    (!passphrase.trim().is_empty()).then_some(passphrase),
                ) {
                    Ok(k) => k,
                    Err(e) => {
                        if let russh::keys::Error::IO(ref ioe) = e {
                            match ioe.kind() {
                                std::io::ErrorKind::NotFound => {
                                    anyhow::bail!(SshConnectError::KeyNotFound);
                                }
                                std::io::ErrorKind::PermissionDenied => {
                                    anyhow::bail!(SshConnectError::KeyPermissionDenied);
                                }
                                _ => {}
                            };
                        }
                        return Err(e.into());
                    }
                };
                let key_with_hash = russh::keys::PrivateKeyWithHashAlg::new(
                    Arc::new(private_key),
                    Some(russh::keys::HashAlg::Sha256),
                );
                handle.authenticate_publickey(user, key_with_hash).await?
            }
            crate::session::AuthMethod::Interactive => {
                let mut resp = handle
                    .authenticate_keyboard_interactive_start(user, None::<String>)
                    .await?;
                let mut success = false;
                loop {
                    match resp {
                        russh::client::KeyboardInteractiveAuthResponse::Success => {
                            success = true;
                            break;
                        }
                        russh::client::KeyboardInteractiveAuthResponse::Failure { .. } => break,
                        russh::client::KeyboardInteractiveAuthResponse::InfoRequest {
                            prompts,
                            ..
                        } => {
                            let answers = prompts
                                .iter()
                                .map(|_p| password.to_string())
                                .collect::<Vec<_>>();
                            resp = handle
                                .authenticate_keyboard_interactive_respond(answers)
                                .await?;
                        }
                    }
                }
                if success {
                    client::AuthResult::Success
                } else {
                    client::AuthResult::Failure {
                        remaining_methods: russh::MethodSet::empty(),
                        partial_success: false,
                    }
                }
            }
            crate::session::AuthMethod::Agent => {
                match connect_ssh_agent().await {
                    Some((mut agent, keys)) => {
                        if keys.is_empty() {
                            log::warn!(
                                "[SshSession] SSH agent has no keys, falling back to none auth"
                            );
                            handle.authenticate_none(user).await?
                        } else {
                            let mut authed = false;
                            for key in keys {
                                match handle
                                    .authenticate_publickey_with(user, key, None, &mut agent)
                                    .await
                                {
                                    Ok(client::AuthResult::Success) => {
                                        authed = true;
                                        break;
                                    }
                                    Ok(client::AuthResult::Failure { .. }) => continue,
                                    Err(e) => {
                                        log::debug!("[SshSession] Agent auth failed: {}", e);
                                        break;
                                    }
                                }
                            }
                            if authed {
                                client::AuthResult::Success
                            } else {
                                client::AuthResult::Failure {
                                    remaining_methods: russh::MethodSet::empty(),
                                    partial_success: false,
                                }
                            }
                        }
                    }
                    None => {
                        log::warn!(
                            "[SshSession] Failed to connect to SSH agent, falling back to none auth"
                        );
                        handle.authenticate_none(user).await?
                    }
                }
            }
        };

        if !matches!(authed, client::AuthResult::Success) {
            anyhow::bail!(SshConnectError::AuthFailed);
        }

        let base = BaseSshConnection::from_handle(handle);
        let channel = base.open_channel(80, 24).await?;
        Ok(Arc::new(channel))
    }

    pub async fn connect(
        &self,
        host: &str,
        port: u16,
        user: &str,
        password: &str,
    ) -> Result<Arc<SshChannel>> {
        self.connect_with_auth(
            host,
            port,
            user,
            crate::session::AuthMethod::Password,
            password,
            "",
            "",
            &[],
            crate::settings::HostKeyPolicy::Strict,
        )
        .await
    }

    /// 连接并返回 BaseSshConnection（不含 channel），用于连接池多路复用。
    pub async fn connect_base(
        &self,
        host: &str,
        port: u16,
        user: &str,
        auth: crate::session::AuthMethod,
        password: &str,
        private_key_path: &str,
        passphrase: &str,
        known_hosts: &[crate::settings::KnownHostRecord],
        host_key_policy: crate::settings::HostKeyPolicy,
    ) -> anyhow::Result<Arc<BaseSshConnection>> {
        let config = Arc::new(client::Config::default());
        let mut handle =
            connect_russh_prefer_ipv4(config, host, port, known_hosts.to_vec(), host_key_policy)
                .await?;

        let authed = match auth {
            crate::session::AuthMethod::Password => {
                handle.authenticate_password(user, password).await?
            }
            crate::session::AuthMethod::Key { .. } => {
                let p = private_key_path.trim();
                if p.is_empty() {
                    anyhow::bail!(SshConnectError::KeyPathEmpty);
                }
                let private_key = match russh::keys::load_secret_key(
                    std::path::Path::new(p),
                    (!passphrase.trim().is_empty()).then_some(passphrase),
                ) {
                    Ok(k) => k,
                    Err(e) => return Err(e.into()),
                };
                let key_with_hash = russh::keys::PrivateKeyWithHashAlg::new(
                    Arc::new(private_key),
                    Some(russh::keys::HashAlg::Sha256),
                );
                handle.authenticate_publickey(user, key_with_hash).await?
            }
            crate::session::AuthMethod::Interactive => {
                let mut resp = handle
                    .authenticate_keyboard_interactive_start(user, None::<String>)
                    .await?;
                let mut success = false;
                loop {
                    match resp {
                        russh::client::KeyboardInteractiveAuthResponse::Success => {
                            success = true;
                            break;
                        }
                        russh::client::KeyboardInteractiveAuthResponse::Failure { .. } => break,
                        russh::client::KeyboardInteractiveAuthResponse::InfoRequest {
                            prompts,
                            ..
                        } => {
                            let answers = prompts
                                .iter()
                                .map(|_p| password.to_string())
                                .collect::<Vec<_>>();
                            resp = handle
                                .authenticate_keyboard_interactive_respond(answers)
                                .await?;
                        }
                    }
                }
                if success {
                    client::AuthResult::Success
                } else {
                    client::AuthResult::Failure {
                        remaining_methods: russh::MethodSet::empty(),
                        partial_success: false,
                    }
                }
            }
            crate::session::AuthMethod::Agent => {
                match connect_ssh_agent().await {
                    Some((mut agent, keys)) => {
                        if keys.is_empty() {
                            handle.authenticate_none(user).await?
                        } else {
                            let mut authed = false;
                            for key in keys {
                                match handle
                                    .authenticate_publickey_with(user, key, None, &mut agent)
                                    .await
                                {
                                    Ok(client::AuthResult::Success) => {
                                        authed = true;
                                        break;
                                    }
                                    Ok(client::AuthResult::Failure { .. }) => continue,
                                    Err(e) => {
                                        log::debug!("[SshSession] Agent auth failed: {}", e);
                                        break;
                                    }
                                }
                            }
                            if authed {
                                client::AuthResult::Success
                            } else {
                                client::AuthResult::Failure {
                                    remaining_methods: russh::MethodSet::empty(),
                                    partial_success: false,
                                }
                            }
                        }
                    }
                    None => handle.authenticate_none(user).await?,
                }
            }
        };

        if !matches!(authed, client::AuthResult::Success) {
            anyhow::bail!(SshConnectError::AuthFailed);
        }

        Ok(BaseSshConnection::from_handle(handle))
    }
}

// ============================================================================
// Keyboard-interactive authentication
// ============================================================================

#[derive(Debug, Clone)]
pub struct KeyboardInteractivePrompt {
    pub prompt: String,
    pub echo: bool,
}

#[derive(Debug, Clone)]
pub struct KeyboardInteractiveInfo {
    pub name: String,
    pub instructions: String,
    pub prompts: Vec<KeyboardInteractivePrompt>,
}

#[derive(Debug, Clone)]
pub enum KeyboardInteractiveStep {
    Success,
    Failure,
    InfoRequest(KeyboardInteractiveInfo),
}

/// A connect/auth session that can be driven step-by-step for keyboard-interactive flows.
pub struct InteractiveAuthSession {
    handle: client::Handle<Client>,
    user: String,
}

impl InteractiveAuthSession {
    pub async fn connect(
        host: &str,
        port: u16,
        user: &str,
        known_hosts: &[crate::settings::KnownHostRecord],
        host_key_policy: crate::settings::HostKeyPolicy,
    ) -> Result<Self> {
        let config = Arc::new(client::Config::default());
        let handle =
            connect_russh_prefer_ipv4(config, host, port, known_hosts.to_vec(), host_key_policy)
                .await?;
        Ok(Self {
            handle,
            user: user.to_string(),
        })
    }

    pub async fn start(mut self) -> Result<(Self, KeyboardInteractiveStep)> {
        let resp = self
            .handle
            .authenticate_keyboard_interactive_start(self.user.clone(), None::<String>)
            .await?;
        Ok((self, map_kbd_resp(resp)))
    }

    pub async fn respond(
        mut self,
        answers: Vec<String>,
    ) -> Result<(Self, KeyboardInteractiveStep)> {
        let resp = self
            .handle
            .authenticate_keyboard_interactive_respond(answers)
            .await?;
        Ok((self, map_kbd_resp(resp)))
    }

    pub async fn finish_into_session(self, cols: u16, rows: u16) -> Result<SshChannel> {
        let base = BaseSshConnection::from_handle(self.handle);
        let channel = base.open_channel(cols, rows).await?;
        Ok(channel)
    }
}

fn map_kbd_resp(resp: russh::client::KeyboardInteractiveAuthResponse) -> KeyboardInteractiveStep {
    match resp {
        russh::client::KeyboardInteractiveAuthResponse::Success => KeyboardInteractiveStep::Success,
        russh::client::KeyboardInteractiveAuthResponse::Failure { .. } => {
            KeyboardInteractiveStep::Failure
        }
        russh::client::KeyboardInteractiveAuthResponse::InfoRequest {
            name,
            instructions,
            prompts,
        } => KeyboardInteractiveStep::InfoRequest(KeyboardInteractiveInfo {
            name,
            instructions,
            prompts: prompts
                .into_iter()
                .map(|p| KeyboardInteractivePrompt {
                    prompt: p.prompt,
                    echo: p.echo,
                })
                .collect(),
        }),
    }
}

// ============================================================================
// SSH Agent connection
// ============================================================================

/// 连接到 SSH Agent 并获取可用密钥列表
///
/// TODO: SSH_AUTH_SOCK is a Unix domain socket path, but AgentClient requires TcpStream.
/// This will fail silently on most systems. Need to use a stream abstraction or
/// unix-specific agent client that supports UnixStream.
async fn connect_ssh_agent() -> Option<(
    russh::keys::agent::client::AgentClient<tokio::net::TcpStream>,
    Vec<russh::keys::PublicKey>,
)> {
    use russh::keys::agent::client::AgentClient;
    use std::env;

    let agent_socket = env::var("SSH_AUTH_SOCK").ok()?;

    let stream = tokio::net::TcpStream::connect(&agent_socket).await.ok()?;
    let mut agent = AgentClient::connect(stream);

    let keys = agent.request_identities().await.ok()?;

    Some((agent, keys))
}
