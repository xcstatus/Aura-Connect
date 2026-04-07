//! Session lifecycle management.
//!
//! Manages one active SSH session per tab, independent of tab switching.
//! Supports SSH connection multiplexing via SharedSessionManager.

use crate::backend::ssh_session::AsyncSession;
use std::fmt::Debug;

/// A live SSH session bound to a specific tab.
pub struct TabSession {
    pub session: Box<dyn AsyncSession>,
    pub tab_id: usize,
    /// 连接键：用于多路复用追踪
    pub connection_key: Option<crate::backend::shared_ssh_session::ConnectionKey>,
}
impl Debug for TabSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TabSession")
            .field("tab_id", &self.tab_id)
            .field("connection_key", &self.connection_key)
            .finish()
    }
}

/// Unified session registry: each tab may hold at most one active session.
///
/// Pump loop (`tick.rs`) calls `pump_all` to service all sessions.
/// Individual tabs access their session via `get_session` / `session_mut`.
pub struct SessionManager {
    sessions: Vec<Option<TabSession>>,
    /// 共享会话管理器：支持多路复用
    pub shared_manager: std::sync::Arc<tokio::sync::RwLock<crate::backend::shared_ssh_session::SharedSessionManager>>,
    /// Index of the currently focused tab.
    active_tab: usize,
}

impl SessionManager {
    pub fn new(initial_tabs: usize) -> Self {
        Self {
            sessions: (0..initial_tabs).map(|_| None).collect(),
            shared_manager: std::sync::Arc::new(tokio::sync::RwLock::new(
                crate::backend::shared_ssh_session::SharedSessionManager::new(20), // 最多 20 个活跃连接
            )),
            active_tab: 0,
        }
    }

    /// Set the active tab index (called when user switches tabs).
    pub fn set_active_tab(&mut self, tab_index: usize) {
        self.active_tab = tab_index;
    }

    /// Attach a session to a tab, replacing any existing session on that tab.
    pub fn attach_session(&mut self, tab_index: usize, session: Box<dyn AsyncSession>, connection_key: Option<crate::backend::shared_ssh_session::ConnectionKey>) {
        self.ensure_capacity(tab_index);
        self.sessions[tab_index] = Some(TabSession {
            session,
            tab_id: tab_index,
            connection_key,
        });
    }

    /// Detach (disconnect) the session on a specific tab.
    /// 会自动通知 shared_manager 减少引用计数
    pub fn detach_session(&mut self, tab_index: usize) {
        if tab_index < self.sessions.len() {
            // 获取连接键以便减少引用计数
            let connection_key = self.sessions[tab_index]
                .as_ref()
                .and_then(|ts| ts.connection_key.clone());

            self.sessions[tab_index] = None;

            // 通知 shared_manager 关闭连接（异步）
            if let Some(key) = connection_key {
                let manager = self.shared_manager.clone();
                tokio::spawn(async move {
                    manager.write().await.close(&key).await;
                });
            }
        }
    }

    /// Returns true if a session is attached to the given tab.
    pub fn has_session(&self, tab_index: usize) -> bool {
        self.sessions
            .get(tab_index)
            .is_some_and(|s| s.is_some())
    }

    /// Immutable access to a tab's session.
    pub fn get_session(&self, tab_index: usize) -> Option<&dyn AsyncSession> {
        self.sessions
            .get(tab_index)
            .and_then(|s| s.as_ref())
            .map(|ts| ts.session.as_ref() as _)
    }

    /// Mutable access to a tab's session.
    pub fn session_mut(&mut self, tab_index: usize) -> Option<&mut dyn AsyncSession> {
        self.ensure_capacity(tab_index);
        self.sessions[tab_index]
            .as_mut()
            .map(|ts| ts.session.as_mut() as _)
    }

    /// Mutable access to the active tab's session.
    pub fn active_session_mut(&mut self) -> Option<&mut dyn AsyncSession> {
        self.session_mut(self.active_tab)
    }

    /// Number of tabs currently registered.
    pub fn tab_count(&self) -> usize {
        self.sessions.len()
    }

    /// Add a new tab slot at the end.
    pub fn add_tab(&mut self) {
        self.sessions.push(None);
    }

    /// Remove a tab slot. If the removed tab was after active_tab, active_tab shifts.
    /// Returns the session that was on the removed tab (if any).
    pub fn remove_tab(&mut self, tab_index: usize) -> Option<TabSession> {
        if tab_index >= self.sessions.len() {
            return None;
        }
        let removed = self.sessions.remove(tab_index);
        if self.active_tab > tab_index && self.active_tab > 0 {
            self.active_tab -= 1;
        }
        removed
    }

    fn ensure_capacity(&mut self, tab_index: usize) {
        if tab_index >= self.sessions.len() {
            self.sessions.resize_with(tab_index + 1, || None);
        }
    }
}
