//! RustSSH 设计系统 → 代码：色票、布局 token、Iced 主题桥接（`doc/设计系统_色卡.md`，`doc/组件设计规范.md`）。

mod iced_bridge;
pub mod layout;
pub mod tokens;

pub use iced_bridge::{
    default_rustssh_iced_theme, iced_extended, iced_palette, rustssh_iced_theme,
};
pub use tokens::{DesignTokens, RustSshThemeId, TerminalAnsiTokens};
