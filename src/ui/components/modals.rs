use eframe::egui;
use crate::app::{RustSshApp, DetailedConfigState};
use crate::ui::theme::Theme;
use crate::session::ProtocolType;

pub fn render_quick_connect(app: &mut RustSshApp, ctx: &egui::Context, theme: &Theme, header_height: f32) {
    if !app.show_quick_connect { return; }

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
                                .hint_text("输入 root@host:port 或搜索会话...")
                                .desired_width(f32::INFINITY)
                                .lock_focus(true)
                        );
                        resp.request_focus();
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
                                crate::session::TransportConfig::Ssh(ssh) => ssh.host.to_lowercase().contains(&search_lower),
                                crate::session::TransportConfig::Telnet(telnet) => telnet.host.to_lowercase().contains(&search_lower),
                                crate::session::TransportConfig::Serial(serial) => serial.port.to_lowercase().contains(&search_lower),
                            };
                            name_match || transport_match
                        })
                        .collect();

                    if !filtered_sessions.is_empty() {
                        ui.label(egui::RichText::new("  已保存会话").weak().size(12.0));
                        for s in &filtered_sessions {
                            let (icon, addr) = match &s.transport {
                                crate::session::TransportConfig::Ssh(ssh) => ("🖥", ssh.host.clone()),
                                crate::session::TransportConfig::Telnet(telnet) => ("🌐", telnet.host.clone()),
                                crate::session::TransportConfig::Serial(serial) => ("🔌", serial.port.clone()),
                            };
                            let display = format!("  {} {} ({})", icon, s.name, addr);
                            if ui.add(egui::Button::new(display).selected(false)).clicked() {
                                let tab_name = match &s.transport {
                                    crate::session::TransportConfig::Ssh(ssh) => format!("{}@{}", ssh.user, ssh.host),
                                    crate::session::TransportConfig::Telnet(telnet) => telnet.host.clone(),
                                    crate::session::TransportConfig::Serial(serial) => serial.port.clone(),
                                };
                                app.tabs.push(tab_name);
                                app.active_tab_index = app.tabs.len() - 1;
                                app.show_quick_connect = false;
                            }
                        }
                        ui.add_space(4.0);
                    }
                    
                    ui.label(egui::RichText::new("  建议 / 最近连接").weak().size(12.0));
                    for (icon, name, addr) in [
                        ("🚀", "Edge Gateway", "edge-01.local"),
                        ("📁", "Database Backup", "db.internal.lan"),
                    ] {
                        if name.to_lowercase().contains(&search_lower) || addr.to_lowercase().contains(&search_lower) {
                            if ui.add(egui::Button::new(format!("  {} {} ({})", icon, name, addr)).selected(false)).clicked() {
                                app.tabs.push(addr.to_string());
                                app.active_tab_index = app.tabs.len() - 1;
                                app.show_quick_connect = false;
                            }
                        }
                    }
                    
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
                                });
                                app.show_quick_connect = false;
                            }
                        }
                    }

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                    
                    if ui.add(egui::Button::new("➕ 新建完整会话配置...").selected(false)).clicked() {
                        app.editing_session = Some(DetailedConfigState::default());
                        app.show_quick_connect = false;
                    }
                    ui.add_space(8.0);
                });
            });
    });

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.show_quick_connect = false;
    }
}

