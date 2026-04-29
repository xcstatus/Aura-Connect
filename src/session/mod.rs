//! Session module - 会话模型与仓库
//!
//! 提供会话配置、传输协议、认证方式等数据模型，
//! 以及会话存储抽象和 JSON 实现。

pub mod repository;
pub mod session;

pub use repository::{JsonSessionStore, SessionManager, SessionStore};
pub use session::{
    AuthMethod, ProtocolType, SerialConfig, SessionConfig, SessionLibrary, SessionProfile,
    SshConfig, TelnetConfig, TransportConfig,
};
