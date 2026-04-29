fn main() -> iced::Result {
    // 确保系统 locale 正确设置，使原生文件对话框跟随系统语言
    #[cfg(target_os = "macos")]
    {
        if std::env::var("LANG").is_err() {
            // 尝试从系统获取 locale
            if let Ok(output) = std::process::Command::new("defaults")
                .args(["read", "NSGlobalDomain", "AppleLocale"])
                .output()
            {
                if let Ok(locale) = String::from_utf8(output.stdout) {
                    let locale = locale.trim();
                    // 将 AppleLocale (如 "zh_CN") 转换为 POSIX 格式 (如 "zh_CN.UTF-8")
                    let lang = if locale.contains('.') {
                        locale.to_string()
                    } else {
                        format!("{}.UTF-8", locale)
                    };
                    unsafe { std::env::set_var("LANG", &lang); }
                }
            }
        }
    }

    let _logging = rust_ssh::logging::init().expect("logging init failed");
    rust_ssh::app::run()
}