pub fn render_detailed_config(app: &mut RustSshApp, ctx: &egui::Context) {
    if app.editing_session.is_none() { return; }

    let content_rect = ctx.available_rect();
    egui::Area::new(egui::Id::new("modal_overlay"))
        .order(egui::Order::Foreground)
        .fixed_pos(content_rect.min)
        .show(ctx, |ui| {
            ui.painter().rect_filled(content_rect, egui::CornerRadius::ZERO, egui::Color32::from_black_alpha(200));
            let panel_width = 600.0;
            let panel_height = 500.0;
            let panel_rect = egui::Rect::from_center_size(content_rect.center(), egui::vec2(panel_width, panel_height));
            
            let frame = egui::Frame::new()
                .fill(egui::Color32::from_rgb(30, 30, 35))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 65)))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::same(24));

            ui.scope_builder(egui::UiBuilder::new().max_rect(panel_rect), |ui| {
                frame.show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("新建/编辑会话配置");
                        ui.add_space(16.0);

                        if let Some(state) = &mut app.editing_session {
                            ui.horizontal(|ui| {
                                ui.label("会话名称:");
                                ui.text_edit_singleline(&mut state.name);
                            });
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label("连接协议:");
                                ui.selectable_value(&mut state.protocol, ProtocolType::SSH, "SSH");
                                ui.selectable_value(&mut state.protocol, ProtocolType::Serial, "Serial");
                                ui.selectable_value(&mut state.protocol, ProtocolType::Telnet, "Telnet");
                                ui.add_space(24.0);
                                ui.label("标签颜色:");
                                let mut color = egui::Color32::from_rgb(state.color_tag[0], state.color_tag[1], state.color_tag[2]);
                                if ui.color_edit_button_srgba(&mut color).changed() {
                                    state.color_tag = [color.r(), color.g(), color.b()];
                                }
                            });
                            ui.separator();
                            ui.add_space(12.0);

                            match state.protocol {
                                ProtocolType::SSH => {
                                    ui.horizontal(|ui| {
                                        ui.label("主机地址:");
                                        ui.text_edit_singleline(&mut state.ssh_host);
                                        ui.add_space(12.0);
                                        ui.label("端口:");
                                        ui.text_edit_singleline(&mut state.ssh_port);
                                    });
                                    ui.add_space(8.0);
                                    ui.horizontal(|ui| {
                                        ui.label("用户名  :");
                                        ui.text_edit_singleline(&mut state.ssh_user);
                                    });
                                }
                                ProtocolType::Serial => {
                                    ui.horizontal(|ui| {
                                        ui.label("串口设备:");
                                        ui.text_edit_singleline(&mut state.serial_port);
                                        ui.add_space(12.0);
                                        ui.label("波特率:");
                                        ui.text_edit_singleline(&mut state.serial_baud);
                                    });
                                }
                                ProtocolType::Telnet => {
                                    ui.horizontal(|ui| {
                                        ui.label("主机地址:");
                                        ui.text_edit_singleline(&mut state.telnet_host);
                                        ui.label("端口:");
                                        ui.text_edit_singleline(&mut state.telnet_port);
                                    });
                                }
                            }

                            ui.add_space(32.0);
                            ui.separator();
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("保存并关闭").clicked() {
                                    let final_state = app.editing_session.take();
                                    if let Some(state) = final_state {
                                        let mut sm = app.session_manager.clone();
                                        let config = crate::session::SessionProfile {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            name: if state.name.is_empty() { 
                                                match state.protocol {
                                                    ProtocolType::SSH => state.ssh_host.clone(),
                                                    ProtocolType::Serial => state.serial_port.clone(),
                                                    ProtocolType::Telnet => state.telnet_host.clone(),
                                                }
                                            } else { state.name.clone() },
                                            folder: Some(state.folder.clone()),
                                            color_tag: Some(state.color_tag),
                                            transport: match state.protocol {
                                                ProtocolType::SSH => crate::session::TransportConfig::Ssh(crate::session::SshConfig {
                                                    host: state.ssh_host.clone(),
                                                    port: state.ssh_port.parse().unwrap_or(22),
                                                    user: state.ssh_user.clone(),
                                                    auth: state.ssh_auth.clone(),
                                                    credential_id: None,
                                                }),
                                                ProtocolType::Serial => crate::session::TransportConfig::Serial(crate::session::SerialConfig {
                                                    port: state.serial_port.clone(),
                                                    baud_rate: state.serial_baud.parse().unwrap_or(115200),
                                                    data_bits: 8,
                                                    stop_bits: 1,
                                                    parity: "N".to_string(),
                                                    flow_control: "None".to_string(),
                                                }),
                                                ProtocolType::Telnet => crate::session::TransportConfig::Telnet(crate::session::TelnetConfig {
                                                    host: state.telnet_host.clone(),
                                                    port: state.telnet_port.parse().unwrap_or(23),
                                                    encoding: "UTF-8".to_string(),
                                                }),
                                            },
                                        };
                                        tokio::spawn(async move {
                                            let _ = sm.add_session(config).await;
                                        });
                                    }
                                }
                                if ui.button("取消").clicked() {
                                    app.editing_session = None;
                                }
                            });
                        }
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
                                println!("📦 UI 线程正在接收 Session 并应用到活跃状态");
                                app.active_session = Some(sess);
                                app.tabs.push(format!("{}@{}", prompt.user, prompt.host));
                                app.active_tab_index = app.tabs.len() - 1;
                                should_keep = false;
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
                                    let ctx_clone = ctx.clone();
                                    
                                    tokio::spawn(async move {
                                        let mut session = crate::backend::ssh_session::SshSession::new();
                                        match session.connect(&host, port, &user, &password).await {
                                            Ok(_) => {
                                                println!("✅ SSH 认证成功: {}@{}", user, host);
                                                let mut lock = success_session.lock().await;
                                                *lock = Some(Box::new(session));
                                            }
                                            Err(e) => {
                                                println!("❌ SSH 认证失败: {}", e);
                                                // 此处应回传错误信息给 UI
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
