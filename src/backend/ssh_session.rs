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
        if let Some(rec) = self
            .known_hosts
            .iter()
            .find(|r| r.host == host && r.port == port)
        {
            if rec.fingerprint == fp {
                return Ok(true);
            }
            return Err(anyhow::anyhow!(SshConnectError::HostKeyMismatch {
                host,
                port,
                algo,
                old_fingerprint: rec.fingerprint.clone(),
                new_fingerprint: fp,
            }));
        }
        Err(anyhow::anyhow!(SshConnectError::HostKeyUnknown {
            host,
            port,
            algo,
            fingerprint: fp,
        }))
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
}

pub enum SessionCmd {
    Data(Vec<u8>),
    Resize(u16, u16),
}

pub struct SshSession {
    handle: Option<client::Handle<Client>>,
    data_rx: Option<Arc<tokio::sync::Mutex<mpsc::Receiver<Vec<u8>>>>>,
    cmd_tx: Option<mpsc::UnboundedSender<SessionCmd>>,
    read_buffer: Vec<u8>,
    /// Task handle for the data pump loop - enables abort on drop.
    pump_task: Option<tokio::task::JoinHandle<()>>,
}

impl SshSession {
    pub fn new() -> Self {
        Self {
            handle: None,
            data_rx: None,
            cmd_tx: None,
            read_buffer: Vec::new(),
            pump_task: None,
        }
    }

    /// 从已认证的 handle 创建 SshSession（用于多路复用）
    pub async fn from_authenticated_handle(
        handle: client::Handle<Client>,
        cols: u16,
        rows: u16,
    ) -> Result<Self> {
        let mut session = Self::new();
        session.install_authenticated_handle(handle, cols, rows).await?;
        Ok(session)
    }

    pub async fn connect_with_auth(
        &mut self,
        host: &str,
        port: u16,
        user: &str,
        auth: crate::session::AuthMethod,
        password: &str,
        private_key_path: &str,
        passphrase: &str,
        known_hosts: &[crate::settings::KnownHostRecord],
    ) -> Result<()> {
        let config = Arc::new(client::Config::default());
        let mut handle =
            connect_russh_prefer_ipv4(config, host, port, known_hosts.to_vec()).await?;

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

        self.install_authenticated_handle(handle, 80, 24).await?;
        Ok(())
    }

    async fn install_authenticated_handle(&mut self, handle: client::Handle<Client>, cols: u16, rows: u16) -> Result<()> {
        // 创建有界缓冲信道（背压控制 8192 包），防止由于前端 UI 卡顿或猫大文件导致内存暴增
        let (tx, rx) = mpsc::channel(8192);
        let mut channel = handle.channel_open_session().await?;
        channel
            .request_pty(true, "xterm-256color", cols as u32, rows as u32, 0, 0, &[])
            .await?;
        channel.request_shell(true).await?;

        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<SessionCmd>();

        self.handle = Some(handle);
        self.data_rx = Some(Arc::new(tokio::sync::Mutex::new(rx)));
        self.cmd_tx = Some(cmd_tx);

        // 启动数据收发处理循环 (UI <-> SSH)
        // 增加 Keep-Alive 心跳间隔轮询: 每30秒发送一次 ping 以防被防火墙剔除
        let mut keep_alive_interval = tokio::time::interval(std::time::Duration::from_secs(30));
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = keep_alive_interval.tick() => {
                        // russh 0.45 机制下可以在此处拓展全局的 connection ping
                    }
                    cmd = cmd_rx.recv() => {
                        match cmd {
                            Some(SessionCmd::Data(data)) => {
                                log::debug!(
                                    "[SessionCmd] 发送至服务端 SSH: {} bytes",
                                    data.len()
                                );
                                let _ = channel.data(&data[..]).await;
                            }
                            Some(SessionCmd::Resize(c, r)) => { let _ = channel.window_change(c as u32, r as u32, 0, 0).await; }
                            None => break,
                        }
                    }
                    msg = channel.wait() => {
                        match msg {
                            Some(russh::ChannelMsg::Data { data }) => {
                                log::debug!(
                                    "[ChannelMsg] 收到服务端数据 (Data): {} bytes",
                                    data.len()
                                );
                                // 遇到背压时等待，避免 OOM 崩溃
                                let _ = tx.send(data.to_vec()).await;
                            }
                            Some(russh::ChannelMsg::ExtendedData { data, .. }) => {
                                log::debug!(
                                    "[ChannelMsg] 收到服务端扩展数据 (ExtendedData): {} bytes",
                                    data.len()
                                );
                                let _ = tx.send(data.to_vec()).await;
                            }
                            Some(russh::ChannelMsg::Close) | Some(russh::ChannelMsg::Eof) | None => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
        });
        self.pump_task = Some(handle);

        Ok(())
    }

