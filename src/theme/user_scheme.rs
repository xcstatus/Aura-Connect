//! 用户自定义配色管理模块
//!
//! 提供用户配色方案的创建、编辑、删除和序列化功能。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// 获取当前 Unix 时间戳（毫秒）
fn current_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 用户自定义配色方案
///
/// 用户配色基于某个内置预设，通过覆盖特定颜色字段来实现定制化。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserColorScheme {
    /// 唯一 ID（格式: user_<timestamp>）
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 基于的内置预设 ID（如 "terminal_dark", "nord" 等）
    pub based_on: String,
    /// 颜色覆盖映射表
    /// - Key: 颜色字段名（如 "bg_primary", "accent_base", "term_bg"）
    /// - Value: 覆盖的 HEX 字符串（格式 "#RRGGBB"），None 表示重置为衍生值
    #[serde(default)]
    pub overrides: HashMap<String, Option<String>>,
    /// 创建时间（Unix 毫秒）
    pub created_at: i64,
    /// 最后修改时间（Unix 毫秒）
    pub modified_at: i64,
}

impl UserColorScheme {
    /// 创建新的用户配色方案
    pub fn new(name: String, based_on: String) -> Self {
        let now = current_time_ms();
        Self {
            id: format!("user_{}", now),
            name,
            based_on,
            overrides: HashMap::new(),
            created_at: now,
            modified_at: now,
        }
    }

    /// 设置颜色覆盖
    pub fn set_override(&mut self, field: &str, hex_value: Option<&str>) {
        self.overrides.insert(field.to_string(), hex_value.map(String::from));
        self.modified_at = current_time_ms();
    }

    /// 移除颜色覆盖（重置为衍生值）
    pub fn remove_override(&mut self, field: &str) {
        self.overrides.remove(field);
        self.modified_at = current_time_ms();
    }

    /// 检查是否有指定的覆盖
    pub fn has_override(&self, field: &str) -> bool {
        self.overrides.contains_key(field)
    }

    /// 获取指定字段的覆盖值
    pub fn get_override(&self, field: &str) -> Option<&Option<String>> {
        self.overrides.get(field)
    }
}

/// 用户配色方案列表
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserColorSchemes {
    /// 用户配色方案列表
    pub schemes: Vec<UserColorScheme>,
}

impl UserColorSchemes {
    /// 创建空的用户配色列表
    pub fn new() -> Self {
        Self { schemes: Vec::new() }
    }

    /// 根据 ID 查找用户配色
    pub fn find_by_id(&self, id: &str) -> Option<&UserColorScheme> {
        self.schemes.iter().find(|s| s.id == id)
    }

    /// 根据 ID 查找可变的用户配色
    pub fn find_by_id_mut(&mut self, id: &str) -> Option<&mut UserColorScheme> {
        self.schemes.iter_mut().find(|s| s.id == id)
    }

    /// 添加新的用户配色
    pub fn add(&mut self, scheme: UserColorScheme) {
        self.schemes.push(scheme);
    }

    /// 删除用户配色
    pub fn remove(&mut self, id: &str) -> bool {
        if let Some(pos) = self.schemes.iter().position(|s| s.id == id) {
            self.schemes.remove(pos);
            true
        } else {
            false
        }
    }

    /// 更新用户配色
    pub fn update(&mut self, scheme: UserColorScheme) -> bool {
        if let Some(existing) = self.find_by_id_mut(&scheme.id) {
            *existing = scheme;
            true
        } else {
            false
        }
    }

    /// 获取所有用户配色数量
    pub fn len(&self) -> usize {
        self.schemes.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.schemes.is_empty()
    }
}

/// 导出格式版本
const EXPORT_VERSION: &str = "1.0";

/// 配色方案导出格式
///
/// 用于将配色方案导出为 JSON 文件进行分享或备份。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedScheme {
    /// 格式版本
    pub version: String,
    /// 配色方案名称
    pub name: String,
    /// 基于的内置预设 ID（仅用户配色有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub based_on: Option<String>,
    /// 颜色覆盖映射（仅用户配色有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<String, Option<String>>>,
}

impl ExportedScheme {
    /// 创建用户配色的导出格式
    pub fn from_user_scheme(scheme: &UserColorScheme) -> Self {
        Self {
            version: EXPORT_VERSION.to_string(),
            name: scheme.name.clone(),
            based_on: Some(scheme.based_on.clone()),
            overrides: if scheme.overrides.is_empty() {
                None
            } else {
                Some(scheme.overrides.clone())
            },
        }
    }

    /// 验证导出格式版本兼容性
    pub fn is_compatible(&self) -> bool {
        self.version == EXPORT_VERSION
    }
}

