//! SSH channel multiplexing: BaseSshConnection + SshChannel
//!
//! `BaseSshConnection` manages an authenticated SSH connection with multiple
//! PTY channels. Each tab gets a `SshChannel` that implements `AsyncSession`.

use anyhow::Result;
use russh::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{RwLock, mpsc};

use super::Client;
use super::traits::{AsyncSession, SessionCmd};

/// 单个 Channel 的运行状态（用 Arc 共享，使 SshChannel 可直接访问）
pub(crate) struct ChannelEntry {
    /// 数据接收器：pump 循环通过此发送数据给上层（SshChannel 通过此读取）
    pub(crate) data_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<Vec<u8>>>>,
    /// 命令发送器：上层通过此发送数据给 pump 循环
    pub(crate) cmd_tx: mpsc::UnboundedSender<SessionCmd>,
    /// 溢出缓冲区：当 mpsc 数据包大于调用者 buffer 时，存储超出的数据
    pub(crate) overflow_buf: Arc<tokio::sync::Mutex<Vec<u8>>>,
}

/// 共享 SSH 连接（Base）：管理已认证的 `client::Handle`，支持多路复用多个 PTY channel。
///
/// 每个 channel 拥有独立的 pump 循环，通过 `Arc<client::Handle>>` 共享底层连接。
/// pump 循环负责监听 channel 事件（Data / ExtendedData / Close）和命令（Data / Resize）。
pub struct BaseSshConnection {
    /// 共享的 russh 客户端句柄
    pub(crate) handle: Arc<client::Handle<Client>>,
    /// 活跃 channel 映射：channel_id → 通道状态
    pub(crate) channels: RwLock<HashMap<ChannelId, ChannelEntry>>,
    /// 每个 channel pump 任务的句柄
    pub(crate) pump_handles: RwLock<HashMap<ChannelId, tokio::task::JoinHandle<()>>>,
    /// 引用计数
    pub(crate) ref_count: AtomicUsize,
    /// 连接健康状态：心跳成功后设为 true，断开后设为 false
    pub(crate) is_healthy: std::sync::atomic::AtomicBool,
}

impl BaseSshConnection {
    /// 从已认证的 handle 创建 BaseSshConnection
    pub(crate) fn from_handle(handle: client::Handle<Client>) -> Arc<Self> {
        Arc::new(Self {
            handle: Arc::new(handle),
            channels: RwLock::new(HashMap::new()),
            pump_handles: RwLock::new(HashMap::new()),
            ref_count: AtomicUsize::new(0),
            is_healthy: std::sync::atomic::AtomicBool::new(true),
        })
    }

    /// 检查连接是否存活
    pub fn is_alive(&self) -> bool {
        self.is_healthy.load(std::sync::atomic::Ordering::SeqCst)
            && self
                .channels
                .try_read()
                .map(|c| !c.is_empty())
                .unwrap_or(false)
    }

    /// 获取连接健康状态
    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 发送心跳检测
    pub async fn send_ping(&self) -> bool {
        match self.handle.send_ping().await {
            Ok(()) => {
                self.is_healthy
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                true
            }
            Err(e) => {
                log::warn!("[BaseSshConnection] Keep-alive failed: {}", e);
                self.is_healthy
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                false
            }
        }
    }

