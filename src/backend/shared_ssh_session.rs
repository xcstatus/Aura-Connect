//! 共享 SSH Session 管理。
//!
//! 支持多个标签页连接到同一台服务器时，复用单个 TCP 连接。

use crate::backend::ssh_session::{BaseSshConnection, SshChannel};
use crate::session::AuthMethod;
use crate::app::ConnectErrorKind;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 连接标识符：用于识别可复用的 SSH 连接。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionKey {
    pub host: String,
    pub port: u16,
    pub user: String,
    /// 认证方式也作为键的一部分（不同认证方式不能复用）
    pub auth: AuthMethod,
}

impl ConnectionKey {
    pub fn new(host: String, port: u16, user: String, auth: AuthMethod) -> Self {
        Self { host, port, user, auth }
    }
}

/// 活跃的共享连接
struct ActiveConnection {
    /// 底层的 BaseSshConnection
    conn: Arc<BaseSshConnection>,
}

/// 连接管理器：按 ConnectionKey 聚合活跃连接，支持 channel 多路复用。
pub struct SharedSessionManager {
    /// 活跃连接映射: ConnectionKey -> ActiveConnection
    connections: RwLock<HashMap<ConnectionKey, ActiveConnection>>,
    /// 最大连接数限制
    max_connections: usize,
}

impl SharedSessionManager {
    pub fn new(max_connections: usize) -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            max_connections,
        }
    }

    /// 获取或创建连接，并在该连接上打开一个新的 PTY channel。
    ///
    /// 如果已存在相同 ConnectionKey 的连接，复用它并打开新 channel；
    /// 否则调用 `connect_fn` 创建新连接。
    ///
    /// 注意：channel 的生命周期由 BaseSshConnection 管理。
    /// 当所有 channel 关闭后，BaseSshConnection 不会自动关闭（需要外部调用 close）
    /// —— 但这通常不是问题，因为用户通常不会在短时间内重复连接。
    pub async fn get_or_open_channel(
        &self,
        key: ConnectionKey,
        cols: u16,
        rows: u16,
        connect_fn: impl FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Arc<BaseSshConnection>, ConnectErrorKind>> + Send>>,
    ) -> Result<Arc<SshChannel>, SharedSessionError> {
        // 检查是否已有连接
        {
            let connections = self.connections.read().await;
            if connections.contains_key(&key) {
                // 获取连接并在锁外打开 channel
                let conn = connections.get(&key).unwrap().conn.clone();
                drop(connections);
                log::info!(
                    "[SharedSession] Reusing connection for {}@{}:{} (channels: {})",
                    key.user, key.host, key.port,
                    conn.channel_count()
                );
                let channel = conn.open_channel(cols, rows).await
                    .map_err(SharedSessionError::ChannelOpenFailed)?;
                return Ok(Arc::new(channel));
            }
        }

        // 检查连接数限制
        {
            let connections = self.connections.read().await;
            if connections.len() >= self.max_connections {
                return Err(SharedSessionError::MaxConnectionsReached(self.max_connections));
            }
        }

        // 创建新连接
        log::info!(
            "[SharedSession] Creating new shared connection for {}@{}:{}",
            key.user, key.host, key.port
        );

        let conn = connect_fn().await.map_err(SharedSessionError::ConnectFailed)?;

        {
            let mut connections = self.connections.write().await;
            connections.insert(key, ActiveConnection { conn: conn.clone() });
        }

        let channel = conn.open_channel(cols, rows).await
            .map_err(SharedSessionError::ChannelOpenFailed)?;

        Ok(Arc::new(channel))
    }

    /// 关闭一个连接（会关闭所有 channel）
    pub async fn close(&self, key: &ConnectionKey) {
        let mut connections = self.connections.write().await;
        if let Some(active) = connections.remove(key) {
            log::info!(
                "[SharedSession] Closed connection for {}@{}:{} (channels: {})",
                key.user, key.host, key.port,
                active.conn.channel_count()
            );
            // 异步关闭所有 channel
            let conn = active.conn;
            tokio::spawn(async move {
                conn.close().await;
            });
        }
    }

    /// 关闭所有连接
    pub async fn close_all(&self) {
        let mut connections = self.connections.write().await;
        let count = connections.len();
        for (key, active) in connections.drain() {
            log::info!(
                "[SharedSession] Closing connection for {}@{}:{}",
                key.user, key.host, key.port
            );
            let conn = active.conn;
            tokio::spawn(async move {
                conn.close().await;
            });
        }
        log::info!("[SharedSession] Closed all {} connections", count);
    }

    /// 获取当前连接数
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// 获取统计信息
    pub async fn stats(&self) -> SharedSessionStats {
        let connections = self.connections.read().await;
        let mut total_channels = 0;
        let mut details = Vec::new();

        for (key, active) in connections.iter() {
            let ch_count = active.conn.channel_count();
            total_channels += ch_count;
            details.push(SharedConnectionInfo {
                key: key.clone(),
                channel_count: ch_count,
            });
        }

        SharedSessionStats {
            connection_count: connections.len(),
            total_channels,
            max_connections: self.max_connections,
            connections: details,
        }
    }

    /// 检查连接是否存在
    pub async fn has_connection(&self, key: &ConnectionKey) -> bool {
        let connections = self.connections.read().await;
        connections.contains_key(key)
    }
}

/// 共享会话错误
#[derive(Debug, thiserror::Error)]
pub enum SharedSessionError {
    #[error("连接失败: {0}")]
    ConnectFailed(ConnectErrorKind),
    #[error("SSH 通道打开失败")]
    ChannelOpenFailed(anyhow::Error),
    #[error("已达到最大连接数限制 ({0})")]
    MaxConnectionsReached(usize),
}

/// 共享会话统计
#[derive(Debug, Clone)]
pub struct SharedSessionStats {
    pub connection_count: usize,
    pub total_channels: usize,
    pub max_connections: usize,
    pub connections: Vec<SharedConnectionInfo>,
}

/// 单个连接的信息
#[derive(Debug, Clone)]
pub struct SharedConnectionInfo {
    pub key: ConnectionKey,
    pub channel_count: usize,
}
