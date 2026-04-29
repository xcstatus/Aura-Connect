//! Core SSH session traits and command types.

use anyhow::Result;

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