/// 可编辑的颜色字段定义
///
/// 列出所有用户可编辑的颜色字段及其分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorFieldCategory {
    /// 基础/UI 颜色
    Base,
    /// 终端颜色
    Terminal,
    /// ANSI 调色板
    Ansi,
}

/// 可编辑的颜色字段元数据
#[derive(Debug, Clone)]
pub struct ColorFieldMeta {
    /// 字段名（与 ColorScheme 对应）
    pub field: &'static str,
    /// 显示名称
    pub display_name: &'static str,
    /// 所属分类
    pub category: ColorFieldCategory,
    /// 说明文字
    pub description: &'static str,
}

/// 用户可编辑的所有颜色字段
pub const COLOR_FIELDS: &[ColorFieldMeta] = &[
    // === 基础 UI 颜色 ===
    ColorFieldMeta {
        field: "bg_primary",
        display_name: "主背景",
        category: ColorFieldCategory::Base,
        description: "应用主背景色",
    },
    ColorFieldMeta {
        field: "bg_secondary",
        display_name: "次要背景",
        category: ColorFieldCategory::Base,
        description: "次要区域背景",
    },
    ColorFieldMeta {
        field: "text_primary",
        display_name: "主文本",
        category: ColorFieldCategory::Base,
        description: "主要文字颜色",
    },
    ColorFieldMeta {
        field: "text_secondary",
        display_name: "次要文本",
        category: ColorFieldCategory::Base,
        description: "次要/辅助文字",
    },
    ColorFieldMeta {
        field: "accent_base",
        display_name: "强调色",
        category: ColorFieldCategory::Base,
        description: "主要强调色/高亮色",
    },
    ColorFieldMeta {
        field: "selection_bg",
        display_name: "选中背景",
        category: ColorFieldCategory::Base,
        description: "选中区域背景色",
    },
    // === 终端颜色 ===
    ColorFieldMeta {
        field: "term_bg",
        display_name: "终端背景",
        category: ColorFieldCategory::Terminal,
        description: "终端模拟器背景色",
    },
    ColorFieldMeta {
        field: "term_fg",
        display_name: "终端前景",
        category: ColorFieldCategory::Terminal,
        description: "终端默认前景色",
    },
    ColorFieldMeta {
        field: "term_cursor",
        display_name: "光标色",
        category: ColorFieldCategory::Terminal,
        description: "终端光标颜色",
    },
    ColorFieldMeta {
        field: "term_selection_bg",
        display_name: "终端选中背景",
        category: ColorFieldCategory::Terminal,
        description: "终端文本选中背景",
    },
];

/// 根据分类获取所有字段
pub fn get_fields_by_category(category: ColorFieldCategory) -> Vec<&'static ColorFieldMeta> {
    COLOR_FIELDS
        .iter()
        .filter(|f| f.category == category)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_scheme_creation() {
        let mut scheme = UserColorScheme::new("My Theme".to_string(), "terminal_dark".to_string());
        assert!(scheme.id.starts_with("user_"));
        assert_eq!(scheme.name, "My Theme");
        assert_eq!(scheme.based_on, "terminal_dark");
        assert!(scheme.overrides.is_empty());
    }

    #[test]
    fn test_user_scheme_overrides() {
        let mut scheme = UserColorScheme::new("Test".to_string(), "nord".to_string());

        scheme.set_override("bg_primary", Some("#FF0000"));
        assert!(scheme.has_override("bg_primary"));
        assert_eq!(scheme.get_override("bg_primary"), Some(&Some("#FF0000".to_string())));

        scheme.set_override("bg_primary", None);
        assert_eq!(scheme.get_override("bg_primary"), Some(&None));

        scheme.remove_override("bg_primary");
        assert!(!scheme.has_override("bg_primary"));
    }

    #[test]
    fn test_user_schemes_list() {
        let mut list = UserColorSchemes::new();
        assert!(list.is_empty());

        let scheme = UserColorScheme::new("Theme 1".to_string(), "terminal_dark".to_string());
        list.add(scheme);

        assert_eq!(list.len(), 1);
        assert!(list.find_by_id("user_123").is_none());

        let id = list.schemes[0].id.clone();
        assert!(list.find_by_id(&id).is_some());

        list.remove(&id);
        assert!(list.is_empty());
    }

    #[test]
    fn test_export_format() {
        let scheme = UserColorScheme::new("Export Test".to_string(), "nord".to_string());
        let exported = ExportedScheme::from_user_scheme(&scheme);

        assert_eq!(exported.version, "1.0");
        assert_eq!(exported.name, "Export Test");
        assert_eq!(exported.based_on, Some("nord".to_string()));
        assert!(exported.is_compatible());
    }
}
