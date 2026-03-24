/// 密码强度检测工具
/// 返回 0-5 的强度评分
pub fn check_password_strength(password: &str) -> usize {
    if password.is_empty() { return 0; }
    
    let mut score = 0;
    
    // 1. 长度检查
    if password.len() >= 8 { score += 1; }
    if password.len() >= 12 { score += 1; }
    
    // 2. 字符多样性
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_number = password.chars().any(|c| c.is_numeric());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());
    
    if has_uppercase && has_lowercase { score += 1; }
    if has_number { score += 1; }
    if has_special { score += 1; }
    
    score.min(5)
}
