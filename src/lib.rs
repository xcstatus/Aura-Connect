pub mod app_state;
pub mod backend;
pub mod connection_input;
pub mod i18n;
pub mod iced_app;
pub mod logging;
pub mod repository;
pub mod security;
pub mod session;
pub mod settings;
pub mod storage;
pub mod terminal;
pub mod session_repository;
pub mod theme;
pub mod vault;
pub mod vault_utils;

// Re-exports from old paths for backward compatibility
// These will be removed in a future version
pub use terminal::controller::TerminalController;
pub use terminal::palette::scheme_vt_bytes;
pub use terminal::selection::TerminalSelection;
