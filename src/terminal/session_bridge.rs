use crate::backend::ssh_session::AsyncSession;
use anyhow::Result;

/// UI-thread bridge between an `AsyncSession` and terminal backends.
///
/// Notes:
/// - `AsyncSession::read_stream` in this codebase is non-blocking (try-recv),
///   so we drain it per-frame with a byte budget to avoid UI stalls.
pub struct TerminalSessionBridge {
    buf: [u8; 4096],
    pending: Vec<u8>,
    pending_off: usize,
    max_bytes_per_frame: usize,
    max_reads_per_frame: usize,
}

impl Default for TerminalSessionBridge {
    fn default() -> Self {
        Self {
            buf: [0u8; 4096],
            pending: Vec::new(),
            pending_off: 0,
            max_bytes_per_frame: 32 * 1024,
            max_reads_per_frame: 64,
        }
    }
}

impl TerminalSessionBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_budgets(&mut self, max_bytes_per_frame: usize, max_reads_per_frame: usize) {
        self.max_bytes_per_frame = max_bytes_per_frame.max(1024);
        self.max_reads_per_frame = max_reads_per_frame.max(1);
    }

    /// Drain session output (remote -> UI) with a frame budget.
    /// Returns the total bytes drained.
    pub fn drain_output(
        &mut self,
        session: &mut dyn AsyncSession,
        mut on_bytes: impl FnMut(&[u8]),
    ) -> Result<usize> {
        let mut total = 0usize;

        // 1) Drain pending remainder from last frame first.
        if self.pending_off < self.pending.len() {
            let remaining_budget = self.max_bytes_per_frame.saturating_sub(total);
            if remaining_budget == 0 {
                return Ok(0);
            }
            let rem = &self.pending[self.pending_off..];
            let take = rem.len().min(remaining_budget);
            if take > 0 {
                on_bytes(&rem[..take]);
                total += take;
                self.pending_off += take;
            }
            if self.pending_off >= self.pending.len() {
                self.pending.clear();
                self.pending_off = 0;
            }
        }

        for _ in 0..self.max_reads_per_frame {
            if total >= self.max_bytes_per_frame {
                break;
            }
            let n = session.read_stream(&mut self.buf)?;
            if n == 0 {
                break;
            }
            let remaining_budget = self.max_bytes_per_frame - total;
            if n <= remaining_budget {
                total += n;
                on_bytes(&self.buf[..n]);
            } else {
                // Emit only up to budget, and keep the remainder for next frame.
                on_bytes(&self.buf[..remaining_budget]);
                total += remaining_budget;
                self.pending.clear();
                self.pending
                    .extend_from_slice(&self.buf[remaining_budget..n]);
                self.pending_off = 0;
                break;
            }
        }
        Ok(total)
    }

    /// Best-effort resize propagation to remote PTY.
    ///
    /// **SSOT note:** keep PTY sizing logic centralized in the UI viewport spec (Iced) or
    /// the vt widget layout (egui). Avoid calling `resize_pty` directly elsewhere.
    pub fn resize_pty_if_needed(
        &mut self,
        session: &mut dyn AsyncSession,
        last: &mut (u16, u16),
        cols: u16,
        rows: u16,
    ) -> Result<bool> {
        let cols = cols.max(1);
        let rows = rows.max(1);
        if last.0 == cols && last.1 == rows {
            return Ok(false);
        }
        session.resize_pty(cols, rows)?;
        *last = (cols, rows);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::mock_session::MockSession;

    #[test]
    fn drain_output_respects_byte_budget() {
        let mut s = MockSession::new();
        s.push_data(&vec![b'a'; 1000]);
        s.push_data(&vec![b'b'; 1000]);
        s.push_data(&vec![b'c'; 1000]);

        let mut bridge = TerminalSessionBridge::new();
        bridge.set_budgets(1500, 100);

        let mut seen = Vec::<u8>::new();
        let drained = bridge
            .drain_output(&mut s, |bytes| seen.extend_from_slice(bytes))
            .unwrap();

        assert!(drained <= 1500);
        assert_eq!(seen.len(), drained);
        assert!(drained >= 1000);
    }

    #[test]
    fn drain_output_respects_read_budget() {
        let mut s = MockSession::new();
        s.push_data(b"1");
        s.push_data(b"2");
        s.push_data(b"3");

        let mut bridge = TerminalSessionBridge::new();
        bridge.set_budgets(1024 * 1024, 2);

        let mut chunks = 0usize;
        let drained = bridge.drain_output(&mut s, |_bytes| chunks += 1).unwrap();

        assert_eq!(chunks, 2);
        assert_eq!(drained, 2);
    }

    #[test]
    fn resize_pty_if_needed_debounces() {
        let mut s = MockSession::new();
        let mut bridge = TerminalSessionBridge::new();
        let mut last = (0u16, 0u16);

        let changed = bridge
            .resize_pty_if_needed(&mut s, &mut last, 80, 24)
            .unwrap();
        assert!(changed);
        assert_eq!(last, (80, 24));

        let changed2 = bridge
            .resize_pty_if_needed(&mut s, &mut last, 80, 24)
            .unwrap();
        assert!(!changed2);
    }
}