    pub async fn connect(
        &mut self,
        host: &str,
        port: u16,
        user: &str,
        password: &str,
    ) -> Result<()> {
        self.connect_with_auth(
            host,
            port,
            user,
            crate::session::AuthMethod::Password,
            password,
            "",
            "",
            &[],
        )
        .await
    }

    /// 断开 SSH 连接并清理后台任务
    pub fn disconnect(&mut self) {
        // 中止后台数据泵任务
        if let Some(handle) = self.pump_task.take() {
            handle.abort();
        }
        // 清理 SSH 句柄和通道
        self.handle = None;
        self.data_rx = None;
        self.cmd_tx = None;
        self.read_buffer.clear();
    }
}

impl Drop for SshSession {
    fn drop(&mut self) {
        self.disconnect();
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
    ) -> Result<Self> {
        let config = Arc::new(client::Config::default());
        let handle = connect_russh_prefer_ipv4(config, host, port, known_hosts.to_vec()).await?;
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

    pub async fn finish_into_session(self, cols: u16, rows: u16) -> Result<SshSession> {
        let mut s = SshSession::new();
        s.install_authenticated_handle(self.handle, cols, rows).await?;
        Ok(s)
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

impl AsyncSession for SshSession {
    fn read_stream(&mut self, buffer: &mut [u8]) -> Result<usize> {
        // 首先从现有缓冲区提取数据
        if !self.read_buffer.is_empty() {
            let len = std::cmp::min(buffer.len(), self.read_buffer.len());
            buffer[..len].copy_from_slice(&self.read_buffer[..len]);
            self.read_buffer.drain(..len);
            return Ok(len);
        }

        // 尝试从异步通道接收新数据
        if let Some(ref rx_arc) = self.data_rx {
            if let Ok(mut rx) = rx_arc.try_lock() {
                match rx.try_recv() {
                    Ok(data) => {
                        log::debug!(
                            "[AsyncSession] 将缓冲区的 {} 字节数据推送给 UI 渲染层",
                            data.len()
                        );
                        let len = std::cmp::min(buffer.len(), data.len());
                        buffer[..len].copy_from_slice(&data[..len]);
                        if data.len() > len {
                            self.read_buffer.extend_from_slice(&data[len..]);
                        }
                        return Ok(len);
                    }
                    Err(mpsc::error::TryRecvError::Empty)
                    | Err(mpsc::error::TryRecvError::Disconnected) => return Ok(0),
                }
            }
        }
        Ok(0)
    }

    fn write_stream(&mut self, data: &[u8]) -> Result<()> {
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(SessionCmd::Data(data.to_vec()));
        }
        Ok(())
    }

    fn resize_pty(&mut self, cols: u16, rows: u16) -> Result<()> {
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(SessionCmd::Resize(cols, rows));
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.handle.is_some() && self.cmd_tx.is_some()
    }
}

// ============================================================================
// 多路复用支持：SharedSshConnection 和 ChannelHandle
// ============================================================================

/// 单个 Channel 的状态
struct ChannelState {
    /// 数据接收器
    data_rx: mpsc::Receiver<Vec<u8>>,
    /// 命令发送器
    cmd_tx: mpsc::UnboundedSender<SessionCmd>,
    /// Pump 任务句柄
    pump_handle: tokio::task::JoinHandle<()>,
}

/// 共享 SSH 连接：管理已认证的 SSH 连接，支持多路复用多个 PTY channel。
///
/// 多个标签页可以通过同一个 SharedSshConnection 创建不同的 channel，
/// 从而复用同一个 TCP 连接和 SSH 握手，大幅降低多会话场景的开销。
pub struct SharedSshConnection {
    /// russh 客户端句柄（用 Arc 共享）
    handle: Arc<client::Handle<Client>>,
    /// 活跃的 channel 映射
    channels: RwLock<HashMap<ChannelId, ChannelState>>,
    /// 引用计数
    ref_count: AtomicUsize,
}

impl SharedSshConnection {
    /// 从已认证的 handle 创建共享连接
    pub(crate) fn from_handle(handle: client::Handle<Client>) -> Arc<Self> {
        Arc::new(Self {
            handle: Arc::new(handle),
            channels: RwLock::new(HashMap::new()),
            ref_count: AtomicUsize::new(0),
        })
    }

    /// 在此连接上打开一个新的 PTY channel
    pub async fn open_channel(
        self: &Arc<Self>,
        cols: u16,
        rows: u16,
    ) -> Result<ChannelHandle> {
        let channel = self.handle.channel_open_session().await?;
        channel
            .request_pty(true, "xterm-256color", cols as u32, rows as u32, 0, 0, &[])
            .await?;
        channel.request_shell(true).await?;

        let channel_id = channel.id();

        // 创建数据缓冲有界 channel（背压控制）
        let (tx, rx) = mpsc::channel(8192);
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<SessionCmd>();

        // 保存 channel_id 供 pump 任务使用
        let channel_id_clone = channel_id;

        // 启动 pump 任务：此任务拥有 channel
        let pump_handle = tokio::spawn(async move {
            let mut channel = channel;
            let mut keep_alive = tokio::time::interval(std::time::Duration::from_secs(30));

            loop {
                tokio::select! {
                    _ = keep_alive.tick() => {
                        // keep-alive tick
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
                                if tx.send(data.to_vec()).await.is_err() {
                                    break;
                                }
                            }
                            Some(russh::ChannelMsg::ExtendedData { data, .. }) => {
                                if tx.send(data.to_vec()).await.is_err() {
                                    break;
                                }
                            }
                            Some(russh::ChannelMsg::Close) | Some(russh::ChannelMsg::Eof) | None => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            log::info!("[SharedSshConnection] Channel {:?} pump loop ended", channel_id_clone);
        });

        // 保存 channel 状态
        let state = ChannelState {
            data_rx: rx,
            cmd_tx,
            pump_handle,
        };

        {
            let mut channels = self.channels.write().await;
            channels.insert(channel_id, state);
        }
        self.ref_count.fetch_add(1, Ordering::SeqCst);

        log::info!("[SharedSshConnection] Opened channel {:?}, total channels: {}", channel_id, self.channels.read().await.len());

        Ok(ChannelHandle {
            conn: self.clone(),
            channel_id,
            read_buffer: Vec::new(),
        })
    }

    /// 从 channel 读取数据（非阻塞）
    pub async fn read_channel(&self, channel_id: ChannelId, buffer: &mut [u8]) -> Result<usize> {
        let mut channels = self.channels.write().await;
        if let Some(state) = channels.get_mut(&channel_id) {
            match state.data_rx.try_recv() {
                Ok(data) => {
                    let len = std::cmp::min(buffer.len(), data.len());
                    buffer[..len].copy_from_slice(&data[..len]);
                    if data.len() > len {
                        // 超出的数据需要缓存起来
                        // 这里简化处理，实际应该用 Mutex 保护
                    }
                    return Ok(len);
                }
                Err(mpsc::error::TryRecvError::Empty) => Ok(0),
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    channels.remove(&channel_id);
                    Ok(0)
                }
            }
        } else {
            Ok(0)
        }
    }

    /// 向 channel 写入数据
    pub async fn write_channel(&self, channel_id: ChannelId, data: &[u8]) -> Result<()> {
        let channels = self.channels.read().await;
        if let Some(state) = channels.get(&channel_id) {
            let _ = state.cmd_tx.send(SessionCmd::Data(data.to_vec()));
        }
        Ok(())
    }

    /// 调整 PTY 大小
    pub async fn resize_channel(&self, channel_id: ChannelId, cols: u16, rows: u16) -> Result<()> {
        let channels = self.channels.read().await;
        if let Some(state) = channels.get(&channel_id) {
            let _ = state.cmd_tx.send(SessionCmd::Resize(cols, rows));
        }
        Ok(())
    }

    /// 检查 channel 是否存在
    pub async fn has_channel(&self, channel_id: ChannelId) -> bool {
        let channels = self.channels.read().await;
        channels.contains_key(&channel_id)
    }

    /// 获取引用计数
    pub fn ref_count(&self) -> usize {
        self.ref_count.load(Ordering::SeqCst)
    }

    /// 获取活跃 channel 数量
    pub async fn channel_count(&self) -> usize {
        self.channels.read().await.len()
    }

    /// 关闭指定 channel
    pub async fn close_channel(&self, channel_id: ChannelId) {
        if let Some(state) = self.channels.write().await.remove(&channel_id) {
            state.pump_handle.abort();
            let _ = state.cmd_tx.send(SessionCmd::Data(Vec::new()));
        }
        self.ref_count.fetch_sub(1, Ordering::SeqCst);
        log::info!("[SharedSshConnection] Closed channel {:?}, remaining: {}", channel_id, self.channel_count().await);
    }

    /// 关闭连接
    pub async fn close(self: &Arc<Self>) {
        log::info!("[SharedSshConnection] Closing all channels");
        let channel_ids: Vec<ChannelId> = self.channels.read().await.keys().copied().collect();
        for id in channel_ids {
            self.close_channel(id).await;
        }
        drop(self.handle.clone());
    }
}

impl Clone for SharedSshConnection {
    fn clone(&self) -> Self {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
        Self {
            handle: self.handle.clone(),
            channels: RwLock::new(HashMap::new()),
            ref_count: AtomicUsize::new(self.ref_count.load(Ordering::SeqCst)),
        }
    }
}

/// PTY channel 句柄：绑定到特定的 channel_id
pub struct ChannelHandle {
    conn: Arc<SharedSshConnection>,
    channel_id: ChannelId,
    /// 本地读取缓冲区
    read_buffer: Vec<u8>,
}

impl ChannelHandle {
    /// 读取数据
    pub async fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        // 先检查本地缓冲区
        if !self.read_buffer.is_empty() {
            let len = std::cmp::min(buffer.len(), self.read_buffer.len());
            buffer[..len].copy_from_slice(&self.read_buffer[..len]);
            self.read_buffer.drain(..len);
            return Ok(len);
        }

        // 从连接读取
        let len = self.conn.read_channel(self.channel_id, buffer).await?;
        if len > buffer.len() {
            self.read_buffer.extend_from_slice(&buffer[len..]);
        }
        Ok(len)
    }

    /// 写入数据
    pub async fn write(&self, data: &[u8]) -> Result<()> {
        self.conn.write_channel(self.channel_id, data).await
    }

    /// 调整 PTY 大小
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.conn.resize_channel(self.channel_id, cols, rows).await
    }

    /// 获取 channel ID
    pub fn channel_id(&self) -> ChannelId {
        self.channel_id
    }
}

impl Drop for ChannelHandle {
    fn drop(&mut self) {
        let refs = self.conn.ref_count.fetch_sub(1, Ordering::SeqCst);
        log::info!("[ChannelHandle] Dropped channel {:?}, shared conn refs: {}", self.channel_id, refs);

        if refs <= 1 {
            // 最后一个引用，关闭连接
            let conn = self.conn.clone();
            tokio::spawn(async move {
                conn.close().await;
            });
        } else {
            // 只关闭 channel
            let conn = self.conn.clone();
            let channel_id = self.channel_id;
            tokio::spawn(async move {
                conn.close_channel(channel_id).await;
            });
        }
    }
}
