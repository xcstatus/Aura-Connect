//! RustSsh - SSH 终端模拟器
//!
//! 这是一个基于 Iced GUI 框架的 SSH 客户端，使用 Rust 实现。

pub mod app;
pub mod backend;
pub mod i18n;
pub mod logging;
pub mod security;
pub mod session;
pub mod settings;
pub mod sftp;
pub mod terminal;
pub mod theme;
pub mod time_utils;
pub mod utils;
pub mod vault;

// Re-exports for convenience
pub use terminal::controller::TerminalController;
pub use terminal::palette::scheme_vt_bytes;
pub use terminal::selection::TerminalSelection;
