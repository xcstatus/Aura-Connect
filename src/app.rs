use eframe::egui;
use crate::session::{ProtocolType, AuthMethod};
use crate::session_manager::SessionManager;

pub struct DetailedConfigState {
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
            name: String::new(),
            folder: "Default".to_string(),
            color_tag: [0, 120, 215],
            protocol: ProtocolType::SSH,
            ssh_host: String::new(),
            ssh_port: "22".to_string(),
            ssh_user: String::new(),
            ssh_auth: AuthMethod::Password,
            ssh_password: secrecy::SecretString::from("".to_string()),
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
    pub needs_restart: bool,
    pub settings_search_text: String,
    pub active_tab_indices: std::collections::HashMap<SettingTab, usize>,
    pub password_change: Option<PasswordChangeState>,
    pub terminal_view: crate::ui::components::terminal_view::TerminalView,
    pub active_session: Option<Box<dyn crate::backend::ssh_session::AsyncSession>>,
    pub connection_prompt: Option<ConnectionPromptState>,
}

pub struct ConnectionPromptState {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: secrecy::SecretString,
    pub is_connecting: bool,
    pub error: Option<String>,
    pub success_session: std::sync::Arc<tokio::sync::Mutex<Option<Box<dyn crate::backend::ssh_session::AsyncSession>>>>,
}

pub struct PasswordChangeState {
    pub old_password: secrecy::SecretString,
    pub new_password: secrecy::SecretString,
    pub confirm_password: secrecy::SecretString,
    pub is_busy: bool,
    pub progress: f32,
    pub error_msg: Option<String>,
}

impl Default for PasswordChangeState {
    fn default() -> Self {
        Self {
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
        let store = std::sync::Arc::new(crate::repository::JsonSessionStore::new(sessions_path));
        let sm = SessionManager::new(store);
        
        let ctx = cc.egui_ctx.clone();
        let mut sm_clone = sm.clone();
        tokio::spawn(async move {
            let _ = sm_clone.load().await;
            ctx.request_repaint();
        });

        Self {
            show_settings: false,
            active_settings_tab: SettingTab::General,
            show_add_button: true,
            show_quick_connect: false,
            editing_session: None, 
            search_text: String::new(),
            tabs: vec![
                "root@server1".to_string(),
                "user@database".to_string(),
                "admin@web-01".to_string(),
                "backup@store".to_string(),
                "root@gateway".to_string(),
            ],
            active_tab_index: 0,
            session_manager: sm,
            settings: crate::settings::Settings::load(),
            needs_restart: false,
            settings_search_text: String::new(),
            active_tab_indices: std::collections::HashMap::new(),
            password_change: None,
            terminal_view: crate::ui::components::terminal_view::TerminalView::new(),
            active_session: Some(Box::new(crate::backend::ssh_session::SshSession::new())),
            connection_prompt: None,
        }
    }
}

impl eframe::App for RustSshApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let theme = crate::ui::theme::Theme::dark();

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
            .frame(window_frame)
            .show_separator_line(false)
            .show(ctx, |ui: &mut egui::Ui| {
                crate::ui::components::header::render_header(self, ui, &theme);
                
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

        // 2. 底部栏 - 固定在最下方
        egui::TopBottomPanel::bottom("bottom_bar")
            .frame(egui::Frame::default().fill(theme.bg_secondary).inner_margin(egui::vec2(10.0, 4.0)))
            .show_separator_line(false)
            .show(ctx, |ui: &mut egui::Ui| {
                ui.horizontal(|ui: &mut egui::Ui| {
                    ui.label(egui::RichText::new("连接: server1").color(theme.accent_base));
                    ui.separator();
                    ui.label(egui::RichText::new("🔓 Vault 已解锁").color(theme.text_secondary));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                        ui.label("14:30:45");
                    });
                });
            });

        // 3. 主内容区域 - 自动填充中间部分
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::BLACK).inner_margin(0.0))
            .show(ctx, |ui: &mut egui::Ui| {
                self.terminal_view.render(ui, &theme, &mut self.active_session);
            });

        // 4. 模态框与弹出层
        crate::ui::components::modals::render_quick_connect(self, ctx, &theme, 32.0);
        crate::ui::components::modals::render_detailed_config(self, ctx);
        crate::ui::components::modals::render_connection_modal(self, ctx, &theme);
        crate::ui::settings::render_settings_modal(self, ctx, &theme);
        crate::ui::components::password_modal::render_password_change_modal(self, ctx, &theme);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // 在应用退出时执行显式的资源释放和持久化
        let _ = self.settings.save();
        
        // 如果有其他运行中的任务（如未收割的异步任务句柄），可在此通过信号量或通知机制触发关闭
        println!("RustSSH Application is shutting down. Resources cleared.");
        
        // 显式强制退出，防止某些平台相关的异步任务或系统辅助进程（如 AutoFill）残留
        std::process::exit(0);
    }
}
