use crate::repository::SessionStore;
use crate::session::SessionLibrary;
use anyhow::Result;
use std::sync::Arc;

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
        // 当前 JSON 实现是全量覆盖，未来 SQLite 可实现增量
        for session in &self.library.sessions {
            self.store.save(session.clone()).await?;
        }
        Ok(())
    }

    /// 异步添加会话
    pub async fn add_session(&mut self, config: crate::session::SessionConfig) -> Result<()> {
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
    pub async fn upsert_session(&mut self, config: crate::session::SessionConfig) -> Result<()> {
        self.store.save(config.clone()).await?;
        if let Some(existing) = self.library.sessions.iter_mut().find(|s| s.id == config.id) {
            *existing = config;
        } else {
            self.library.sessions.push(config);
        }
        Ok(())
    }
}
