use anyhow::Result;
use async_trait::async_trait;
use russh::*;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use russh_keys::*;
use tokio::sync::{mpsc, RwLock};

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
}

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

/// SSH 客户端回调处理
struct Client {
    tx: Option<mpsc::UnboundedSender<Vec<u8>>>,
    host: String,
    port: u16,
    known_hosts: Vec<crate::settings::KnownHostRecord>,
    host_key_policy: crate::settings::HostKeyPolicy,
}

#[async_trait]
impl client::Handler for Client {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        let algo = server_public_key.name().to_string();
        let fp = server_public_key.fingerprint();
        let host = self.host.clone();
        let port = self.port;

        // 查找 known_hosts 中是否有该主机的记录
        let known_record = self.known_hosts.iter()
            .find(|r| r.host == host && r.port == port);

        match known_record {
            Some(rec) if rec.fingerprint == fp => {
                // 已知主机且指纹匹配 → 始终允许
                Ok(true)
            }
            Some(rec) => {
                // 已知主机但指纹不匹配 → 始终拒绝（安全策略）
                Err(anyhow::anyhow!(SshConnectError::HostKeyMismatch {
                    host,
                    port,
                    algo,
                    old_fingerprint: rec.fingerprint.clone(),
                    new_fingerprint: fp,
                }))
            }
            None => {
                // 未知主机 → 根据策略决定
                match self.host_key_policy {
                    crate::settings::HostKeyPolicy::Strict => {
                        Err(anyhow::anyhow!(SshConnectError::HostKeyUnknown {
                            host,
                            port,
                            algo,
                            fingerprint: fp,
                        }))
                    }
                    crate::settings::HostKeyPolicy::AcceptNew => {
                        // 自动接受新主机
                        Ok(true)
                    }
                    crate::settings::HostKeyPolicy::Ask => {
                        // 未知主机 → 报错让 UI 层弹窗确认
                        Err(anyhow::anyhow!(SshConnectError::HostKeyUnknown {
                            host,
                            port,
                            algo,
                            fingerprint: fp,
                        }))
                    }
                }
            }
        }
    }

    async fn data(
        &mut self,
        _channel: ChannelId,
        data: &[u8],
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        if let Some(ref tx) = self.tx {
            let _ = tx.send(data.to_vec());
        }
        Ok(())
    }
}

/// 面向跨协议（SSH/Serial/Telnet）扩展的核心抽象
pub trait AsyncSession: Send + Sync {
    fn read_stream(&mut self, buffer: &mut [u8]) -> Result<usize>;
    fn write_stream(&mut self, data: &[u8]) -> Result<()>;
    fn resize_pty(&mut self, cols: u16, rows: u16) -> Result<()>;
    fn is_connected(&self) -> bool;
    /// 获取 shell/命令的退出状态（如果已收到）。
    fn exit_status(&self) -> Option<u32>;
}

