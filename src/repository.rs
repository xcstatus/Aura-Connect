use async_trait::async_trait;
use crate::session::SessionProfile;
use std::path::PathBuf;
use anyhow::Result;

/// 领域驱动设计下的存储隔离边界
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// 获取所有会话列表
    async fn fetch_all(&self) -> Result<Vec<SessionProfile>>;
    /// 保存单个会话
    async fn save(&self, session: SessionProfile) -> Result<()>;
    /// 删除指定 ID 的会话
    async fn delete(&self, id: &str) -> Result<()>;
}

/// 基于本地 JSON 文件的 SessionStore 实现
pub struct JsonSessionStore {
    path: PathBuf,
}

impl JsonSessionStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[async_trait]
impl SessionStore for JsonSessionStore {
    async fn fetch_all(&self) -> Result<Vec<SessionProfile>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let content = tokio::fs::read_to_string(&self.path).await?;
        let library: crate::session::SessionLibrary = serde_json::from_str(&content)?;
        Ok(library.sessions)
    }

    async fn save(&self, session: SessionProfile) -> Result<()> {
        let mut sessions = self.fetch_all().await?;
        if let Some(pos) = sessions.iter().position(|s| s.id == session.id) {
            sessions[pos] = session;
        } else {
            sessions.push(session);
        }
        
        let library = crate::session::SessionLibrary {
            sessions,
            version: "1.1.0".to_string(),
        };
        
        let content = serde_json::to_string_pretty(&library)?;
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&self.path, content).await?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut sessions = self.fetch_all().await?;
        sessions.retain(|s| s.id != id);
        
        let library = crate::session::SessionLibrary {
            sessions,
            version: "1.1.0".to_string(),
        };
        
        let content = serde_json::to_string_pretty(&library)?;
        tokio::fs::write(&self.path, content).await?;
        Ok(())
    }
}