    /// 在此连接上打开一个新的 PTY channel，返回 `SshChannel`
    pub async fn open_channel(self: &Arc<Self>, cols: u16, rows: u16) -> Result<SshChannel> {
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
                        if base_conn.handle.send_ping().await.is_ok() {
                            base_conn.is_healthy.store(true, std::sync::atomic::Ordering::SeqCst);
                        } else {
                            log::warn!("[BaseSshConnection] Keep-alive failed for channel {:?}", channel_id_clone);
                            base_conn.is_healthy.store(false, std::sync::atomic::Ordering::SeqCst);
                            break;
                        }
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
                                    break;
                                }
                            }
                            Some(russh::ChannelMsg::ExtendedData { data, .. }) => {
                                if data_tx.send(data.to_vec()).await.is_err() {
                                    break;
                                }
                            }
                            Some(russh::ChannelMsg::ExitStatus { exit_status }) => {
                                log::info!(
                                    "[BaseSshConnection] Channel {:?} exit_status: {}",
                                    channel_id_clone, exit_status
                                );
                                let mut status = exit_status_for_pump.lock().unwrap();
                                *status = Some(exit_status);
                            }
                            Some(russh::ChannelMsg::ExitSignal { .. }) => {
                                log::info!(
                                    "[BaseSshConnection] Channel {:?} exit_signal received",
                                    channel_id_clone
                                );
                                let mut status = exit_status_for_pump.lock().unwrap();
                                *status = Some(255);
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
    pub(crate) async fn remove_channel(&self, channel_id: ChannelId) {
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
    pub async fn read_channel(&self, channel_id: ChannelId, buffer: &mut [u8]) -> Result<usize> {
        let channels = self.channels.read().await;
        match channels.get(&channel_id) {
            Some(entry) => {
                // 优先从溢出缓冲区读取
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
        self.channels
            .try_read()
            .map(|c| c.contains_key(&channel_id))
            .unwrap_or(false)
    }

    /// 获取活跃 channel 数量
    pub fn channel_count(&self) -> usize {
        self.channels.try_read().map(|c| c.len()).unwrap_or(0)
    }

    /// 获取引用计数
    pub fn ref_count(&self) -> usize {
        self.ref_count.load(Ordering::SeqCst)
    }

    /// 获取共享的 russh client handle（用于 SFTP 等子系统）
    pub fn handle(&self) -> Arc<client::Handle<Client>> {
        self.handle.clone()
    }

    /// 打开 SFTP subsystem channel
    pub async fn open_sftp_channel(
        self: &Arc<Self>,
    ) -> anyhow::Result<russh::Channel<russh::client::Msg>> {
        let handle = self.handle();
        let channel = handle.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let channel_id = channel.id();
        log::info!("[BaseSshConnection] Opened SFTP channel {:?}", channel_id);
        Ok(channel)
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
            is_healthy: std::sync::atomic::AtomicBool::new(
                self.is_healthy.load(std::sync::atomic::Ordering::SeqCst),
            ),
        }
    }
}

// ============================================================================
// SshChannel
// ============================================================================

/// SSH PTY Channel：绑定到 `BaseSshConnection` 的特定 channel_id，实现 `AsyncSession`。
///
/// 每个标签页对应一个 `SshChannel`，多个 `SshChannel` 共享同一个 `BaseSshConnection`。
pub struct SshChannel {
    /// 底层的共享 SSH 连接
    pub(crate) conn: Arc<BaseSshConnection>,
    /// 此 channel 的唯一标识
    pub(crate) channel_id: ChannelId,
    /// 本地读取缓冲区（处理 buffer 不够大的情况）
    pub(crate) read_buffer: Vec<u8>,
    /// 独立引用计数（仅 SshChannel 句柄的计数，用于安全 clone）
    pub(crate) handle_ref_count: Arc<AtomicUsize>,
    /// 退出状态（shell/命令的退出码）
    pub(crate) exit_status: Arc<std::sync::Mutex<Option<u32>>>,
}

impl Clone for SshChannel {
    fn clone(&self) -> Self {
        self.handle_ref_count.fetch_add(1, Ordering::SeqCst);
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

        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            log::warn!(
                "[SshChannel] No tokio runtime in drop context, cleanup skipped for channel {:?}",
                self.channel_id
            );
            return;
        };

        if handle_refs <= 1 {
            let conn = self.conn.clone();
            let channel_id = self.channel_id;
            handle.spawn(async move {
                conn.remove_channel(channel_id).await;
            });
        }

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

    /// 获取底层的 BaseSshConnection（用于 SFTP 初始化）
    pub fn base_connection(&self) -> Arc<BaseSshConnection> {
        self.conn.clone()
    }
}

impl AsyncSession for SshChannel {
    fn read_stream(&mut self, buffer: &mut [u8]) -> Result<usize> {
        if !self.read_buffer.is_empty() {
            let len = std::cmp::min(buffer.len(), self.read_buffer.len());
            buffer[..len].copy_from_slice(&self.read_buffer[..len]);
            if len < self.read_buffer.len() {
                let remaining = self.read_buffer[len..].to_vec();
                self.read_buffer = remaining;
            } else {
                self.read_buffer.clear();
            }
            return Ok(len);
        }

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
