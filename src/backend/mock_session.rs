use crate::backend::ssh_session::AsyncSession;
use anyhow::Result;
use std::collections::VecDeque;

pub struct MockSession {
    pub incoming_data: VecDeque<Vec<u8>>,
    pub outgoing_commands: Vec<Vec<u8>>,
    pub is_connected: bool,
}

impl MockSession {
    pub fn new() -> Self {
        Self {
            incoming_data: VecDeque::new(),
            outgoing_commands: Vec::new(),
            is_connected: true,
        }
    }

    pub fn push_data(&mut self, data: &[u8]) {
        self.incoming_data.push_back(data.to_vec());
    }
}

impl AsyncSession for MockSession {
    fn read_stream(&mut self, buffer: &mut [u8]) -> Result<usize> {
        if let Some(data) = self.incoming_data.pop_front() {
            let len = std::cmp::min(buffer.len(), data.len());
            buffer[..len].copy_from_slice(&data[..len]);
            // 如果 buffer 不够大，简单的 Mock 暂时不处理剩余部分，或在实际测试中保证 buffer 足够
            return Ok(len);
        }
        Ok(0)
    }

    fn write_stream(&mut self, data: &[u8]) -> Result<()> {
        self.outgoing_commands.push(data.to_vec());
        Ok(())
    }

    fn resize_pty(&mut self, _cols: u16, _rows: u16) -> Result<()> {
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.is_connected
    }

    fn exit_status(&self) -> Option<u32> {
        None
    }
}
