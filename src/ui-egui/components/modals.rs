use eframe::egui;
use crate::app::{RustSshApp, DetailedConfigState};
use crate::ui_egui::theme::Theme;
use crate::session::ProtocolType;

fn session_group(ui: &mut egui::Ui, theme: &Theme, border_subtle: egui::Color32, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme.surface_1)
        .stroke(egui::Stroke::new(1.0, border_subtle))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(20))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(title).size(14.0).strong().color(egui::Color32::WHITE));
            ui.add_space(12.0);
            add(ui);
        });
}

fn session_row(
    ui: &mut egui::Ui,
    label_w: f32,
    label: &str,
    required: bool,
    label_color: egui::Color32,
    add: impl FnOnce(&mut egui::Ui) -> egui::Response,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.set_min_height(36.0);
        let mut l = label.to_string();
        if required {
            l.push_str(" *");
        }
        ui.add_sized([label_w, 20.0], egui::Label::new(egui::RichText::new(l).size(12.0).color(label_color)));
        add(ui)
    })
    .inner
}

fn mark_recent_connection(app: &mut RustSshApp, session_id: &str) {
    app.recent_session_ids.retain(|id| id != session_id);
    app.recent_session_ids.insert(0, session_id.to_string());
    if app.recent_session_ids.len() > 20 {
        app.recent_session_ids.truncate(20);
    }
}

fn load_prefilled_password(_app: &RustSshApp, _ssh: &crate::session::SshConfig) -> secrecy::SecretString {
    // Vault integration is deprecated for egui UI; do not prefill secrets here.
    secrecy::SecretString::from(String::new())
}

pub fn render_quick_connect(app: &mut RustSshApp, ctx: &egui::Context, theme: &Theme, header_height: f32) {
    let focus_once_id = egui::Id::new("quick_connect_focus_once");
    if !app.show_quick_connect {
        ctx.data_mut(|d| d.insert_temp(focus_once_id, true));
        return;
    }

    let area = egui::Area::new(egui::Id::new("quick_connect_area"))
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, header_height + 20.0));
    
    area.show(ctx, |ui| {
        egui::Frame::window(&ctx.style())
            .fill(theme.bg_header)
            .corner_radius(egui::CornerRadius::same(10))
            .shadow(egui::Shadow {
                color: egui::Color32::from_black_alpha(180),
                offset: [0, 8].into(),
                blur: 24,
                ..Default::default()
            })
            .show(ui, |ui| {
                ui.set_width(400.0);
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("⚡").size(18.0));
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut app.search_text)
                                .id(egui::Id::new("quick_connect_search"))
                                .hint_text("输入 root@host:port 或搜索会话...")
                                .desired_width(f32::INFINITY)
                        );
                        let should_focus_once = ctx
                            .data_mut(|d| d.get_temp::<bool>(focus_once_id))
                            .unwrap_or(true);
                        if should_focus_once {
                            resp.request_focus();
                            ctx.data_mut(|d| d.insert_temp(focus_once_id, false));
                        }
                        ui.add_space(8.0);
                    });
                    ui.add_space(4.0);
                    ui.separator();
                    
                    ui.add_space(4.0);
                    let search_lower = app.search_text.to_lowercase();
                    
                    let filtered_sessions: Vec<_> = app.session_manager.library.sessions.iter()
                        .filter(|s| {
                            let name_match = s.name.to_lowercase().contains(&search_lower);
                            let transport_match = match &s.transport {
                                crate::session::TransportConfig::Ssh(ssh) => {
                                    let host = ssh.host.to_lowercase();
                                    let user_host = format!("{}@{}", ssh.user.to_lowercase(), host);
                                    host.contains(&search_lower) || user_host.contains(&search_lower)
                                }
                                crate::session::TransportConfig::Telnet(telnet) => telnet.host.to_lowercase().contains(&search_lower),
                                crate::session::TransportConfig::Serial(serial) => serial.port.to_lowercase().contains(&search_lower),
                            };
                            name_match || transport_match
                        })
                        .cloned()
                        .collect();

                    // 1) 最近连接（最多 6 条，搜索时隐藏）
                    if search_lower.trim().is_empty() {
                        let mut recent_items = Vec::new();
                        for sid in &app.recent_session_ids {
                            if let Some(sess) = app
                                .session_manager
                                .library
                                .sessions
                                .iter()
                                .find(|s| &s.id == sid)
                            {
                                recent_items.push(sess.clone());
                            }
                            if recent_items.len() >= 6 {
                                break;
                            }
                        }
                        if !recent_items.is_empty() {
                            ui.label(egui::RichText::new("  最近连接").weak().size(12.0));
                            for s in &recent_items {
                                let (icon, addr) = match &s.transport {
                                    crate::session::TransportConfig::Ssh(ssh) => ("🖥", ssh.host.clone()),
                                    crate::session::TransportConfig::Telnet(telnet) => ("🌐", telnet.host.clone()),
                                    crate::session::TransportConfig::Serial(serial) => ("🔌", serial.port.clone()),
                                };
                                let display = format!("  {} {} ({})", icon, s.name, addr);
                                if ui.add(egui::Button::new(display).selected(false)).clicked() {
                                    match &s.transport {
                                        crate::session::TransportConfig::Ssh(ssh) => {
                                            app.connection_prompt = Some(crate::app::ConnectionPromptState {
                                                host: ssh.host.clone(),
                                                port: ssh.port,
                                                user: ssh.user.clone(),
                                                password: load_prefilled_password(app, ssh),
                                                is_connecting: false,
                                                error: None,
                                                success_session: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
                                                error_msg: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
                                            });
                                        }
                                        crate::session::TransportConfig::Telnet(telnet) => {
                                            app.tabs.push(telnet.host.clone());
                                            app.active_tab_index = app.tabs.len() - 1;
                                        }
                                        crate::session::TransportConfig::Serial(serial) => {
                                            app.tabs.push(serial.port.clone());
                                            app.active_tab_index = app.tabs.len() - 1;
                                        }
                                    }
                                    mark_recent_connection(app, &s.id);
                                    app.show_quick_connect = false;
                                }
                            }
                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);
                        }
                    }

                    // 2) 新增连接按钮
                    if ui.add(egui::Button::new("➕ 新增连接").selected(false)).clicked() {
                        app.editing_session = Some(DetailedConfigState::default());
                        app.show_quick_connect = false;
                    }
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.label(egui::RichText::new("  已保存连接").weak().size(12.0));
                    if filtered_sessions.is_empty() {
                        let msg = if search_lower.is_empty() {
                            "  暂无已保存连接"
                        } else {
                            "  未找到匹配的连接（支持按名称、IP/主机搜索）"
                        };
                        ui.label(egui::RichText::new(msg).size(12.0).color(theme.text_secondary));
                    } else {
                        let mut grouped: std::collections::BTreeMap<String, Vec<crate::session::SessionProfile>> =
                            std::collections::BTreeMap::new();
                        for s in &filtered_sessions {
                            let group_name = s
                                .folder
                                .as_ref()
                                .map(|f| f.trim().to_string())
                                .filter(|f| !f.is_empty())
                                .unwrap_or_else(|| "Default".to_string());
                            grouped.entry(group_name).or_default().push(s.clone());
                        }

                        for (group_name, sessions) in grouped.iter_mut() {
                            sessions.sort_by_key(|s| s.name.to_lowercase());

                            // 默认组不显示分组名称
                            if group_name != "Default" && group_name != "未分类" {
                                ui.label(
                                    egui::RichText::new(format!("  {}", group_name))
                                        .size(12.0)
                                        .color(theme.text_secondary),
                                );
                            }

                            for s in sessions.iter() {
                                let (icon, addr) = match &s.transport {
                                    crate::session::TransportConfig::Ssh(ssh) => ("🖥", ssh.host.clone()),
                                    crate::session::TransportConfig::Telnet(telnet) => ("🌐", telnet.host.clone()),
                                    crate::session::TransportConfig::Serial(serial) => ("🔌", serial.port.clone()),
                                };
                                let display = format!("  {} {} ({})", icon, s.name, addr);
                                if ui.add(egui::Button::new(display).selected(false)).clicked() {
                                    match &s.transport {
                                        crate::session::TransportConfig::Ssh(ssh) => {
                                            app.connection_prompt = Some(crate::app::ConnectionPromptState {
                                                host: ssh.host.clone(),
                                                port: ssh.port,
                                                user: ssh.user.clone(),
                                                password: load_prefilled_password(app, ssh),
                                                is_connecting: false,
                                                error: None,
                                                success_session: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
                                                error_msg: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
                                            });
                                        }
                                        crate::session::TransportConfig::Telnet(telnet) => {
                                            app.tabs.push(telnet.host.clone());
                                            app.active_tab_index = app.tabs.len() - 1;
                                        }
                                        crate::session::TransportConfig::Serial(serial) => {
                                            app.tabs.push(serial.port.clone());
                                            app.active_tab_index = app.tabs.len() - 1;
                                        }
                                    }
                                    mark_recent_connection(app, &s.id);
                                    app.show_quick_connect = false;
                                }
                            }
                        }
                    }
                    ui.add_space(4.0);
                    
                    if !search_lower.is_empty() && search_lower.contains('@') && !filtered_sessions.iter().any(|s| {
                        match &s.transport {
                            crate::session::TransportConfig::Ssh(ssh) => ssh.host == search_lower || format!("{}@{}", ssh.user, ssh.host) == search_lower,
                            _ => false
                        }
                    }) {
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                        
                        let parts: Vec<&str> = search_lower.split('@').collect();
                        if parts.len() == 2 && !parts[1].is_empty() {
                            let user = parts[0].to_string();
                            let host_port = parts[1];
                            let (host, port) = if host_port.contains(':') {
                                let hp: Vec<&str> = host_port.split(':').collect();
                                (hp[0].to_string(), hp[1].parse::<u16>().unwrap_or(22))
                            } else {
                                (host_port.to_string(), 22)
                            };

                            if ui.add(egui::Button::new(egui::RichText::new(format!("🚀 立即连接: {}@{}:{}", user, host, port)).color(theme.accent_base)).selected(false)).clicked() {
                                // 初始化认证提示，引导进入下一步 (符合 2.2 终端核心功能要求)
                                app.connection_prompt = Some(crate::app::ConnectionPromptState {
                                    host,
                                    port,
                                    user,
                                    password: secrecy::SecretString::from("".to_string()),
                                    is_connecting: false,
                                    error: None,
                                    success_session: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
                                error_msg: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
                                });
                                app.show_quick_connect = false;
                            }
                        }
                    }

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.add_space(8.0);
                });
            });
    });

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.show_quick_connect = false;
    }
}