pub enum SessionCmd {
    Data(Vec<u8>),
    Resize(u16, u16),
}

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
        let mut handle = connect_russh_prefer_ipv4(
            config,
            host,
            port,
            known_hosts.to_vec(),
            host_key_policy,
        )
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
                let kp = match russh_keys::load_secret_key(
                    std::path::Path::new(p),
                    (!passphrase.trim().is_empty()).then_some(passphrase),
                ) {
                    Ok(k) => k,
                    Err(e) => {
                        // `russh_keys` errors are not stable; classify common IO cases for UI.
                        if let russh_keys::Error::IO(ioe) = &e {
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
                handle.authenticate_publickey(user, Arc::new(kp)).await?
            }
            crate::session::AuthMethod::Interactive => {
                let mut resp = handle
                    .authenticate_keyboard_interactive_start(user, None::<String>)
                    .await?;
                // Back-compat minimal loop: respond to prompts using provided password (common case).
                loop {
                    match resp {
                        russh::client::KeyboardInteractiveAuthResponse::Success => break true,
                        russh::client::KeyboardInteractiveAuthResponse::Failure => break false,
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
            }
            crate::session::AuthMethod::Agent => {
                let mut agent = russh_keys::agent::client::AgentClient::connect_env()
                    .await
                    .map_err(|_e| anyhow::anyhow!(SshConnectError::AgentUnavailable))?;
                let identities = agent
                    .request_identities()
                    .await
                    .map_err(|_e| anyhow::anyhow!(SshConnectError::AgentUnavailable))?;
                if identities.is_empty() {
                    anyhow::bail!(SshConnectError::AgentUnavailable);
                }
                let mut ok = false;
                for pk in identities {
                    let (a, r) = handle.authenticate_future(user, pk, agent).await;
                    agent = a;
                    match r {
                        Ok(true) => {
                            ok = true;
                            break;
                        }
                        Ok(false) => {
                            // Not accepted; try next identity.
                        }
                        Err(_e) => {
                            // Agent signing errors are not user-actionable; treat as unavailable.
                            anyhow::bail!(SshConnectError::AgentUnavailable);
                        }
                    }
                }
                ok
            }
        };

        if !authed {
            anyhow::bail!(SshConnectError::AuthFailed);
        }

        // 创建 BaseSshConnection 并打开第一个 channel
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
        let mut handle = connect_russh_prefer_ipv4(
            config,
            host,
            port,
            known_hosts.to_vec(),
            host_key_policy,
        )
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
                let kp = match russh_keys::load_secret_key(
                    std::path::Path::new(p),
                    (!passphrase.trim().is_empty()).then_some(passphrase),
                ) {
                    Ok(k) => k,
                    Err(e) => {
                        return Err(e.into());
                    }
                };
                handle.authenticate_publickey(user, Arc::new(kp)).await?
            }
            crate::session::AuthMethod::Interactive => {
                let mut resp = handle
                    .authenticate_keyboard_interactive_start(user, None::<String>)
                    .await?;
                loop {
                    match resp {
                        russh::client::KeyboardInteractiveAuthResponse::Success => break true,
                        russh::client::KeyboardInteractiveAuthResponse::Failure => break false,
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
            }
            crate::session::AuthMethod::Agent => {
                let mut agent = russh_keys::agent::client::AgentClient::connect_env()
                    .await
                    .map_err(|_e| anyhow::anyhow!(SshConnectError::AgentUnavailable))?;
                let identities = agent
                    .request_identities()
                    .await
                    .map_err(|_e| anyhow::anyhow!(SshConnectError::AgentUnavailable))?;
                if identities.is_empty() {
                    anyhow::bail!(SshConnectError::AgentUnavailable);
                }
                let mut ok = false;
                for pk in identities {
                    let (a, r) = handle.authenticate_future(user, pk, agent).await;
                    agent = a;
                    match r {
                        Ok(true) => {
                            ok = true;
                            break;
                        }
                        Ok(false) => {}
                        Err(_e) => {
                            anyhow::bail!(SshConnectError::AgentUnavailable);
                        }
                    }
                }
                ok
            }
        };

        if !authed {
            anyhow::bail!(SshConnectError::AuthFailed);
        }

        Ok(BaseSshConnection::from_handle(handle))
    }
}

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
        let handle = connect_russh_prefer_ipv4(
            config,
            host,
            port,
            known_hosts.to_vec(),
            host_key_policy,
        )
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
        russh::client::KeyboardInteractiveAuthResponse::Failure => KeyboardInteractiveStep::Failure,
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
// 多路复用支持：BaseSshConnection 和 SshChannel
// ============================================================================

/// 单个 Channel 的运行状态（用 Arc 共享，使 SshChannel 可直接访问）
struct ChannelEntry {
    /// 数据接收器：pump 循环通过此发送数据给上层（SshChannel 通过此读取）
    data_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<Vec<u8>>>>,
    /// 命令发送器：上层通过此发送数据给 pump 循环
    cmd_tx: mpsc::UnboundedSender<SessionCmd>,
    /// 溢出缓冲区：当 mpsc 数据包大于调用者 buffer 时，存储超出的数据
    overflow_buf: Arc<tokio::sync::Mutex<Vec<u8>>>,
}

/// 共享 SSH 连接（Base）：管理已认证的 `client::Handle`，支持多路复用多个 PTY channel。
///
/// 每个 channel 拥有独立的 pump 循环，通过 `Arc<client::Handle>>` 共享底层连接。
/// pump 循环负责监听 channel 事件（Data / ExtendedData / Close）和命令（Data / Resize）。
pub struct BaseSshConnection {
    /// 共享的 russh 客户端句柄
    handle: Arc<client::Handle<Client>>,
    /// 活跃 channel 映射：channel_id → 通道状态
    channels: RwLock<HashMap<ChannelId, ChannelEntry>>,
    /// 每个 channel pump 任务的句柄
    pump_handles: RwLock<HashMap<ChannelId, tokio::task::JoinHandle<()>>>,
    /// 引用计数
    ref_count: AtomicUsize,
}

impl BaseSshConnection {
    /// 从已认证的 handle 创建 BaseSshConnection
    pub(crate) fn from_handle(handle: client::Handle<Client>) -> Arc<Self> {
        Arc::new(Self {
            handle: Arc::new(handle),
            channels: RwLock::new(HashMap::new()),
            pump_handles: RwLock::new(HashMap::new()),
            ref_count: AtomicUsize::new(0),
        })
    }

    /// 在此连接上打开一个新的 PTY channel，返回 `SshChannel`
    pub async fn open_channel(
        self: &Arc<Self>,
        cols: u16,
        rows: u16,
    ) -> Result<SshChannel> {
        let channel = self.handle.channel_open_session().await?;
        channel
            .request_pty(true, "xterm-256color", cols as u32, rows as u32, 0, 0, &[])
            .await?;
        channel.request_shell(true).await?;

        let channel_id = channel.id();

        // 创建数据缓冲有界 channel（背压控制 8192 包）
        let (data_tx, data_rx) = mpsc::channel(8192);
        let data_rx = Arc::new(tokio::sync::Mutex::new(data_rx));
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<SessionCmd>();
        // 创建溢出缓冲区，用于缓存大于调用者 buffer 的数据
        let overflow_buf = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        // 克隆 exit_status Arc 供 pump 任务使用（pump 和 SshChannel 各持有一个）
        let exit_status_arc = Arc::new(std::sync::Mutex::new(None));
        let exit_status_for_pump = exit_status_arc.clone();
        // SshChannel 持有独立的 exit_status Arc（pump 和 SshChannel 不共享 Mutex）

        // 在进入 async move 前先克隆需要的值
        let channel_id_val = channel_id;
        let base_conn_for_pump: Arc<BaseSshConnection> = Arc::clone(self);

        // 启动独立 pump 任务：拥有 channel，监听事件和命令
        let pump_handle = tokio::spawn(async move {
            let mut channel = channel;
            let channel_id_clone = channel_id_val;
            let base_conn = base_conn_for_pump;
            let mut keep_alive = tokio::time::interval(std::time::Duration::from_secs(30));

            loop {
                tokio::select! {
                    _ = keep_alive.tick() => {
                        // keep-alive tick（预留扩展）
                    }
                    cmd = cmd_rx.recv() => {
                        match cmd {
                            Some(SessionCmd::Data(data)) => {
                                if channel.data(&data[..]).await.is_err() {
                                    break;
                                }
                            }
                            Some(SessionCmd::Resize(c, r)) => {
                                let _ = channel.window_change(c as u32, r as u32, 0, 0).await;
                            }
                            None => break,
                        }
                    }
                    msg = channel.wait() => {
                        match msg {
                            Some(russh::ChannelMsg::Data { data }) => {
                                if data_tx.send(data.to_vec()).await.is_err() {
                                    // 接收端已断开，channel 可以关闭
                                    break;
                                }
                            }
                            Some(russh::ChannelMsg::ExtendedData { data, .. }) => {
                                if data_tx.send(data.to_vec()).await.is_err() {
                                    break;
                                }
                            }
                            // 退出码：ExitStatus
                            Some(russh::ChannelMsg::ExitStatus { exit_status }) => {
                                log::info!(
                                    "[BaseSshConnection] Channel {:?} exit_status: {}",
                                    channel_id_clone, exit_status
                                );
                                let mut status = exit_status_for_pump.lock().unwrap();
                                *status = Some(exit_status);
                            }
                            // 退出码：ExitSignal
                            Some(russh::ChannelMsg::ExitSignal { .. }) => {
                                log::info!(
                                    "[BaseSshConnection] Channel {:?} exit_signal received",
                                    channel_id_clone
                                );
                                let mut status = exit_status_for_pump.lock().unwrap();
                                *status = Some(255); // 信号通常映射为 255
                            }
                            Some(russh::ChannelMsg::Close) | Some(russh::ChannelMsg::Eof) | None => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            log::info!(
                "[BaseSshConnection] Channel {:?} pump loop ended",
                channel_id_clone
            );
            // channel 关闭时清理
            base_conn.remove_channel(channel_id_clone).await;
        });

        // 保存 channel 状态
        let entry = ChannelEntry {
            data_rx,
            cmd_tx,
            overflow_buf: overflow_buf.clone(),
        };

        {
            let mut channels = self.channels.write().await;
            channels.insert(channel_id, entry);
        }
        {
            let mut handles = self.pump_handles.write().await;
            handles.insert(channel_id, pump_handle);
        }
        self.ref_count.fetch_add(1, Ordering::SeqCst);

        log::info!(
            "[BaseSshConnection] Opened channel {:?}, total channels: {}",
            channel_id,
            self.channels.read().await.len()
        );

        Ok(SshChannel {
            conn: self.clone(),
            channel_id,
            read_buffer: Vec::new(),
            handle_ref_count: Arc::new(AtomicUsize::new(1)),
            exit_status: exit_status_arc,
        })
    }

    /// 内部：移除 channel 并停止 pump 任务
    async fn remove_channel(&self, channel_id: ChannelId) {
        let had_entry = {
            let mut channels = self.channels.write().await;
            channels.remove(&channel_id)
        };

        if had_entry.is_some() {
            if let Some(h) = self.pump_handles.write().await.remove(&channel_id) {
                h.abort();
            }
            self.ref_count.fetch_sub(1, Ordering::SeqCst);
            log::info!(
                "[BaseSshConnection] Removed channel {:?}, remaining: {}",
                channel_id,
                self.channels.read().await.len()
            );
        }
    }

    /// 从 channel 读取数据（非阻塞）
    pub async fn read_channel(
        &self,
        channel_id: ChannelId,
        buffer: &mut [u8],
    ) -> Result<usize> {
        let channels = self.channels.read().await;
        match channels.get(&channel_id) {
            Some(entry) => {
                // 优先从溢出缓冲区读取（上次读取遗留的数据）
                {
                    let mut overflow = entry.overflow_buf.lock().await;
                    if !overflow.is_empty() {
                        let len = std::cmp::min(buffer.len(), overflow.len());
                        buffer[..len].copy_from_slice(&overflow[..len]);
                        if len < overflow.len() {
                            overflow.drain(..len);
                        } else {
                            overflow.clear();
                        }
                        return Ok(len);
                    }
                }

                // 从 mpsc 接收数据
                let mut rx = entry.data_rx.lock().await;
                match rx.try_recv() {
                    Ok(data) => {
                        let len = std::cmp::min(buffer.len(), data.len());
                        buffer[..len].copy_from_slice(&data[..len]);
                        if data.len() > len {
                            // 修复：将超出的数据存入溢出缓冲区
                            let remaining = data[len..].to_vec();
                            let mut overflow = entry.overflow_buf.lock().await;
                            overflow.extend_from_slice(&remaining);
                        }
                        Ok(len)
                    }
                    Err(mpsc::error::TryRecvError::Empty) => Ok(0),
                    Err(mpsc::error::TryRecvError::Disconnected) => Ok(0),
                }
            }
            None => Ok(0),
        }
    }

    /// 向 channel 写入数据
    pub fn write_channel(&self, channel_id: ChannelId, data: &[u8]) {
        if let Some(ch) = self.channels.try_read().ok() {
            if let Some(entry) = ch.get(&channel_id) {
                let _ = entry.cmd_tx.send(SessionCmd::Data(data.to_vec()));
            }
        }
    }

    /// 调整 PTY 大小
    pub fn resize_channel(&self, channel_id: ChannelId, cols: u16, rows: u16) {
        if let Some(ch) = self.channels.try_read().ok() {
            if let Some(entry) = ch.get(&channel_id) {
                let _ = entry.cmd_tx.send(SessionCmd::Resize(cols, rows));
            }
        }
    }

    /// 检查 channel 是否存在
    pub fn has_channel(&self, channel_id: ChannelId) -> bool {
        self.channels.try_read().map(|c| c.contains_key(&channel_id)).unwrap_or(false)
    }

    /// 获取活跃 channel 数量
    pub fn channel_count(&self) -> usize {
        self.channels.try_read().map(|c| c.len()).unwrap_or(0)
    }

    /// 获取引用计数
    pub fn ref_count(&self) -> usize {
        self.ref_count.load(Ordering::SeqCst)
    }

    /// 关闭连接（关闭所有 channel）
    pub async fn close(self: &Arc<Self>) {
        log::info!("[BaseSshConnection] Closing all channels");
        let channel_ids: Vec<ChannelId> = self.channels.read().await.keys().copied().collect();
        for id in channel_ids {
            self.remove_channel(id).await;
        }
    }
}

impl Clone for BaseSshConnection {
    fn clone(&self) -> Self {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
        Self {
            handle: self.handle.clone(),
            channels: RwLock::new(HashMap::new()),
            pump_handles: RwLock::new(HashMap::new()),
            ref_count: AtomicUsize::new(self.ref_count.load(Ordering::SeqCst)),
        }
    }
}

/// SSH PTY Channel：绑定到 `BaseSshConnection` 的特定 channel_id，实现 `AsyncSession`。
///
/// 每个标签页对应一个 `SshChannel`，多个 `SshChannel` 共享同一个 `BaseSshConnection`。
pub struct SshChannel {
    /// 底层的共享 SSH 连接
    conn: Arc<BaseSshConnection>,
    /// 此 channel 的唯一标识
    channel_id: ChannelId,
    /// 本地读取缓冲区（处理 buffer 不够大的情况）
    read_buffer: Vec<u8>,
    /// 独立引用计数（仅 SshChannel 句柄的计数，用于安全 clone）
    handle_ref_count: Arc<AtomicUsize>,
    /// 退出状态（shell/命令的退出码）
    exit_status: Arc<std::sync::Mutex<Option<u32>>>,
}

impl Clone for SshChannel {
    fn clone(&self) -> Self {
        // 增加句柄引用计数
        self.handle_ref_count.fetch_add(1, Ordering::SeqCst);
        // 增加 BaseSshConnection 的引用计数（安全 clone）
        self.conn.ref_count.fetch_add(1, Ordering::SeqCst);
        Self {
            conn: self.conn.clone(),
            channel_id: self.channel_id,
            read_buffer: Vec::new(),
            handle_ref_count: self.handle_ref_count.clone(),
            exit_status: self.exit_status.clone(),
        }
    }
}

impl Drop for SshChannel {
    fn drop(&mut self) {
        let handle_refs = self.handle_ref_count.fetch_sub(1, Ordering::SeqCst);
        let conn_refs = self.conn.ref_count.fetch_sub(1, Ordering::SeqCst);

        log::info!(
            "[SshChannel] Dropped channel {:?} (handles: {}, conn_refs: {})",
            self.channel_id,
            handle_refs,
            conn_refs,
        );

        // 获取当前 tokio runtime handle
        // 在 shutdown 阶段可能没有 runtime 上下文，此时跳过清理（OS 会自动释放资源）
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            log::warn!(
                "[SshChannel] No tokio runtime in drop context, cleanup skipped for channel {:?}",
                self.channel_id
            );
            return;
        };

        // 只有最后一个句柄才关闭 channel（防止重复 remove_channel）
        if handle_refs <= 1 {
            let conn = self.conn.clone();
            let channel_id = self.channel_id;
            handle.spawn(async move {
                conn.remove_channel(channel_id).await;
            });
        }

        // BaseSshConnection 引用计数 <= 1 时关闭整个连接
        if conn_refs <= 1 {
            let conn = self.conn.clone();
            handle.spawn(async move {
                conn.close().await;
            });
        }
    }
}

impl SshChannel {
    /// 获取 channel ID
    pub fn channel_id(&self) -> ChannelId {
        self.channel_id
    }
}

impl AsyncSession for SshChannel {
    fn read_stream(&mut self, buffer: &mut [u8]) -> Result<usize> {
        // 优先从本地缓冲区读取（不 drain，用 swap 保持类型安全）
        if !self.read_buffer.is_empty() {
            let len = std::cmp::min(buffer.len(), self.read_buffer.len());
            buffer[..len].copy_from_slice(&self.read_buffer[..len]);
            // 修复：保留未读取的尾部字节（不能直接清空，否则会丢失数据）
            if len < self.read_buffer.len() {
                let remaining = self.read_buffer[len..].to_vec();
                self.read_buffer = remaining;
            } else {
                self.read_buffer.clear();
            }
            return Ok(len);
        }

        // 从连接异步读取
        let rt = tokio::runtime::Handle::current();
        rt.block_on(self.conn.read_channel(self.channel_id, buffer))
    }

    fn write_stream(&mut self, data: &[u8]) -> Result<()> {
        self.conn.write_channel(self.channel_id, data);
        Ok(())
    }

    fn resize_pty(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.conn.resize_channel(self.channel_id, cols, rows);
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.conn.has_channel(self.channel_id)
    }

    fn exit_status(&self) -> Option<u32> {
        *self.exit_status.lock().unwrap()
    }
}

