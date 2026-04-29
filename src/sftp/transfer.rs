//! SFTP 文件传输任务管理

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use uuid::Uuid;

/// 文件传输任务
#[derive(Debug, Clone)]
pub struct SftpTransfer {
    /// 传输任务唯一标识
    pub id: Uuid,
    /// 传输类型
    pub direction: TransferDirection,
    /// 源路径（远程或本地）
    pub source: String,
    /// 目标路径（远程或本地）
    pub dest: String,
    /// 文件总大小（字节）
    pub total_size: u64,
    /// 已传输字节数
    pub transferred: Arc<AtomicU64>,
    /// 传输状态
    pub status: TransferStatus,
    /// 错误信息（若有）
    pub error: Option<String>,
    /// 开始时间
    pub start_time: Option<std::time::Instant>,
    /// 完成时间
    pub end_time: Option<std::time::Instant>,
}

impl SftpTransfer {
    /// 创建新的传输任务
    pub fn new(
        direction: TransferDirection,
        source: String,
        dest: String,
        total_size: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            direction,
            source,
            dest,
            total_size,
            transferred: Arc::new(AtomicU64::new(0)),
            status: TransferStatus::Pending,
            error: None,
            start_time: None,
            end_time: None,
        }
    }

    /// 获取当前传输进度（0.0 ~ 1.0）
    pub fn progress(&self) -> f64 {
        if self.total_size == 0 {
            return 1.0;
        }
        let transferred = self.transferred.load(Ordering::Relaxed) as f64;
        let total = self.total_size as f64;
        (transferred / total).min(1.0)
    }

    /// 获取已传输字节数
    pub fn transferred_bytes(&self) -> u64 {
        self.transferred.load(Ordering::Relaxed)
    }

    /// 更新已传输字节数
    pub fn update_transferred(&self, bytes: u64) {
        self.transferred.store(bytes, Ordering::Relaxed);
    }

    /// 启动传输
    pub fn start(&mut self) {
        self.status = TransferStatus::Running;
        self.start_time = Some(std::time::Instant::now());
    }

    /// 完成传输
    pub fn complete(&mut self) {
        self.status = TransferStatus::Completed;
        self.end_time = Some(std::time::Instant::now());
        self.transferred.store(self.total_size, Ordering::Relaxed);
    }

    /// 标记传输失败
    pub fn fail(&mut self, error: String) {
        self.status = TransferStatus::Failed;
        self.error = Some(error);
        self.end_time = Some(std::time::Instant::now());
    }

    /// 取消传输
    pub fn cancel(&mut self) {
        self.status = TransferStatus::Cancelled;
        self.end_time = Some(std::time::Instant::now());
    }

    /// 获取传输速度（字节/秒）
    pub fn speed(&self) -> Option<f64> {
        let start = self.start_time?;
        let transferred = self.transferred_bytes() as f64;
        let elapsed = start.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            Some(transferred / elapsed)
        } else {
            None
        }
    }

    /// 获取传输速度的人类可读格式
    pub fn speed_human(&self) -> String {
        match self.speed() {
            Some(speed) => {
                const KB: f64 = 1024.0;
                const MB: f64 = KB * 1024.0;
                const GB: f64 = MB * 1024.0;

                if speed >= GB {
                    format!("{:.1}G/s", speed / GB)
                } else if speed >= MB {
                    format!("{:.1}M/s", speed / MB)
                } else if speed >= KB {
                    format!("{:.1}K/s", speed / KB)
                } else {
                    format!("{:.0}B/s", speed)
                }
            }
            None => String::from("-"),
        }
    }

    /// 获取剩余时间（秒）
    pub fn remaining_time(&self) -> Option<f64> {
        let speed = self.speed()?;
        let remaining = self.total_size.saturating_sub(self.transferred_bytes()) as f64;
        if speed > 0.0 {
            Some(remaining / speed)
        } else {
            None
        }
    }

    /// 获取剩余时间的人类可读格式
    pub fn remaining_time_human(&self) -> String {
        match self.remaining_time() {
            Some(secs) => {
                if secs < 1.0 {
                    return String::from("< 1s");
                }
                let secs = secs as u64;
                if secs < 60 {
                    format!("{secs}s")
                } else if secs < 3600 {
                    format!("{}m {}s", secs / 60, secs % 60)
                } else {
                    format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
                }
            }
            None => String::from("-"),
        }
    }
}

/// 传输方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    /// 下载（远程 -> 本地）
    Download,
    /// 上传（本地 -> 远程）
    Upload,
}

impl TransferDirection {
    /// 获取传输方向的显示名称
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            TransferDirection::Download => "下载",
            TransferDirection::Upload => "上传",
        }
    }
}

/// 传输状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStatus {
    /// 等待中
    Pending,
    /// 传输中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

impl TransferStatus {
    /// 判断是否为最终状态
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TransferStatus::Completed | TransferStatus::Failed | TransferStatus::Cancelled
        )
    }
}