pub fn render_detailed_config(app: &mut RustSshApp, ctx: &egui::Context) {
    crate::ui_egui::session_editor::render_modal(app, ctx);
}

pub fn render_detailed_config_legacy2(app: &mut RustSshApp, ctx: &egui::Context) {
    if app.editing_session.is_none() {
        return;
    }

    // Esc closes modal (spec)
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.editing_session = None;
        app.session_test_in_progress = false;
        return;
    }

    let theme = crate::ui_egui::theme::Theme::dark();
    let accent = theme.accent_base;
    let text_primary = egui::Color32::WHITE;
    let text_secondary = theme.text_secondary;
    let border_subtle = egui::Color32::from_rgba_premultiplied(255, 255, 255, 10);
    let error = egui::Color32::from_rgb(255, 59, 48);

    let content_rect = ctx.available_rect();
    egui::Area::new(egui::Id::new("modal_overlay_v2"))
        .order(egui::Order::Foreground)
        .fixed_pos(content_rect.min)
        .show(ctx, |ui| {
            ui.painter().rect_filled(
                content_rect,
                egui::CornerRadius::ZERO,
                egui::Color32::from_rgba_premultiplied(0, 0, 0, 153),
            );

            let panel_width = (content_rect.width() * 0.85).clamp(800.0, 1200.0);
            let panel_height = (content_rect.height() * 0.8).max(600.0);
            let panel_rect =
                egui::Rect::from_center_size(content_rect.center(), egui::vec2(panel_width, panel_height));

            let frame = egui::Frame::new()
                .fill(theme.bg_header)
                .stroke(egui::Stroke::new(1.0, border_subtle))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::ZERO)
                .shadow(egui::Shadow {
                    color: egui::Color32::from_black_alpha(120),
                    offset: [0, 8].into(),
                    blur: 24,
                    spread: 0,
                });

            ui.scope_builder(egui::UiBuilder::new().max_rect(panel_rect), |ui| {
                frame.show(ui, |ui| {
                    let sidebar_w = 200.0;
                    let top_h = 32.0;
                    let footer_h = 64.0;

                    ui.horizontal(|ui| {
                        // ---- Left nav ----
                        egui::Frame::new()
                            .fill(theme.bg_secondary)
                            .corner_radius(egui::CornerRadius { nw: 12, ne: 0, sw: 12, se: 0 })
                            .inner_margin(egui::Margin { left: 8, right: 8, top: 32, bottom: 16 })
                            .show(ui, |ui| {
                                ui.set_width(sidebar_w);
                                ui.set_height(panel_height);
                                let full = ui.available_rect_before_wrap();
                                ui.painter().vline(full.right(), full.y_range(), egui::Stroke::new(1.0, border_subtle));

                                let nav_items = [
                                    "通用（General）",
                                    "高级设置（Advanced）",
                                    "端口转发（Port Forwarding）",
                                    "加密方法（Encryption）",
                                ];
                                for (idx, label) in nav_items.iter().enumerate() {
                                    let is_active = app.session_editor_nav == idx;
                                    let mut row = ui.available_rect_before_wrap();
                                    row.set_height(40.0);
                                    let resp = ui.interact(row, ui.id().with(("session_nav", idx)), egui::Sense::click());
                                    let bg = if is_active {
                                        theme.surface_2
                                    } else if resp.hovered() {
                                        theme.surface_1
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    };
                                    ui.painter().rect_filled(row, egui::CornerRadius::same(6), bg);
                                    if is_active {
                                        let strip = egui::Rect::from_min_size(row.min, egui::vec2(2.0, row.height()));
                                        ui.painter().rect_filled(strip, egui::CornerRadius::ZERO, accent);
                                    }
                                    ui.painter().text(
                                        row.left_center() + egui::vec2(12.0, 0.0),
                                        egui::Align2::LEFT_CENTER,
                                        *label,
                                        egui::FontId::proportional(14.0),
                                        if is_active { text_primary } else if resp.hovered() { text_primary } else { text_secondary },
                                    );
                                    if resp.clicked() {
                                        app.session_editor_nav = idx;
                                    }
                                    ui.add_space(8.0);
                                }
                            });

                        // ---- Right content ----
                        ui.vertical(|ui| {
                            ui.set_width(panel_width - sidebar_w);
                            ui.set_height(panel_height);

                            // top area with close button
                            ui.allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), top_h),
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add_space(8.0);
                                    let r = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click()).0;
                                    let resp = ui.interact(r, ui.id().with("session_editor_close"), egui::Sense::click());
                                    if resp.hovered() {
                                        ui.painter().rect_filled(r, egui::CornerRadius::same(6), theme.surface_1);
                                    }
                                    ui.painter().text(
                                        r.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "✕",
                                        egui::FontId::proportional(14.0),
                                        text_secondary,
                                    );
                                    if resp.clicked() {
                                        app.editing_session = None;
                                        app.session_test_in_progress = false;
                                    }
                                },
                            );

                            let content_h = ui.available_height() - footer_h;
                            let mut first_error_rect: Option<egui::Rect> = None;
                            let mut request_test = false;
                            let mut request_save = false;

                            egui::ScrollArea::vertical()
                                .id_salt("session_editor_scroll_v2")
                                .max_height(content_h)
                                .show(ui, |ui| {
                                    ui.add_space(24.0);

                                    if let Some(state) = app.editing_session.as_mut() {
                                        state.protocol = ProtocolType::SSH;

                                        // top fixed grid
                                        ui.horizontal(|ui| {
                                            ui.add_space(24.0);
                                            ui.spacing_mut().item_spacing = egui::vec2(16.0, 16.0);

                                            ui.vertical(|ui| {
                                                ui.label(egui::RichText::new("名称 *").size(12.0).color(text_primary));
                                                ui.add_sized(
                                                    [260.0, 36.0],
                                                    egui::TextEdit::singleline(&mut state.name)
                                                        .desired_width(f32::INFINITY)
                                                        .margin(egui::vec2(12.0, 10.0))
                                                        .background_color(theme.surface_1),
                                                );
                                            });
                                            ui.vertical(|ui| {
                                                ui.label(egui::RichText::new("分组").size(12.0).color(text_primary));
                                                ui.add_sized(
                                                    [260.0, 36.0],
                                                    egui::TextEdit::singleline(&mut state.folder)
                                                        .desired_width(f32::INFINITY)
                                                        .margin(egui::vec2(12.0, 10.0))
                                                        .background_color(theme.surface_1),
                                                );
                                            });
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let btn = egui::Button::new(egui::RichText::new("测试连接").size(12.0).color(text_primary))
                                                    .min_size(egui::vec2(120.0, 36.0))
                                                    .fill(theme.surface_1)
                                                    .stroke(egui::Stroke::new(1.0, border_subtle))
                                                    .corner_radius(egui::CornerRadius::same(6));
                                                let resp = ui.add_enabled(!app.session_test_in_progress, btn);
                                                if app.session_test_in_progress {
                                                    ui.put(
                                                        egui::Rect::from_center_size(resp.rect.left_center() + egui::vec2(16.0, 0.0), egui::vec2(16.0, 16.0)),
                                                        egui::Spinner::new(),
                                                    );
                                                }
                                                if resp.clicked() {
                                                    request_test = true;
                                                }
                                            });
                                        });

                                        ui.add_space(24.0);

                                        session_group(ui, &theme, border_subtle, "通用（General）", |ui| {
                                            let label_w = 100.0;
                                            let host_resp = session_row(ui, label_w, "主机地址", true, text_secondary, |ui| {
                                                ui.add_sized(
                                                    [ui.available_width(), 36.0],
                                                    egui::TextEdit::singleline(&mut state.ssh_host)
                                                        .hint_text("192.168.1.100 或 example.com")
                                                        .margin(egui::vec2(12.0, 10.0))
                                                        .background_color(theme.surface_1),
                                                )
                                            });
                                            if state.ssh_host.trim().is_empty() {
                                                ui.add_space(4.0);
                                                ui.label(egui::RichText::new("主机地址为必填项").size(11.0).color(error));
                                                first_error_rect.get_or_insert(host_resp.rect);
                                            }
                                            ui.add_space(16.0);

                                            let port_resp = session_row(ui, label_w, "端口", true, text_secondary, |ui| {
                                                ui.add_sized(
                                                    [160.0, 36.0],
                                                    egui::TextEdit::singleline(&mut state.ssh_port)
                                                        .margin(egui::vec2(12.0, 10.0))
                                                        .background_color(theme.surface_1),
                                                )
                                            });
                                            let port_ok = state.ssh_port.parse::<u32>().ok().is_some_and(|p| p >= 1 && p <= 65535);
                                            if !port_ok {
                                                ui.add_space(4.0);
                                                ui.label(egui::RichText::new("端口范围 1–65535").size(11.0).color(error));
                                                first_error_rect.get_or_insert(port_resp.rect);
                                            }
                                            ui.add_space(16.0);

                                            let user_resp = session_row(ui, label_w, "用户名", true, text_secondary, |ui| {
                                                ui.add_sized(
                                                    [ui.available_width(), 36.0],
                                                    egui::TextEdit::singleline(&mut state.ssh_user)
                                                        .margin(egui::vec2(12.0, 10.0))
                                                        .background_color(theme.surface_1),
                                                )
                                            });
                                            if state.ssh_user.trim().is_empty() {
                                                ui.add_space(4.0);
                                                ui.label(egui::RichText::new("用户名为必填项").size(11.0).color(error));
                                                first_error_rect.get_or_insert(user_resp.rect);
                                            }
                                            ui.add_space(16.0);

                                            ui.horizontal(|ui| {
                                                ui.add_sized(
                                                    [label_w, 20.0],
                                                    egui::Label::new(
                                                        egui::RichText::new("认证方式 *").size(12.0).color(text_secondary),
                                                    ),
                                                );
                                                let mut current = match &state.ssh_auth {
                                                    crate::session::AuthMethod::Key { .. } => 1,
                                                    _ => 0,
                                                };
                                                let pw = ui.radio_value(&mut current, 0, "密码");
                                                let key = ui.radio_value(&mut current, 1, "私钥");
                                                if pw.clicked() { state.ssh_auth = crate::session::AuthMethod::Password; }
                                                if key.clicked() {
                                                    state.ssh_auth = crate::session::AuthMethod::Key { private_key_path: state.ssh_private_key_path.clone() };
                                                }
                                            });
                                            ui.add_space(16.0);

                                            match &mut state.ssh_auth {
                                                crate::session::AuthMethod::Password => {
                                                    use secrecy::ExposeSecret;
                                                    let mut raw = state.ssh_password.expose_secret().to_string();
                                                    let pwd_resp = session_row(ui, label_w, "密码", true, text_secondary, |ui| {
                                                        ui.horizontal(|ui| {
                                                            let resp = ui.add_sized(
                                                                [ui.available_width() - 64.0, 36.0],
                                                                egui::TextEdit::singleline(&mut raw)
                                                                    .password(!state.ui_show_password)
                                                                    .margin(egui::vec2(12.0, 10.0))
                                                                    .background_color(theme.surface_1),
                                                            );
                                                            if resp.changed() {
                                                                state.ssh_password = secrecy::SecretString::from(raw.clone());
                                                            }
                                                            let toggle = ui.add_sized(
                                                                [56.0, 36.0],
                                                                egui::Button::new(if state.ui_show_password { "隐藏" } else { "显示" })
                                                                    .fill(theme.surface_1)
                                                                    .stroke(egui::Stroke::new(1.0, border_subtle))
                                                                    .corner_radius(egui::CornerRadius::same(6)),
                                                            );
                                                            if toggle.clicked() { state.ui_show_password = !state.ui_show_password; }
                                                            resp
                                                        }).inner
                                                    });
                                                    if raw.trim().is_empty() {
                                                        ui.add_space(4.0);
                                                        ui.label(egui::RichText::new("密码为必填项").size(11.0).color(error));
                                                        first_error_rect.get_or_insert(pwd_resp.rect);
                                                    }
                                                }
                                                crate::session::AuthMethod::Key { private_key_path } => {
                                                    let key_resp = session_row(ui, label_w, "私钥路径", true, text_secondary, |ui| {
                                                        ui.horizontal(|ui| {
                                                            let resp = ui.add_sized(
                                                                [ui.available_width() - 44.0, 36.0],
                                                                egui::TextEdit::singleline(&mut state.ssh_private_key_path)
                                                                    .margin(egui::vec2(12.0, 10.0))
                                                                    .background_color(theme.surface_1),
                                                            );
                                                            *private_key_path = state.ssh_private_key_path.clone();
                                                            let _browse = ui.add_sized(
                                                                [36.0, 36.0],
                                                                egui::Button::new("…")
                                                                    .fill(egui::Color32::TRANSPARENT)
                                                                    .corner_radius(egui::CornerRadius::same(6)),
                                                            );
                                                            resp
                                                        }).inner
                                                    });
                                                    if state.ssh_private_key_path.trim().is_empty() {
                                                        ui.add_space(4.0);
                                                        ui.label(egui::RichText::new("私钥路径为必填项").size(11.0).color(error));
                                                        first_error_rect.get_or_insert(key_resp.rect);
                                                    }
                                                    ui.add_space(16.0);

                                                    use secrecy::ExposeSecret;
                                                    let mut raw = state.ssh_passphrase.expose_secret().to_string();
                                                    let _ = session_row(ui, label_w, "密码短语", false, text_secondary, |ui| {
                                                        ui.horizontal(|ui| {
                                                            let resp = ui.add_sized(
                                                                [ui.available_width() - 64.0, 36.0],
                                                                egui::TextEdit::singleline(&mut raw)
                                                                    .password(!state.ui_show_passphrase)
                                                                    .margin(egui::vec2(12.0, 10.0))
                                                                    .background_color(theme.surface_1),
                                                            );
                                                            if resp.changed() {
                                                                state.ssh_passphrase = secrecy::SecretString::from(raw.clone());
                                                            }
                                                            let toggle = ui.add_sized(
                                                                [56.0, 36.0],
                                                                egui::Button::new(if state.ui_show_passphrase { "隐藏" } else { "显示" })
                                                                    .fill(theme.surface_1)
                                                                    .stroke(egui::Stroke::new(1.0, border_subtle))
                                                                    .corner_radius(egui::CornerRadius::same(6)),
                                                            );
                                                            if toggle.clicked() { state.ui_show_passphrase = !state.ui_show_passphrase; }
                                                            resp
                                                        }).inner
                                                    });
                                                }
                                                _ => {}
                                            }
                                        });

                                        ui.add_space(16.0);
                                        session_group(ui, &theme, border_subtle, "高级设置（Advanced）", |ui| {
                                            ui.label(egui::RichText::new("（UI 已预留字段，后续接入保存结构）").size(12.0).color(text_secondary));
                                        });
                                        ui.add_space(16.0);
                                        session_group(ui, &theme, border_subtle, "端口转发（Port Forwarding）", |ui| {
                                            ui.label(egui::RichText::new("（设计稿：本地/远程转发列表，后续接入）").size(12.0).color(text_secondary));
                                        });
                                        ui.add_space(16.0);
                                        session_group(ui, &theme, border_subtle, "加密方法（Encryption）", |ui| {
                                            ui.label(egui::RichText::new("（设计稿：算法单选 + 压缩/HMAC，后续接入）").size(12.0).color(text_secondary));
                                        });

                                        if ui.ctx().input(|i| i.key_pressed(egui::Key::Enter)) {
                                            request_save = true;
                                        }
                                    }
                                });

                            // footer
                            let mut cancel_clicked = false;
                            let mut save_clicked = false;
                            let footer_rect = ui.allocate_exact_size(egui::vec2(ui.available_width(), footer_h), egui::Sense::hover()).0;
                            ui.painter().hline(footer_rect.x_range(), footer_rect.top(), egui::Stroke::new(1.0, border_subtle));
                            ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.add_space(24.0);
                                    let save = ui.add_sized(
                                        [80.0, 36.0],
                                        egui::Button::new(egui::RichText::new("保存").size(12.0).strong().color(egui::Color32::BLACK))
                                            .fill(accent)
                                            .corner_radius(egui::CornerRadius::same(6)),
                                    );
                                    save_clicked = save.clicked();
                                    ui.add_space(12.0);
                                    let cancel = ui.add_sized(
                                        [80.0, 36.0],
                                        egui::Button::new(egui::RichText::new("取消").size(12.0).color(text_primary))
                                            .fill(theme.surface_1)
                                            .stroke(egui::Stroke::new(1.0, border_subtle))
                                            .corner_radius(egui::CornerRadius::same(6)),
                                    );
                                    cancel_clicked = cancel.clicked();
                                });
                            });

                            let now = ctx.input(|i| i.time);
                            app.session_toasts.retain(|t| now - t.created_at < 3.0);

                            if request_test {
                                app.session_test_in_progress = true;
                                app.session_test_started_at = now;
                                if let Some(state) = app.editing_session.as_ref() {
                                    let host = state.ssh_host.trim().to_string();
                                    let user = state.ssh_user.trim().to_string();
                                    let port = state.ssh_port.parse::<u16>().unwrap_or(22);
                                    if host.is_empty() || user.is_empty() {
                                        app.session_toasts.push(crate::app::Toast { message: "请先填写主机地址与用户名".to_string(), is_error: true, created_at: now });
                                        app.session_test_in_progress = false;
                                    } else {
                                        app.connection_prompt = Some(crate::app::ConnectionPromptState {
                                            host,
                                            port,
                                            user,
                                            password: secrecy::SecretString::from("".to_string()),
                                            is_connecting: false,
                                            error: None,
                                            success_session: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
                                            error_msg: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
                                        });
                                    }
                                }
                            }
                            if app.session_test_in_progress && now - app.session_test_started_at > 3.0 {
                                app.session_test_in_progress = false;
                            }

                            if cancel_clicked {
                                app.editing_session = None;
                                app.session_test_in_progress = false;
                            }

                            if save_clicked || request_save {
                                let mut err_msg: Option<String> = None;
                                if let Some(state) = app.editing_session.as_ref() {
                                    if state.name.trim().is_empty() { err_msg = Some("名称为必填项".to_string()); }
                                    else if state.ssh_host.trim().is_empty() { err_msg = Some("主机地址为必填项".to_string()); }
                                    else if state.ssh_user.trim().is_empty() { err_msg = Some("用户名为必填项".to_string()); }
                                    else if !state.ssh_port.parse::<u32>().ok().is_some_and(|p| p >= 1 && p <= 65535) { err_msg = Some("端口范围 1–65535".to_string()); }
                                }

                                if let Some(msg) = err_msg {
                                    app.session_toasts.push(crate::app::Toast { message: msg, is_error: true, created_at: now });
                                    if let Some(r) = first_error_rect {
                                        ui.scroll_to_rect(r, Some(egui::Align::TOP));
                                    }
                                } else if let Some(state) = app.editing_session.take() {
                                    let mut sm = app.session_manager.clone();
                                    let id = state.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                                    let name = state.name.trim().to_string();
                                    let folder = state.folder.trim().to_string();
                                    let host = state.ssh_host.trim().to_string();
                                    let user = state.ssh_user.trim().to_string();
                                    let port = state.ssh_port.parse().unwrap_or(22);
                                    let auth = match state.ssh_auth {
                                        crate::session::AuthMethod::Key { .. } => crate::session::AuthMethod::Key { private_key_path: state.ssh_private_key_path.clone() },
                                        other => other,
                                    };
                                    let config = crate::session::SessionProfile {
                                        id,
                                        name,
                                        folder: Some(if folder.is_empty() { "未分类".to_string() } else { folder }),
                                        color_tag: Some(state.color_tag),
                                        transport: crate::session::TransportConfig::Ssh(crate::session::SshConfig {
                                            host,
                                            port,
                                            user,
                                            auth,
                                            credential_id: None,
                                        }),
                                    };
                                    tokio::spawn(async move {
                                        let _ = sm.upsert_session(config).await;
                                    });
                                }
                            }

                            let toast_area = egui::Area::new(egui::Id::new("session_toasts_v2"))
                                .order(egui::Order::Foreground)
                                .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 16.0));
                            toast_area.show(ctx, |ui| {
                                ui.spacing_mut().item_spacing.y = 8.0;
                                for t in &app.session_toasts {
                                    let bg = if t.is_error { error.gamma_multiply(0.2) } else { accent.gamma_multiply(0.15) };
                                    let stroke = if t.is_error { error } else { accent };
                                    egui::Frame::new()
                                        .fill(bg)
                                        .stroke(egui::Stroke::new(1.0, stroke))
                                        .corner_radius(egui::CornerRadius::same(10))
                                        .inner_margin(egui::Margin::symmetric(12, 10))
                                        .show(ui, |ui| {
                                            ui.label(egui::RichText::new(&t.message).size(12.0).color(text_primary));
                                        });
                                }
                            });
                        });
                    });
                });
            });
        });
}

