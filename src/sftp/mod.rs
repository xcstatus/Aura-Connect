//! SFTP 模块
//!
//! 提供基于 SSH 连接的 SFTP (SSH File Transfer Protocol) 功能，
//! 支持远程文件浏览、下载、文件操作等。

mod error;
mod file_entry;
mod session;
mod transfer;

pub use error::SftpError;
pub use file_entry::{RemoteFileEntry, SftpSortBy, SortDirection};
pub use session::SftpSession;
pub use transfer::{SftpTransfer, TransferDirection, TransferStatus};
