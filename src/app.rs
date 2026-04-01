use eframe::egui;
use crate::session::{ProtocolType, AuthMethod};
use crate::session_manager::SessionManager;

pub struct DetailedConfigState {
    pub id: Option<String>,
    pub name: String,
    pub folder: String,
    pub color_tag: [u8; 3],
    pub protocol: ProtocolType,
    // SSH
    pub ssh_host: String,
    pub ssh_port: String,
    pub ssh_user: String,
    pub ssh_auth: AuthMethod,
    pub ssh_password: secrecy::SecretString,
    pub ssh_private_key_path: String,
    pub ssh_passphrase: secrecy::SecretString,
    pub ui_show_password: bool,
    pub ui_show_passphrase: bool,
    // Advanced (UI only for now)
    pub keep_alive_secs: u32,
    pub connect_timeout_secs: u32,
    pub proxy_type: String, // "无" | "SOCKS5" | "HTTP"
    pub proxy_host: String,
    pub proxy_port: String,
    pub heartbeat_enabled: bool,
    pub heartbeat_interval_secs: u32,
    // Serial
    pub serial_port: String,
    pub serial_baud: String,
    // Telnet
    pub telnet_host: String,
    pub telnet_port: String,
}

impl Default for DetailedConfigState {
    fn default() -> Self {
        Self {
            id: None,
            name: String::new(),
            folder: "Default".to_string(),
            color_tag: [0, 120, 215],
            protocol: ProtocolType::SSH,
            ssh_host: String::new(),
            ssh_port: "22".to_string(),
            ssh_user: String::new(),
            ssh_auth: AuthMethod::Password,
            ssh_password: secrecy::SecretString::from("".to_string()),
            ssh_private_key_path: String::new(),
            ssh_passphrase: secrecy::SecretString::from("".to_string()),
            ui_show_password: false,
            ui_show_passphrase: false,
            keep_alive_secs: 60,
            connect_timeout_secs: 10,
            proxy_type: "无".to_string(),
            proxy_host: String::new(),
            proxy_port: String::new(),
            heartbeat_enabled: false,
            heartbeat_interval_secs: 30,
            serial_port: String::new(),
            serial_baud: "115200".to_string(),
            telnet_host: String::new(),
            telnet_port: "23".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingTab {
    General,
    Terminal,
    Connection,
    Security,
    Backup,
}

pub struct RustSshApp {
    pub show_settings: bool,
    pub active_settings_tab: SettingTab,
    pub show_add_button: bool,
    pub show_quick_connect: bool,
    pub editing_session: Option<DetailedConfigState>,
    pub search_text: String,
    pub tabs: Vec<String>,
    pub active_tab_index: usize,
    pub session_manager: SessionManager,
    pub settings: crate::settings::Settings,
    pub i18n: crate::i18n::I18n,
    pub needs_restart: bool,
    pub settings_search_text: String,
    pub active_tab_indices: std::collections::HashMap<SettingTab, usize>,
    pub password_change: Option<PasswordChangeState>,
    pub terminal_view: crate::ui_egui::components::terminal_view::TerminalView,
    pub active_session: Option<Box<dyn crate::backend::ssh_session::AsyncSession>>,
    pub connection_prompt: Option<ConnectionPromptState>,
    pub biometrics_auth_in_progress: bool,
    pub biometrics_auth_target: bool,
    pub biometrics_auth_start_time: f64,
    pub session_editor_tab: usize,
    pub session_editor_nav: usize,
    pub session_test_in_progress: bool,
    pub session_test_started_at: f64,
    pub session_toasts: Vec<Toast>,
    pub recent_session_ids: Vec<String>,
    #[cfg(debug_assertions)]
    pub diag_last_focused_id: Option<String>,
}

#[derive(Clone)]
pub struct Toast {
    pub message: String,
    pub is_error: bool,
    pub created_at: f64,
}

pub struct ConnectionPromptState {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: secrecy::SecretString,
    pub is_connecting: bool,
    pub error: Option<String>,
    pub success_session: std::sync::Arc<tokio::sync::Mutex<Option<Box<dyn crate::backend::ssh_session::AsyncSession>>>>,
    pub error_msg: std::sync::Arc<tokio::sync::Mutex<Option<String>>>,
}

pub struct PasswordChangeState {
    pub mode: PasswordChangeMode,
    pub old_password: secrecy::SecretString,
    pub new_password: secrecy::SecretString,
    pub confirm_password: secrecy::SecretString,
    pub is_busy: bool,
    pub progress: f32,
    pub error_msg: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordChangeMode {
    Initialize,
    Change,
}

impl Default for PasswordChangeState {
    fn default() -> Self {
        Self::for_change()
    }
}

impl PasswordChangeState {
    pub fn for_init() -> Self {
        Self {
            mode: PasswordChangeMode::Initialize,
            old_password: secrecy::SecretString::from("".to_string()),
            new_password: secrecy::SecretString::from("".to_string()),
            confirm_password: secrecy::SecretString::from("".to_string()),
            is_busy: false,
            progress: 0.0,
            error_msg: None,
        }
    }

    pub fn for_change() -> Self {
        Self {
            mode: PasswordChangeMode::Change,
            old_password: secrecy::SecretString::from("".to_string()),
            new_password: secrecy::SecretString::from("".to_string()),
            confirm_password: secrecy::SecretString::from("".to_string()),
            is_busy: false,
            progress: 0.0,
            error_msg: None,
        }
    }
}

impl RustSshApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 配置中文字体支持
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "CustomFont".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/fonts/Songti.ttc")).into(),
        );
        fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
            .insert(0, "CustomFont".to_owned());
        fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap()
            .push("CustomFont".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        let sessions_path = crate::storage::StorageManager::get_sessions_path()
            .unwrap_or_else(|| std::path::PathBuf::from("sessions.json"));
        let store = std::sync::Arc::new(crate::repository::JsonSessionStore::new(sessions_path.clone()));
        let mut sm = SessionManager::new(store);
        let settings = crate::settings::Settings::load();
        let i18n = crate::i18n::I18n::new(crate::i18n::Locale::from_language_code(
            &settings.general.language,
        ));

        // Load persisted sessions into app state at startup so saved
        // connections are visible immediately after restart.
        if let Ok(content) = std::fs::read_to_string(&sessions_path) {
            if let Ok(library) = serde_json::from_str::<crate::session::SessionLibrary>(&content) {
                sm.library.sessions = library.sessions;
            }
        }

        Self {
            show_settings: false,
            active_settings_tab: SettingTab::General,
            show_add_button: true,
            show_quick_connect: false,
            editing_session: None, 
            search_text: String::new(),
            tabs: Vec::new(),
            active_tab_index: 0,
            session_manager: sm,
            settings,
            i18n,
            needs_restart: false,
            settings_search_text: String::new(),
            active_tab_indices: std::collections::HashMap::new(),
            password_change: None,
            terminal_view: crate::ui_egui::components::terminal_view::TerminalView::new(),
            active_session: Some(Box::new(crate::backend::ssh_session::SshSession::new())),
            connection_prompt: None,
            biometrics_auth_in_progress: false,
            biometrics_auth_target: false,
            biometrics_auth_start_time: 0.0,
            session_editor_tab: 0,
            session_editor_nav: 0,
            session_test_in_progress: false,
            session_test_started_at: 0.0,
            session_toasts: Vec::new(),
            recent_session_ids: Vec::new(),
            #[cfg(debug_assertions)]
            diag_last_focused_id: None,
        }
    }
}

impl eframe::App for RustSshApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(debug_assertions)]
        {
            let focused_id = ctx
                .memory(|mem| mem.focused())
                .map(|id| format!("{id:?}"));
            if self.diag_last_focused_id != focused_id {
                let name = focused_id.as_deref().and_then(|s| {
                    let map = [
                        (format!("{:?}", egui::Id::new("terminal_view_focus_global")), "terminal_view"),
                        (format!("{:?}", egui::Id::new("quick_connect_search")), "quick_connect_search"),
                        (format!("{:?}", egui::Id::new("password_modal_old_pwd")), "password_modal_old_pwd"),
                        (format!("{:?}", egui::Id::new("password_modal_new_pwd")), "password_modal_new_pwd"),
                        (format!("{:?}", egui::Id::new("password_modal_confirm_pwd")), "password_modal_confirm_pwd"),
                    ];
                    map.iter().find_map(|(id_dbg, label)| (id_dbg == s).then_some(*label))
                });
                log::info!(
                    target: "term-diag",
                    "[term-diag] global_focus_change from={:?} to={:?} to_name={}",
                    self.diag_last_focused_id,
                    focused_id,
                    name.unwrap_or("unknown")
                );
                self.diag_last_focused_id = focused_id;
            }
        }
        let theme = crate::ui_egui::theme::Theme::dark();
        let truncate_middle = |s: &str, max_chars: usize| -> String {
            if s.chars().count() <= max_chars {
                return s.to_string();
            }
            if max_chars <= 3 {
                return "...".to_string();
            }
            let keep = (max_chars - 3) / 2;
            let left: String = s.chars().take(keep).collect();
            let right: String = s
                .chars()
                .rev()
                .take(keep)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            format!("{left}...{right}")
        };

