//! SFTP 文件条目数据结构

use std::path::PathBuf;

/// 远程文件系统条目
#[derive(Debug, Clone)]
pub struct RemoteFileEntry {
    /// 文件名
    pub name: String,
    /// 完整路径
    pub path: PathBuf,
    /// 是否为目录
    pub is_dir: bool,
    /// 文件大小（字节）
    pub size: u64,
    /// 最后修改时间（Unix 时间戳，秒）
    pub modified: i64,
    /// 权限字符串（如 "drwxr-xr-x"）
    pub permissions: String,
    /// 文件所有者用户名
    pub owner: Option<String>,
    /// 文件所属组
    pub group: Option<String>,
    /// 是否为符号链接
    pub is_symlink: bool,
    /// 链接目标（如果是符号链接）
    pub symlink_target: Option<PathBuf>,
}

impl RemoteFileEntry {
    /// 文件大小的人类可读格式
    #[must_use]
    pub fn size_human(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        const TB: u64 = GB * 1024;

        if self.size >= TB {
            format!("{:.1}T", self.size as f64 / TB as f64)
        } else if self.size >= GB {
            format!("{:.1}G", self.size as f64 / GB as f64)
        } else if self.size >= MB {
            format!("{:.1}M", self.size as f64 / MB as f64)
        } else if self.size >= KB {
            format!("{:.1}K", self.size as f64 / KB as f64)
        } else {
            format!("{}B", self.size)
        }
    }

    /// 修改时间的人类可读格式
    #[must_use]
    pub fn modified_human(&self) -> String {
        use chrono::{Local, TimeZone};
        if self.modified > 0 {
            let datetime = Local.timestamp_opt(self.modified, 0);
            if let Some(dt) = datetime.single() {
                return dt.format("%Y-%m-%d %H:%M").to_string();
            }
        }
        String::from("-")
    }

    /// 获取父目录路径
    #[must_use]
    pub fn parent_path(&self) -> Option<PathBuf> {
        self.path.parent().map(|p| p.to_path_buf())
    }

    /// 检查文件名是否为隐藏文件（以 . 开头）
    #[must_use]
    pub fn is_hidden(&self) -> bool {
        self.name.starts_with('.')
    }

    /// 获取文件扩展名
    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        self.name.rsplit('.').next().filter(|e| e.len() < self.name.len())
    }
}

/// 排序字段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SftpSortBy {
    /// 按名称排序
    #[default]
    Name,
    /// 按大小排序
    Size,
    /// 按修改时间排序
    Modified,
}

/// 排序方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    /// 升序
    #[default]
    Ascending,
    /// 降序
    Descending,
}

impl SortDirection {
    /// 判断是否降序
    #[must_use]
    pub fn is_descending(&self) -> bool {
        matches!(self, SortDirection::Descending)
    }

    /// 切换排序方向
    #[must_use]
    pub fn toggle(self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }
}
