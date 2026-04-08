use crate::session::session::{SessionLibrary, SessionProfile};
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

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
        let library: SessionLibrary = serde_json::from_str(&content)?;
        Ok(library.sessions)
    }

    async fn save(&self, session: SessionProfile) -> Result<()> {
        let mut sessions = self.fetch_all().await?;
        if let Some(pos) = sessions.iter().position(|s| s.id == session.id) {
            sessions[pos] = session;
        } else {
            sessions.push(session);
        }

        let library = SessionLibrary {
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

        let library = SessionLibrary {
            sessions,
            version: "1.1.0".to_string(),
        };

        let content = serde_json::to_string_pretty(&library)?;
        tokio::fs::write(&self.path, content).await?;
        Ok(())
    }
}

/// 会话管理器
#[derive(Clone)]
pub struct SessionManager {
    pub library: SessionLibrary,
    pub store: Arc<dyn SessionStore>,
}

impl SessionManager {
    pub fn new(store: Arc<dyn SessionStore>) -> Self {
        Self {
            library: SessionLibrary {
                sessions: Vec::new(),
                version: "1.1.0".to_string(),
            },
            store,
        }
    }

    /// 异步加载所有会话
    pub async fn load(&mut self) -> Result<()> {
        let sessions = self.store.fetch_all().await?;
        self.library.sessions = sessions;
        Ok(())
    }

    /// 异步同步当前库到存储
    pub async fn sync(&self) -> Result<()> {
        for session in &self.library.sessions {
            self.store.save(session.clone()).await?;
        }
        Ok(())
    }

    /// 异步添加会话
    pub async fn add_session(&mut self, config: SessionProfile) -> Result<()> {
        self.store.save(config.clone()).await?;
        self.library.sessions.push(config);
        Ok(())
    }

    /// 异步删除会话
    pub async fn delete_session(&mut self, id: &str) -> Result<()> {
        self.store.delete(id).await?;
        self.library.sessions.retain(|s| s.id != id);
        Ok(())
    }

    /// 异步更新会话（按 id 覆盖）
    pub async fn upsert_session(&mut self, config: SessionProfile) -> Result<()> {
        self.store.save(config.clone()).await?;
        if let Some(existing) = self.library.sessions.iter_mut().find(|s| s.id == config.id) {
            *existing = config;
        } else {
            self.library.sessions.push(config);
        }
        Ok(())
    }
}
