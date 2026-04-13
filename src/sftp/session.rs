//! SFTP 会话管理模块
//!
//! 封装与 SSH 连接关联的 SFTP subsystem 通信，基于 russh-sftp 实现。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;

use crate::backend::ssh_session::BaseSshConnection;

use super::error::SftpError;
use super::file_entry::RemoteFileEntry;
use russh_sftp::client::fs::Metadata;
use russh_sftp::client::SftpSession as RusshSftpSession;

/// SFTP 会话，绑定到一个已建立的 SSH 连接
#[derive(Clone)]
pub struct SftpSession {
    /// russh-sftp 客户端会话
    sftp: Arc<RusshSftpSession>,
    /// 当前工作目录（远程路径）
    cwd: Arc<std::sync::Mutex<String>>,
}

impl std::fmt::Debug for SftpSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SftpSession")
            .field("cwd", &*self.cwd.lock().unwrap())
            .finish()
    }
}

impl SftpSession {
    /// 从现有 SSH 连接创建 SFTP 会话
    pub async fn new(conn: Arc<BaseSshConnection>) -> Result<Self, SftpError> {
        // 打开 SFTP subsystem channel
        let channel = conn
            .open_sftp_channel()
            .await
            .map_err(|e| SftpError::ConnectionFailed(e.to_string()))?;

        // 将 channel 转换为 stream 并初始化 russh-sftp
        let stream = channel.into_stream();
        let sftp = RusshSftpSession::new(stream)
            .await
            .map_err(|e| SftpError::ProtocolError(e.to_string()))?;

        Ok(Self {
            sftp: Arc::new(sftp),
            cwd: Arc::new(std::sync::Mutex::new("/".to_string())),
        })
    }

    /// 获取当前工作目录
    pub fn cwd(&self) -> String {
        self.cwd.lock().unwrap().clone()
    }

    /// 设置当前工作目录
    pub fn set_cwd(&self, path: &str) {
        let mut cwd = self.cwd.lock().unwrap();
        *cwd = path.to_string();
    }

    /// 标准化路径
    fn normalize_path(&self, path: &str) -> String {
        if path.starts_with('/') {
            return Self::canonicalize_path(path);
        }

        let cwd = self.cwd();
        let base = if cwd == "/" {
            String::new()
        } else {
            cwd.clone()
        };

        Self::canonicalize_path(&format!("{}/{}", base, path))
    }

