//! 端口转发管理器
//!
//! 支持本地端口转发 (L)
//!
//! 注意：这是一个基础实现，完整的端口转发功能需要与 BaseSshConnection 集成

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use crate::settings::PortForwardConfig;

/// 端口转发管理器
///
/// 管理端口转发配置和活跃的转发连接
pub struct PortForwardManager {
    /// 配置列表
    configs: RwLock<Vec<PortForwardConfig>>,
    /// 活跃的端口转发映射: port -> BindInfo
    active_forwards: RwLock<HashMap<u16, BindInfo>>,
}

/// 绑定的端口信息
#[allow(dead_code)]
struct BindInfo {
    /// 监听器
    listener: TcpListener,
    /// 预留：目标地址
    target_addr: String,
    /// 预留：目标端口
    target_port: u16,
}

impl PortForwardManager {
    /// 创建端口转发管理器
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            configs: RwLock::new(Vec::new()),
            active_forwards: RwLock::new(HashMap::new()),
        })
    }

    /// 添加端口转发配置
    pub async fn add_config(&self, config: PortForwardConfig) {
        self.configs.write().await.push(config);
    }

    /// 移除端口转发配置
    pub async fn remove_config(&self, index: usize) {
        self.configs.write().await.remove(index);
    }

    /// 获取所有配置
    pub async fn get_configs(&self) -> Vec<PortForwardConfig> {
        self.configs.read().await.clone()
    }

    /// 启动本地端口转发
    ///
    /// 返回实际绑定的端口号
    pub async fn start_local_forward(&self, config: &PortForwardConfig) -> Result<u16> {
        let bind_addr = format!("{}:{}", config.bind_addr, config.bind_port);
        let listener = TcpListener::bind(&bind_addr).await?;
        let actual_port = listener.local_addr()?.port();

        log::info!(
            "[PortForward] Local forward started: {}:{} -> {}:{}",
            config.bind_addr,
            actual_port,
            config.target_addr,
            config.target_port
        );

        self.active_forwards.write().await.insert(
            actual_port,
            BindInfo {
                listener,
                target_addr: config.target_addr.clone(),
                target_port: config.target_port,
            },
        );

        Ok(actual_port)
    }

    /// 停止端口转发
    pub async fn stop_forward(&self, port: u16) -> bool {
        if let Some(_info) = self.active_forwards.write().await.remove(&port) {
            log::info!("[PortForward] Stopped forward on port {}", port);
            // _info 在超出作用域时会自动关闭监听器
            true
        } else {
            false
        }
    }

    /// 获取活跃端口转发列表
    pub async fn list_active_ports(&self) -> Vec<u16> {
        self.active_forwards.read().await.keys().copied().collect()
    }

    /// 获取活跃端口转发数量
    pub async fn count(&self) -> usize {
        self.active_forwards.read().await.len()
    }

    /// 接受新的连接
    ///
    /// 返回 (TcpStream, SocketAddr) 用于后续处理
    pub async fn accept(&self, port: u16) -> Option<(tokio::net::TcpStream, std::net::SocketAddr)> {
        let info = self
            .active_forwards
            .read()
            .await
            .get(&port)?
            .listener
            .accept()
            .await
            .ok()?;
        Some(info)
    }
}

impl Default for PortForwardManager {
    fn default() -> Self {
        Self {
            configs: RwLock::new(Vec::new()),
            active_forwards: RwLock::new(HashMap::new()),
        }
    }
}