pub fn render_detailed_config_legacy(app: &mut RustSshApp, ctx: &egui::Context) {
    if app.editing_session.is_none() { return; }

    let content_rect = ctx.available_rect();
    egui::Area::new(egui::Id::new("modal_overlay"))
        .order(egui::Order::Foreground)
        .fixed_pos(content_rect.min)
        .show(ctx, |ui| {
            // Overlay (prototype: rgba(0,0,0,0.6))
            ui.painter().rect_filled(content_rect, egui::CornerRadius::ZERO, egui::Color32::from_rgba_premultiplied(0, 0, 0, 153));

            // Prototype: 750x500 container (within an 850x600 overlay)
            let panel_width = 750.0f32.min(content_rect.width() - 40.0);
            let panel_height = 500.0f32.min(content_rect.height() - 40.0);
            let panel_rect = egui::Rect::from_center_size(content_rect.center(), egui::vec2(panel_width, panel_height));
            
            let base_bg = egui::Color32::from_rgb(44, 44, 46); // #2C2C2E
            let sidebar_bg = egui::Color32::from_rgb(35, 35, 38); // #232326
            let field_bg = egui::Color32::from_rgb(28, 28, 30); // #1C1C1E
            let stroke_soft = egui::Color32::from_rgba_premultiplied(255, 255, 255, 26); // ~0.1
            let stroke_div = egui::Color32::from_rgba_premultiplied(255, 255, 255, 13); // ~0.05
            let text_muted = egui::Color32::from_rgb(160, 160, 165); // #A0A0A5
            let text_dim = egui::Color32::from_rgb(80, 80, 85); // #505055
            let accent = egui::Color32::from_rgb(0, 255, 128); // #00FF80

            let frame = egui::Frame::new()
                .fill(base_bg)
                .stroke(egui::Stroke::new(1.0, stroke_soft))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::ZERO);

            ui.scope_builder(egui::UiBuilder::new().max_rect(panel_rect), |ui| {
                frame.show(ui, |ui| {
                    // Layout: sidebar (200) + content
                    ui.horizontal(|ui| {
                        // ---- Sidebar ----
                        let sidebar_w = 200.0;
                        egui::Frame::new()
                            .fill(sidebar_bg)
                            .corner_radius(egui::CornerRadius { nw: 12, ne: 0, sw: 12, se: 0 })
                            .inner_margin(egui::Margin::ZERO)
                            .show(ui, |ui| {
                                ui.set_width(sidebar_w);
                                ui.set_height(panel_height);
                                let full = ui.available_rect_before_wrap();
                                ui.painter().vline(full.right(), full.y_range(), egui::Stroke::new(1.0, stroke_div));

                                ui.add_space(40.0);
                                ui.add_space(0.0);

                                if let Some(state) = &mut app.editing_session {
                                    // Keep SSH as default in minimal prototype
                                    state.protocol = ProtocolType::SSH;

                                    ui.add_space(0.0);
                                    ui.add_space(0.0);
                                    ui.add_space(0.0);

                                    ui.add_space(0.0);
                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);
                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    // Prototype positions: x=70, y=90; we use padding 20 inside sidebar
                                    ui.add_space(0.0);
                                    ui.add_space(0.0);
                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    ui.add_space(0.0);

                                    // Real sidebar content
                                    ui.add_space(0.0);
                                    ui.add_space(0.0);
                                    ui.add_space(0.0);
                                }
                            });

                        // ---- Main content ----
                        ui.vertical(|ui| {
                            ui.set_width(panel_width - sidebar_w);
                            ui.set_height(panel_height);

                            // Close button (top-right)
                            ui.allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), 44.0),
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add_space(14.0);
                                    let (rect, _) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::click());
                                    let resp = ui.interact(rect, ui.id().with("close_session_editor"), egui::Sense::click());
                                    ui.painter().line_segment([rect.left_top(), rect.right_bottom()], egui::Stroke::new(1.5, text_muted));
                                    ui.painter().line_segment([rect.right_top(), rect.left_bottom()], egui::Stroke::new(1.5, text_muted));
                                    if resp.clicked() {
                                        app.editing_session = None;
                                    }
                                },
                            );

                            // Tabs bar (prototype: 通用 / 高级设置)
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.add_space(20.0);

                                let tab_labels = ["通用", "高级设置"];
                                for (idx, label) in tab_labels.iter().enumerate() {
                                    let is_active = app.session_editor_tab == idx;
                                    let text = egui::RichText::new(*label)
                                        .size(12.0)
                                        .color(if is_active { egui::Color32::WHITE } else { text_muted })
                                        .strong();
                                    let resp = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
                                    if is_active {
                                        // underline
                                        let mut r = resp.rect;
                                        r.min.y = r.max.y + 6.0;
                                        r.max.y = r.min.y + 2.0;
                                        ui.painter().rect_filled(r, egui::CornerRadius::ZERO, accent);
                                    }
                                    if resp.clicked() {
                                        app.session_editor_tab = idx;
                                    }
                                    ui.add_space(18.0);
                                }
                            });

                            ui.add_space(18.0);

                            // Content area + footer
                            let footer_h = 50.0;
                            let content_h = ui.available_height() - footer_h;

                            // ---- Scroll-less content region ----
                            ui.allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), content_h),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.add_space(10.0);
                                    ui.add_space(0.0);
                                    ui.add_space(0.0);
                                    ui.add_space(0.0);

                                    if let Some(state) = &mut app.editing_session {
                                        // Sidebar content (we render here to avoid duplicated borrowing)
                                        // Note: the sidebar was drawn above; we overlay its widgets using absolute positioning
                                        // by using a temporary Area inside the panel.
                                        // (keeps implementation simple while matching prototype layout)
                                        let sidebar_area = egui::Area::new(egui::Id::new("session_editor_sidebar_area"))
                                            .order(egui::Order::Foreground)
                                            .fixed_pos(panel_rect.min + egui::vec2(20.0, 40.0));
                                        sidebar_area.show(ctx, |ui| {
                                            ui.set_width(sidebar_w - 40.0);
                                            ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0);

                                            ui.label(egui::RichText::new("会话名称").size(11.0).color(text_muted));
                                            ui.add_sized(
                                                [160.0, 32.0],
                                                egui::TextEdit::singleline(&mut state.name)
                                                    .font(egui::TextStyle::Monospace)
                                                    .desired_width(160.0)
                                                    .margin(egui::vec2(10.0, 8.0))
                                                    .background_color(field_bg),
                                            );

                                            ui.add_space(12.0);
                                            ui.label(egui::RichText::new("分组").size(11.0).color(text_muted));
                                            ui.add_sized(
                                                [160.0, 32.0],
                                                egui::TextEdit::singleline(&mut state.folder)
                                                    .desired_width(160.0)
                                                    .margin(egui::vec2(10.0, 8.0))
                                                    .background_color(field_bg),
                                            );
                                        });

                                        match app.session_editor_tab {
                                            0 => {
                                                // ---- 通用 ----
                                                ui.add_space(6.0);
                                                ui.horizontal(|ui| {
                                                    ui.add_space(20.0);
                                                    ui.spacing_mut().item_spacing = egui::vec2(15.0, 8.0);

                                                    // 连接策略
                                                    ui.vertical(|ui| {
                                                        ui.label(egui::RichText::new("连接策略").size(11.0).color(text_muted));
                                                        let mut strategy = 0u8;
                                                        egui::ComboBox::from_id_salt("conn_strategy")
                                                            .selected_text(egui::RichText::new("直连模式").size(10.0).color(egui::Color32::WHITE))
                                                            .width(120.0)
                                                            .show_ui(ui, |ui| {
                                                                ui.selectable_value(&mut strategy, 0, "直连模式");
                                                            });
                                                    });

                                                    // 主机 / IP
                                                    ui.vertical(|ui| {
                                                        ui.label(egui::RichText::new("主机 / IP").size(11.0).color(text_muted));
                                                        let te = egui::TextEdit::singleline(&mut state.ssh_host)
                                                            .desired_width(250.0)
                                                            .margin(egui::vec2(10.0, 8.0))
                                                            .background_color(field_bg);
                                                        let resp = ui.add_sized([250.0, 32.0], te);
                                                        // subtle accent stroke like prototype focus
                                                        if resp.has_focus() {
                                                            ui.painter().rect_stroke(resp.rect, egui::CornerRadius::same(6), egui::Stroke::new(1.0, accent.gamma_multiply(0.3)), egui::StrokeKind::Inside);
                                                        }
                                                    });

                                                    // 端口
                                                    ui.vertical(|ui| {
                                                        ui.label(egui::RichText::new("端口").size(11.0).color(text_muted));
                                                        ui.add_sized(
                                                            [85.0, 32.0],
                                                            egui::TextEdit::singleline(&mut state.ssh_port)
                                                                .desired_width(85.0)
                                                                .margin(egui::vec2(10.0, 8.0))
                                                                .background_color(field_bg),
                                                        );
                                                    });
                                                });

                                                ui.add_space(18.0);
                                                ui.horizontal(|ui| {
                                                    ui.add_space(20.0);
                                                    ui.vertical(|ui| {
                                                        ui.label(egui::RichText::new("跳板机节点 (若适用)").size(11.0).color(text_muted));
                                                        let (rect, _) = ui.allocate_exact_size(egui::vec2(485.0, 32.0), egui::Sense::hover());
                                                        ui.painter().rect_filled(rect, egui::CornerRadius::same(6), field_bg.gamma_multiply(0.4));
                                                        ui.painter().text(
                                                            rect.left_center() + egui::vec2(10.0, 0.0),
                                                            egui::Align2::LEFT_CENTER,
                                                            "无",
                                                            egui::FontId::proportional(11.0),
                                                            text_dim,
                                                        );
                                                    });
                                                });

                                                ui.add_space(18.0);
                                                ui.horizontal(|ui| {
                                                    ui.add_space(20.0);
                                                    ui.vertical(|ui| {
                                                        ui.label(egui::RichText::new("用户名").size(11.0).color(text_muted));
                                                        ui.add_sized(
                                                            [485.0, 32.0],
                                                            egui::TextEdit::singleline(&mut state.ssh_user)
                                                                .desired_width(485.0)
                                                                .margin(egui::vec2(10.0, 8.0))
                                                                .background_color(field_bg),
                                                        );
                                                    });
                                                });

                                                ui.add_space(18.0);
                                                ui.horizontal(|ui| {
                                                    ui.add_space(20.0);
                                                    let w = 485.0;
                                                    let y = ui.cursor().min.y + 10.0;
                                                    ui.painter().line_segment(
                                                        [egui::pos2(ui.cursor().min.x + 20.0, y), egui::pos2(ui.cursor().min.x + 20.0 + w, y)],
                                                        egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 20)),
                                                    );
                                                });

                                                ui.add_space(18.0);
                                                ui.horizontal(|ui| {
                                                    ui.add_space(20.0);
                                                    ui.label(egui::RichText::new("认证凭据").size(12.0).strong().color(accent));
                                                });

                                                ui.add_space(12.0);
                                                ui.horizontal(|ui| {
                                                    ui.add_space(20.0);
                                                    let options: [(&str, crate::session::AuthMethod); 3] = [
                                                        ("密码", crate::session::AuthMethod::Password),
                                                        ("私钥", crate::session::AuthMethod::Key { private_key_path: String::new() }),
                                                        ("交互", crate::session::AuthMethod::Interactive),
                                                    ];

                                                    let current_kind = match &state.ssh_auth {
                                                        crate::session::AuthMethod::Password => 0,
                                                        crate::session::AuthMethod::Key { .. } => 1,
                                                        crate::session::AuthMethod::Interactive => 2,
                                                        crate::session::AuthMethod::Agent => 2,
                                                    };

                                                    for (idx, (label, method)) in options.iter().enumerate() {
                                                        let is_sel = idx == current_kind;
                                                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(74.0, 18.0), egui::Sense::click());
                                                        let c = rect.left_center() + egui::vec2(10.0, 0.0);
                                                        let r = 5.0;
                                                        ui.painter().circle_stroke(c, r, egui::Stroke::new(if is_sel { 2.0 } else { 1.0 }, if is_sel { accent } else { text_muted }));
                                                        if is_sel {
                                                            ui.painter().circle_filled(c, 2.5, accent);
                                                        }
                                                        ui.painter().text(
                                                            c + egui::vec2(15.0, 0.0),
                                                            egui::Align2::LEFT_CENTER,
                                                            *label,
                                                            egui::FontId::proportional(11.0),
                                                            if is_sel { egui::Color32::WHITE } else { text_muted },
                                                        );
                                                        if resp.clicked() {
                                                            state.ssh_auth = method.clone();
                                                        }
                                                    }
                                                });
                                            }
                                            _ => {
                                                // ---- 高级设置（最小原型仅展示占位）----
                                                ui.add_space(6.0);
                                                ui.horizontal(|ui| {
                                                    ui.add_space(20.0);
                                                    ui.label(egui::RichText::new("高级设置（WIP）").size(12.0).color(text_muted));
                                                });
                                            }
                                        }
                                    }
                                }
                            );

                            // ---- Footer ----
                            let footer_rect = ui.allocate_exact_size(egui::vec2(ui.available_width(), footer_h), egui::Sense::hover()).0;
                            ui.painter().hline(footer_rect.x_range(), footer_rect.top(), egui::Stroke::new(1.0, stroke_div));

                            let mut cancel_clicked = false;
                            let mut test_clicked = false;
                            let mut save_clicked = false;
                            ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
                                ui.horizontal(|ui| {
                                    ui.add_space(20.0);

                                    // Left actions: 取消 / 测试连接
                                    let cancel = ui.add(egui::Label::new(egui::RichText::new("取消").size(12.0).color(text_muted)).sense(egui::Sense::click()));
                                    ui.add_space(16.0);
                                    let test = ui.add(egui::Label::new(egui::RichText::new("测试连接").size(12.0).color(egui::Color32::WHITE)).sense(egui::Sense::click()));
                                    cancel_clicked = cancel.clicked();
                                    test_clicked = test.clicked();

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.add_space(20.0);
                                        let save_rect = ui.allocate_exact_size(egui::vec2(100.0, 32.0), egui::Sense::click()).0;
                                        let save_resp = ui.interact(save_rect, ui.id().with("session_save"), egui::Sense::click());
                                        ui.painter().rect_filled(save_rect, egui::CornerRadius::same(6), accent);
                                        ui.painter().text(save_rect.center(), egui::Align2::CENTER_CENTER, "保存", egui::FontId::proportional(12.0), egui::Color32::BLACK);
                                        save_clicked = save_resp.clicked();
                                    });
                                });
                            });

                            // Apply footer actions after UI widgets are built (and borrows ended)
                            if cancel_clicked {
                                app.editing_session = None;
                            } else if test_clicked {
                                if let Some(state) = app.editing_session.as_ref() {
                                    if state.protocol == ProtocolType::SSH {
                                        let host = state.ssh_host.trim().to_string();
                                        let user = state.ssh_user.trim().to_string();
                                        let port = state.ssh_port.parse::<u16>().unwrap_or(22);
                                        if !host.is_empty() && !user.is_empty() {
                                            app.connection_prompt = Some(crate::app::ConnectionPromptState {
                                                host,
                                                port,
                                                user,
                                                password: secrecy::SecretString::from("".to_string()),
                                                is_connecting: false,
                                                error: None,
                                                success_session: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
                                                error_msg: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
                                            });
                                        }
                                    }
                                }
                            } else if save_clicked {
                                if let Some(state) = app.editing_session.take() {
                                    let mut sm = app.session_manager.clone();
                                    let id = state.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                                    let name = if state.name.trim().is_empty() {
                                        if state.protocol == ProtocolType::SSH {
                                            state.ssh_host.trim().to_string()
                                        } else {
                                            "session".to_string()
                                        }
                                    } else {
                                        state.name.trim().to_string()
                                    };
                                    let folder = state.folder.trim().to_string();
                                    let color_tag = state.color_tag;
                                    let host = state.ssh_host.trim().to_string();
                                    let user = state.ssh_user.trim().to_string();
                                    let port = state.ssh_port.parse().unwrap_or(22);
                                    let auth = state.ssh_auth.clone();
                                    let config = crate::session::SessionProfile {
                                        id,
                                        name,
                                        folder: Some(if folder.is_empty() { "未分类".to_string() } else { folder }),
                                        color_tag: Some(color_tag),
                                        transport: crate::session::TransportConfig::Ssh(crate::session::SshConfig {
                                            host,
                                            port,
                                            user,
                                            auth,
                                            credential_id: None,
                                        }),
                                    };
                                    tokio::spawn(async move {
                                        let _ = sm.upsert_session(config).await;
                                    });
                                }
                            }
                        });
                    });
                });
            });
        });
}

