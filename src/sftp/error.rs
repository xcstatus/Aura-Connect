//! SFTP 错误类型

/// SFTP 操作相关的错误类型
#[derive(Debug, thiserror::Error)]
pub enum SftpError {
    /// 连接失败
    #[error("SFTP 连接失败: {0}")]
    ConnectionFailed(String),

    /// 连接断开
    #[error("SFTP 连接已断开")]
    Disconnected,

    /// 会话未初始化
    #[error("SFTP 会话未初始化")]
    SessionNotInitialized,

    /// 路径不存在
    #[error("路径不存在: {0}")]
    PathNotFound(String),

    /// 权限不足
    #[error("权限不足: {0}")]
    PermissionDenied(String),

    /// 文件已存在
    #[error("文件已存在: {0}")]
    FileExists(String),

    /// 非空目录（无法删除）
    #[error("目录非空，无法删除: {0}")]
    DirectoryNotEmpty(String),

    /// 传输失败
    #[error("传输失败: {0}")]
    TransferFailed(String),

    /// 传输被取消
    #[error("传输已取消")]
    TransferCancelled,

    /// 传输会话无效
    #[error("传输会话无效")]
    InvalidTransferSession,

    /// 协议错误
    #[error("SFTP 协议错误: {0}")]
    ProtocolError(String),

    /// 服务器不支持 SFTP
    #[error("服务器不支持 SFTP 子系统")]
    SubsystemNotAvailable,

    /// 超时
    #[error("操作超时")]
    Timeout,

    /// 路径格式错误
    #[error("无效的路径格式: {0}")]
    InvalidPath(String),

    /// 认证失败
    #[error("SFTP 认证失败: {0}")]
    AuthenticationFailed(String),

    /// 其他错误
    #[error("SFTP 错误: {0}")]
    Other(String),
}

impl From<std::io::Error> for SftpError {
    fn from(e: std::io::Error) -> Self {
        use std::io::ErrorKind;
        match e.kind() {
            ErrorKind::NotFound => SftpError::PathNotFound(e.to_string()),
            ErrorKind::PermissionDenied => SftpError::PermissionDenied(e.to_string()),
            ErrorKind::AlreadyExists => SftpError::FileExists(e.to_string()),
            ErrorKind::TimedOut => SftpError::Timeout,
            ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted
            | ErrorKind::BrokenPipe => SftpError::Disconnected,
            _ => SftpError::Other(e.to_string()),
        }
    }
}

impl From<anyhow::Error> for SftpError {
    fn from(e: anyhow::Error) -> Self {
        SftpError::Other(e.to_string())
    }
}