    /// 规范化路径（解析 .，拒绝 .. 防止路径遍历）
    fn canonicalize_path(path: &str) -> String {
        // 拒绝包含 .. 的路径，防止路径遍历攻击
        if path.contains("..") {
            return "/".to_string();
        }

        let mut components = Vec::new();

        for component in path.split('/') {
            match component {
                "" | "." => continue,
                _ => components.push(component),
            }
        }

        let result = components.join("/");
        if result.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", result)
        }
    }

    /// 拼接路径
    fn join_path(base: &str, name: &str) -> String {
        if base == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", base, name)
        }
    }

    /// 格式化权限字符串
    fn format_permissions(metadata: &Metadata) -> String {
        let file_type = if metadata.is_dir() { 'd' } else { '-' };

        // 从 FilePermissions 获取权限位
        let perms = metadata.permissions();

        let r = if perms.owner_read { 'r' } else { '-' };
        let w = if perms.owner_write { 'w' } else { '-' };
        let x = if perms.owner_exec { 'x' } else { '-' };

        let r2 = if perms.group_read { 'r' } else { '-' };
        let w2 = if perms.group_write { 'w' } else { '-' };
        let x2 = if perms.group_exec { 'x' } else { '-' };

        let r3 = if perms.other_read { 'r' } else { '-' };
        let w3 = if perms.other_write { 'w' } else { '-' };
        let x3 = if perms.other_exec { 'x' } else { '-' };

        format!("{}{}{}{}{}{}{}{}{}{}", file_type, r, w, x, r2, w2, x2, r3, w3, x3)
    }

    /// 列出目录内容（不含 "." 和 ".."）
    pub async fn read_dir(&self, path: &str) -> Result<Vec<RemoteFileEntry>, SftpError> {
        let normalized = self.normalize_path(path);
        let mut entries = Vec::new();

        let dir = self
            .sftp
            .read_dir(&normalized)
            .await
            .map_err(|e| SftpError::ProtocolError(e.to_string()))?;

        // DirEntry iterator is synchronous and already filters "." and ".."
        for entry in dir {
            let full_path = Self::join_path(&normalized, &entry.file_name());

            entries.push(RemoteFileEntry {
                name: entry.file_name(),
                path: PathBuf::from(&full_path),
                is_dir: entry.metadata().is_dir(),
                size: entry.metadata().len(),
                modified: entry.metadata().mtime.map(|t| t as i64).unwrap_or(0),
                permissions: Self::format_permissions(&entry.metadata()),
                owner: entry.metadata().user.clone(),
                group: entry.metadata().group.clone(),
                is_symlink: entry.metadata().is_symlink(),
                symlink_target: None,
            });
        }

        Ok(entries)
    }

    /// 切换到指定目录
    pub async fn change_dir(&mut self, path: &str) -> Result<(), SftpError> {
        let normalized = self.normalize_path(path);

        // 验证目录是否存在且可访问
        let exists = self
            .sftp
            .try_exists(&normalized)
            .await
            .map_err(|e| SftpError::ProtocolError(e.to_string()))?;

        if !exists {
            return Err(SftpError::PathNotFound(normalized));
        }

        // 更新 cwd
        self.set_cwd(&normalized);
        Ok(())
    }

    /// 返回上级目录
    pub fn parent_dir(&self) -> Option<String> {
        let cwd = self.cwd();
        if cwd == "/" {
            return None;
        }

        let parent = PathBuf::from(&cwd)
            .parent()
            .map(|p| {
                let s = p.to_string_lossy().to_string();
                if s.is_empty() {
                    "/".to_string()
                } else {
                    s
                }
            })
            .unwrap_or_else(|| "/".to_string());

        Some(parent)
    }

    /// 创建目录
    pub async fn make_dir(&mut self, path: &str) -> Result<(), SftpError> {
        let normalized = self.normalize_path(path);

        self.sftp
            .create_dir(&normalized)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("Permission") || msg.contains("denied") {
                    SftpError::PermissionDenied(normalized)
                } else if msg.contains("exist") {
                    SftpError::FileExists(normalized)
                } else {
                    SftpError::ProtocolError(msg)
                }
            })?;

        Ok(())
    }

    /// 删除文件
    pub async fn remove_file(&self, path: &str) -> Result<(), SftpError> {
        let normalized = self.normalize_path(path);

        self.sftp
            .remove_file(&normalized)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("Permission") || msg.contains("denied") {
                    SftpError::PermissionDenied(normalized)
                } else {
                    SftpError::ProtocolError(msg)
                }
            })?;

        Ok(())
    }

    /// 删除目录（必须是空目录）
    pub async fn remove_dir(&self, path: &str) -> Result<(), SftpError> {
        let normalized = self.normalize_path(path);

        self.sftp
            .remove_dir(&normalized)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("Permission") || msg.contains("denied") {
                    SftpError::PermissionDenied(normalized)
                } else if msg.contains("not empty") || msg.contains("Directory not empty") {
                    SftpError::DirectoryNotEmpty(normalized)
                } else {
                    SftpError::ProtocolError(msg)
                }
            })?;

        Ok(())
    }

    /// 重命名文件
    pub async fn rename(&self, old_path: &str, new_path: &str) -> Result<(), SftpError> {
        let old = self.normalize_path(old_path);
        let new = self.normalize_path(new_path);

        self.sftp
            .rename(&old, &new)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("Permission") || msg.contains("denied") {
                    SftpError::PermissionDenied(old)
                } else {
                    SftpError::ProtocolError(msg)
                }
            })?;

        Ok(())
    }

    /// 获取文件元数据
    pub async fn stat(&self, path: &str) -> Result<RemoteFileEntry, SftpError> {
        let normalized = self.normalize_path(path);
        let normalized_for_error = normalized.clone();

        let metadata = self
            .sftp
            .metadata(&normalized)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("Not found") || msg.contains("No such") {
                    SftpError::PathNotFound(normalized_for_error.clone())
                } else if msg.contains("Permission") || msg.contains("denied") {
                    SftpError::PermissionDenied(normalized_for_error)
                } else {
                    SftpError::ProtocolError(msg)
                }
            })?;

        let name = Path::new(&normalized)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| normalized.clone());

        Ok(RemoteFileEntry {
            name,
            path: PathBuf::from(&normalized),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            modified: metadata.mtime.map(|t| t as i64).unwrap_or(0),
            permissions: Self::format_permissions(&metadata),
            owner: metadata.user.clone(),
            group: metadata.group.clone(),
            is_symlink: metadata.is_symlink(),
            symlink_target: None,
        })
    }

    /// 下载文件到本地
    pub async fn download_file(
        &self,
        remote_path: &str,
        local_path: &Path,
    ) -> Result<(), SftpError> {
        let remote = self.normalize_path(remote_path);

        // 使用 high-level API 直接读取整个文件
        let data = self
            .sftp
            .read(&remote)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("Not found") || msg.contains("No such") {
                    SftpError::PathNotFound(remote.clone())
                } else if msg.contains("Permission") || msg.contains("denied") {
                    SftpError::PermissionDenied(remote)
                } else {
                    SftpError::TransferFailed(msg)
                }
            })?;

        // 写入本地文件
        std::fs::write(local_path, &data)
            .map_err(|e| SftpError::TransferFailed(e.to_string()))?;

        Ok(())
    }

    /// 上传文件到远程
    pub async fn upload_file(
        &self,
        local_path: &Path,
        remote_path: &str,
    ) -> Result<(), SftpError> {
        let remote = self.normalize_path(remote_path);

        // 读取本地文件
        let data = std::fs::read(local_path)
            .map_err(|e| SftpError::TransferFailed(e.to_string()))?;

        // 使用 high-level API 直接写入整个文件
        self.sftp
            .write(&remote, &data)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("Permission") || msg.contains("denied") {
                    SftpError::PermissionDenied(remote)
                } else {
                    SftpError::TransferFailed(msg)
                }
            })?;

        Ok(())
    }

    /// 检查路径是否存在
    pub async fn exists(&self, path: &str) -> Result<bool, SftpError> {
        let normalized = self.normalize_path(path);

        self.sftp
            .try_exists(&normalized)
            .await
            .map_err(|e| SftpError::ProtocolError(e.to_string()))
    }

    /// 获取文件大小
    pub async fn file_size(&self, path: &str) -> Result<u64, SftpError> {
        let entry = self.stat(path).await?;
        Ok(entry.size)
    }

    /// 检查是否为目录
    pub async fn is_dir(&self, path: &str) -> Result<bool, SftpError> {
        let entry = self.stat(path).await?;
        Ok(entry.is_dir)
    }

    /// 关闭 SFTP 会话
    pub async fn close(self) -> Result<(), SftpError> {
        self.sftp
            .close()
            .await
            .map_err(|e| SftpError::ProtocolError(e.to_string()))?;
        Ok(())
    }
}