pub fn render_connection_modal(app: &mut RustSshApp, ctx: &egui::Context, theme: &Theme) {
    if app.connection_prompt.is_none() { 
        return; 
    }

    let content_rect = ctx.available_rect();
    egui::Area::new(egui::Id::new("connection_modal_overlay"))
        .order(egui::Order::Foreground)
        .fixed_pos(content_rect.min)
        .show(ctx, |ui| {
            ui.painter().rect_filled(content_rect, egui::CornerRadius::ZERO, egui::Color32::from_black_alpha(200));
            
            let modal_width = 400.0;
            let modal_height = 240.0;
            let modal_rect = egui::Rect::from_center_size(content_rect.center(), egui::vec2(modal_width, modal_height));
            
            let frame = egui::Frame::new()
                .fill(egui::Color32::from_rgb(30, 30, 35))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 65)))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::same(24));

            ui.scope_builder(egui::UiBuilder::new().max_rect(modal_rect), |ui| {
                frame.show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("SSH 身份验证");
                        ui.add_space(8.0);
                        
                        // 使用 take 取得所有权，以便应对各类借用冲突
                        let mut prompt = app.connection_prompt.take().unwrap();
                        let mut should_keep = true;
                        
                        // --- 状态检查阶段 ---
                        // 检查是否有异步任务回传的成功 Session
                        if let Ok(mut lock) = prompt.success_session.try_lock() {
                            if let Some(sess) = lock.take() {
                                log::debug!("UI 线程正在接收 Session 并应用到活跃状态");
                                app.active_session = Some(sess);
                                app.tabs.push(format!("{}@{}", prompt.user, prompt.host));
                                app.active_tab_index = app.tabs.len() - 1;
                                should_keep = false;
                                ctx.request_repaint();
                            }
                        }

                        // 检查是否有异步任务回传的错误信息
                        if let Ok(mut lock) = prompt.error_msg.try_lock() {
                            if let Some(err) = lock.take() {
                                prompt.is_connecting = false;
                                prompt.error = Some(err);
                                ctx.request_repaint();
                            }
                        }

                        if !should_keep { return; }

                        ui.label(format!("正在连接到 {}@{}:{}", prompt.user, prompt.host, prompt.port));
                        ui.add_space(12.0);

                        ui.label("请输入密码:");
                        use secrecy::ExposeSecret;
                        let mut pwd_raw = prompt.password.expose_secret().to_string();
                        let text_edit = egui::TextEdit::singleline(&mut pwd_raw).password(true).desired_width(f32::INFINITY);
                        if ui.add_enabled(!prompt.is_connecting, text_edit).changed() {
                            prompt.password = secrecy::SecretString::from(pwd_raw);
                        }

                        if let Some(err) = &prompt.error {
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(format!("❌ {}", err)).color(egui::Color32::RED).size(11.0));
                        }

                        ui.add_space(20.0);
                        ui.separator();
                        ui.add_space(16.0);

                        ui.horizontal(|ui| {
                            if prompt.is_connecting {
                                ui.add(egui::Spinner::new());
                                ui.label(" 正在认证...");
                            } else {
                                if ui.add(egui::Button::new("连接").fill(theme.accent_base)).clicked() {
                                    prompt.is_connecting = true;
                                    prompt.error = None;
                                    
                                    let host = prompt.host.clone();
                                    let port = prompt.port;
                                    let user = prompt.user.clone();
                                    let password = prompt.password.expose_secret().to_string();
                                    let success_session = prompt.success_session.clone();
                                    let error_msg = prompt.error_msg.clone();
                                    let ctx_clone = ctx.clone();
                                    
                                    tokio::spawn(async move {
                                        let mut session = crate::backend::ssh_session::SshSession::new();
                                        match session.connect(&host, port, &user, &password).await {
                                            Ok(_) => {
                                                log::info!("SSH 认证成功: {}@{}", user, host);
                                                let mut lock = success_session.lock().await;
                                                *lock = Some(Box::new(session));
                                            }
                                            Err(e) => {
                                                log::warn!("SSH 认证失败: {}", e);
                                                let mut lock = error_msg.lock().await;
                                                *lock = Some(e.to_string());
                                            }
                                        }
                                        ctx_clone.request_repaint();
                                    });
                                }
                                if ui.button("取消").clicked() {
                                    should_keep = false;
                                }
                            }
                        });
                        
                        // 生命周期收束：将状态放回
                        if should_keep {
                            app.connection_prompt = Some(prompt);
                        }
                    });
                });
            });
        });
}