        if ctx.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::K))) {
            self.show_quick_connect = !self.show_quick_connect;
        }

        let window_frame = egui::containers::Frame {
            fill: theme.bg_header,
            inner_margin: egui::Margin::ZERO,
            corner_radius: egui::CornerRadius::same(10),
            stroke: egui::Stroke::NONE,
            ..Default::default()
        };

        // 1. 顶部栏 - 固定在最上方
        egui::TopBottomPanel::top("header_panel")
            .min_height(32.0)
            .frame(window_frame)
            .show_separator_line(false)
            .show(ctx, |ui: &mut egui::Ui| {
                crate::ui_egui::components::header::render_header(self, ui, &theme);
                
                // 渲染重启提醒 Banner
                if self.needs_restart {
                    let banner_frame = egui::Frame::default()
                        .fill(egui::Color32::from_rgb(255, 165, 0).gamma_multiply(0.2)) // 橙色半透明背景
                        .inner_margin(egui::Margin::symmetric(16, 8))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 165, 0)));
                    
                    egui::TopBottomPanel::top("restart_banner")
                        .frame(banner_frame)
                        .show_inside(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("⚠️ 部分设置需要重启应用后生效。").color(egui::Color32::WHITE));
                                ui.add_space(12.0);
                                if ui.button("立即重启").clicked() {
                                    // 这里暂时只重置标志位，实际应用中会调用重启逻辑
                                    self.needs_restart = false;
                                }
                                if ui.button("稍后处理").clicked() {
                                    self.needs_restart = false;
                                }
                            });
                        });
                }
            });

        // 2. Breadcrumb Bar - 顶栏下方的信息与快捷操作区
        egui::TopBottomPanel::top("breadcrumb_bar")
            .exact_height(32.0)
            .frame(egui::Frame::default().fill(theme.bg_secondary))
            .show_separator_line(false)
            .show(ctx, |ui: &mut egui::Ui| {
                let connected = self
                    .active_session
                    .as_ref()
                    .is_some_and(|s| s.is_connected());
                let path_color = if connected {
                    theme.text_primary
                } else {
                    egui::Color32::from_rgb(108, 108, 112)
                };
                let left_node = if let Some(tab) = self.tabs.get(self.active_tab_index) {
                    tab.clone()
                } else if let Some(prompt) = &self.connection_prompt {
                    format!("{}@{}", prompt.user, prompt.host)
                } else {
                    "未连接".to_string()
                };
                let right_node = if let Some(prompt) = &self.connection_prompt {
                    prompt.host.clone()
                } else if let Some(tab) = self.tabs.get(self.active_tab_index) {
                    tab.clone()
                } else {
                    "-".to_string()
                };
                let cwd = self
                    .terminal_view
                    .current_cwd()
                    .map(|v| truncate_middle(v, 42))
                    .or_else(|| {
                        std::env::current_dir()
                            .ok()
                            .map(|p| truncate_middle(&p.display().to_string(), 42))
                    })
                    .unwrap_or_else(|| "-".to_string());

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(truncate_middle(&left_node, 18))
                            .size(11.0)
                            .strong()
                            .color(if connected { theme.accent_base } else { path_color }),
                    );
                    ui.label(egui::RichText::new("/").size(11.0).color(theme.text_secondary));
                    ui.label(
                        egui::RichText::new(truncate_middle(&right_node, 20))
                            .size(11.0)
                            .strong()
                            .color(path_color),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(cwd)
                            .family(egui::FontFamily::Monospace)
                            .size(11.0)
                            .color(theme.text_secondary),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("⋮").size(14.0).color(theme.text_secondary));
                        let secondary_button_stroke =
                            egui::Stroke::new(1.0, theme.border_subtle.gamma_multiply(1.8));

                        let port_btn = ui.add_sized(
                            [50.0, 22.0],
                            egui::Button::new(egui::RichText::new("端口").size(10.0).color(theme.text_primary))
                                .fill(theme.surface_1)
                                .stroke(secondary_button_stroke)
                                .corner_radius(egui::CornerRadius::same(4)),
                        );
                        let _ = port_btn.clicked();

                        let sftp_btn = ui.add_sized(
                            [50.0, 22.0],
                            egui::Button::new(egui::RichText::new("SFTP").size(10.0).color(theme.text_primary))
                                .fill(theme.surface_1)
                                .stroke(secondary_button_stroke)
                                .corner_radius(egui::CornerRadius::same(4)),
                        );
                        let _ = sftp_btn.clicked();

                        let reconnect_btn = ui.add_sized(
                            [76.0, 22.0],
                            egui::Button::new(egui::RichText::new("重新连接").size(10.0).color(theme.text_primary))
                                .fill(theme.surface_1)
                                .stroke(secondary_button_stroke)
                                .corner_radius(egui::CornerRadius::same(4)),
                        );
                        if reconnect_btn.clicked() {
                            self.show_quick_connect = true;
                        }
                    });
                });
            });

        // 3. 底部栏 - 固定在最下方
        egui::TopBottomPanel::bottom("bottom_bar")
            .frame(egui::Frame::default().fill(theme.bg_secondary).inner_margin(egui::vec2(10.0, 4.0)))
            .show_separator_line(false)
            .show(ctx, |ui: &mut egui::Ui| {
                ui.horizontal(|ui: &mut egui::Ui| {
                    let (conn_label, conn_color) = if let Some(prompt) = &self.connection_prompt {
                        if prompt.is_connecting {
                            (format!("连接中: {}@{}:{}", prompt.user, prompt.host, prompt.port), theme.accent_base)
                        } else if let Some(err) = &prompt.error {
                            (format!("连接失败: {}", err), egui::Color32::from_rgb(255, 80, 80))
                        } else {
                            (format!("待连接: {}@{}:{}", prompt.user, prompt.host, prompt.port), theme.text_secondary)
                        }
                    } else if self
                        .active_session
                        .as_ref()
                        .is_some_and(|s| s.is_connected())
                    {
                        let title = self
                            .tabs
                            .get(self.active_tab_index)
                            .cloned()
                            .unwrap_or_else(|| "会话".to_string());
                        (format!("已连接: {}", title), theme.accent_base)
                    } else {
                        ("未连接".to_string(), theme.text_secondary)
                    };

                    ui.label(egui::RichText::new(conn_label).color(conn_color));
                    ui.separator();
                    // Vault flow is deprecated in egui UI; Iced owns vault UX.
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                        ui.label("14:30:45");
                    });
                });
            });

        // 4. 主内容区域 - 自动填充中间部分
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(theme.bg_secondary).inner_margin(0.0))
            .show(ctx, |ui: &mut egui::Ui| {
                self.terminal_view.render(
                    ui,
                    &theme,
                    &mut self.active_session,
                    &self.settings.terminal,
                    self.settings.terminal.keep_selection_highlight,
                    self.settings.terminal.history_search_enabled,
                    self.settings.terminal.local_path_completion_enabled,
                    frame,
                );
            });

        // 5. 模态框与弹出层
        crate::ui_egui::components::modals::render_quick_connect(self, ctx, &theme, 32.0);
        crate::ui_egui::components::modals::render_detailed_config(self, ctx);
        crate::ui_egui::components::modals::render_connection_modal(self, ctx, &theme);
        crate::ui_egui::settings::render_settings_modal(self, ctx, &theme);
        // Vault password modal deprecated (Iced-only).
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // 在应用退出时执行显式的资源释放和持久化
        let _ = self.settings.save();
        
        // 如果有其他运行中的任务（如未收割的异步任务句柄），可在此通过信号量或通知机制触发关闭
        log::info!("RustSSH Application is shutting down. Resources cleared.");
        
        // 显式强制退出，防止某些平台相关的异步任务或系统辅助进程（如 AutoFill）残留
        std::process::exit(0);
    }
}
