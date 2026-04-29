//! Liquid Glass 按钮样式重导出模块。
//!
//! **已废弃**：所有样式函数已迁移到 `crate::theme::liquid_glass`。
//! 请使用以下导入路径：
//!
//! ```ignore
//! use crate::theme::liquid_glass::{
//!     glass_primary_button_style,
//!     glass_secondary_button_style,
//!     glass_icon_button_style,
//!     glass_tab_button_style,
//!     glass_danger_button_style,
//! };
//! ```
//!
//! ## 重导出列表
//!
//! | 旧名称 | 新名称（标准） | 说明 |
//! |--------|----------------|------|
//! | `lg_primary_button` | `glass_primary_button_style` | 主要操作按钮 |
//! | `lg_secondary_button` | `glass_secondary_button_style` | 次要操作按钮 |
//! | `lg_icon_button` | `glass_icon_button_style` | 图标按钮（Active 状态有背景） |
//! | `lg_tab_button` | `glass_tab_button_style` | 标签按钮（Active 状态有背景） |
//! | `lg_icon_button_flat` | `glass_icon_button_style` | 扁平图标按钮 |
//! | `lg_danger_button` | `glass_danger_button_style` | 危险操作按钮 |
//!
//! ## 为什么重导出？
//!
//! 本文件保留是为了向后兼容所有现有调用点。
//! 所有样式函数的实现定义在 `crate::theme::liquid_glass` 中。

pub use crate::theme::liquid_glass::glass_danger_button_style as lg_danger_button;
pub use crate::theme::liquid_glass::glass_icon_button_style as lg_icon_button;
pub use crate::theme::liquid_glass::glass_icon_button_style as lg_icon_button_flat;
pub use crate::theme::liquid_glass::glass_primary_button_style as lg_primary_button;
pub use crate::theme::liquid_glass::glass_secondary_button_style as lg_secondary_button;
pub use crate::theme::liquid_glass::glass_tab_button_style as lg_tab_button;
