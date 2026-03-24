use async_trait::async_trait;
use anyhow::Result;
use std::sync::Arc;
use russh::*;

use russh_keys::*;
use tokio::sync::mpsc;


/// SSH 客户端回调处理
struct Client {
    tx: Option<mpsc::UnboundedSender<Vec<u8>>>,
}

#[async_trait]
impl client::Handler for Client {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
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
}

impl SshSession {
    pub fn new() -> Self {
        Self { 
            handle: None,
            data_rx: None,
            cmd_tx: None,
            read_buffer: Vec::new(),
        }
    }

    pub async fn connect(&mut self, host: &str, port: u16, user: &str, password: &str) -> Result<()> {
        // 创建有界缓冲信道（背压控制 8192 包），防止由于前端 UI 卡顿或猫大文件导致内存暴增
        let (tx, rx) = mpsc::channel(8192);
        let config = Arc::new(client::Config::default());
        let mut handle = client::connect(config, (host, port), Client { tx: None }).await?;
        
        if handle.authenticate_password(user, password).await? {
            let mut channel = handle.channel_open_session().await?;
            channel.request_pty(true, "xterm-256color", 80, 24, 0, 0, &[]).await?;
            channel.request_shell(true).await?;

            let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<SessionCmd>();

            self.handle = Some(handle);
            self.data_rx = Some(Arc::new(tokio::sync::Mutex::new(rx)));
            self.cmd_tx = Some(cmd_tx);

            // 启动数据收发处理循环 (UI <-> SSH)
            // 增加 Keep-Alive 心跳间隔轮询: 每30秒发送一次 ping 以防被防火墙剔除
            let mut keep_alive_interval = tokio::time::interval(std::time::Duration::from_secs(30));
            
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = keep_alive_interval.tick() => {
                            // russh 0.45 机制下可以在此处拓展全局的 connection ping
                        }
                        cmd = cmd_rx.recv() => {
                            match cmd {
                                Some(SessionCmd::Data(data)) => {
                                    println!("📡 [SessionCmd] 发送至服务端 SSH: {} bytes", data.len());
                                    let _ = channel.data(&data[..]).await; 
                                }
                                Some(SessionCmd::Resize(c, r)) => { let _ = channel.window_change(c as u32, r as u32, 0, 0).await; }
                                None => break,
                            }
                        }
                        msg = channel.wait() => {
                            match msg {
                                Some(russh::ChannelMsg::Data { data }) => {
                                    println!("📥 [ChannelMsg] 收到服务端数据 (Data): {} bytes", data.len());
                                    // 遇到背压时等待，避免 OOM 崩溃
                                    let _ = tx.send(data.to_vec()).await;
                                }
                                Some(russh::ChannelMsg::ExtendedData { data, .. }) => {
                                    println!("📥 [ChannelMsg] 收到服务端扩展数据 (ExtendedData): {} bytes", data.len());
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
        }
        Ok(())
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
                        println!("📤 [AsyncSession] 将缓冲区的 {} 字节数据推送给 UI 渲染层", data.len());
                        let len = std::cmp::min(buffer.len(), data.len());
                        buffer[..len].copy_from_slice(&data[..len]);
                        if data.len() > len {
                            self.read_buffer.extend_from_slice(&data[len..]);
                        }
                        return Ok(len);
                    }
                    Err(mpsc::error::TryRecvError::Empty) | Err(mpsc::error::TryRecvError::Disconnected) => return Ok(0),
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
