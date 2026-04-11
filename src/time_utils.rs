//! 时间工具模块：处理 Unix 时间戳到本地时区的格式化转换

use chrono::{Local, TimeZone, Utc};

/// 将 Unix 毫秒时间戳格式化为本地时区的字符串
///
/// 格式：`yyyy-MM-dd HH:mm:ss`
///
/// # Arguments
/// * `ms` - Unix 毫秒时间戳
///
/// # Returns
/// 格式化后的日期时间字符串，如果时间戳无效则返回空字符串
pub fn format_local_datetime(ms: i64) -> String {
    if ms <= 0 {
        return String::new();
    }
    let secs = ms / 1000;
    match Utc.timestamp_opt(secs, 0).single() {
        Some(dt) => dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string(),
        None => String::new(),
    }
}
