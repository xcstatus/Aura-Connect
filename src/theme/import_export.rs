//! 配色方案导入导出模块
//!
//! 提供配色方案的序列化、导入和导出功能。

use crate::theme::user_scheme::{ExportedScheme, UserColorScheme};

/// 导入结果
#[derive(Debug, Clone)]
pub enum ImportResult {
    /// 成功导入为用户配色
    UserScheme(UserColorScheme),
    /// 格式不兼容
    IncompatibleVersion,
    /// 解析失败
    ParseError(String),
}

/// 导入配色方案
///
/// 支持从 JSON 字符串导入用户配色。
pub fn import_from_json(json: &str) -> ImportResult {
    // 尝试解析为 ExportedScheme
    match serde_json::from_str::<ExportedScheme>(json) {
        Ok(exported) => {
            if !exported.is_compatible() {
                return ImportResult::IncompatibleVersion;
            }

            // 如果是用户配色格式
            if let Some(based_on) = exported.based_on {
                let mut scheme = UserColorScheme::new(exported.name, based_on);
                if let Some(overrides) = exported.overrides {
                    for (field, value) in overrides {
                        scheme.set_override(&field, value.as_deref());
                    }
                }
                ImportResult::UserScheme(scheme)
            } else {
                ImportResult::ParseError("缺少 based_on 字段".to_string())
            }
        }
        Err(e) => ImportResult::ParseError(format!("JSON 解析失败: {}", e)),
    }
}

/// 导出用户配色为 JSON 字符串
pub fn export_user_scheme_to_json(scheme: &UserColorScheme) -> String {
    let exported = ExportedScheme::from_user_scheme(scheme);
    serde_json::to_string_pretty(&exported).unwrap_or_else(|_| "{}".to_string())
}
