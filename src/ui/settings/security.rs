use eframe::egui;
use crate::app::RustSshApp;
use crate::ui::theme::Theme;
use crate::ui::settings::render_switch;

pub fn render(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme, tab_idx: usize) {
    match tab_idx {
        0 => render_vault(app, ui, theme),
        1 => render_hosts(app, ui, theme),
        _ => {}
    }
}

fn render_vault(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme) {
    ui.add_space(8.0);
    
    // --- 1. 保险箱安全 (Vault) ---
    ui.label(egui::RichText::new("保险箱安全 (Vault)").size(18.0).strong().color(egui::Color32::WHITE));
    ui.add_space(20.0);

    // 自动锁定超时
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("自动锁定超时").size(14.0).color(egui::Color32::WHITE));
            ui.label(egui::RichText::new("长时间未操作后自动锁定。选择“永不”则不自动锁定。").size(12.0).color(theme.text_secondary));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut timeout = app.settings.security.idle_timeout_mins;
            let text = if timeout == 0 { "永不".to_string() } else { format!("{} 分钟", timeout) };
            
            // 精确对齐原型图 ComboBox 样式
            ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgb(40, 40, 45); // #28282D
            ui.visuals_mut().widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(40, 40, 45);
            ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 58, 63)); // #3A3A3F

            if egui::ComboBox::from_id_salt("timeout_v2")
                .selected_text(egui::RichText::new(text).size(13.0).color(egui::Color32::WHITE))
                .width(224.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut timeout, 1, "1 分钟");
                    ui.selectable_value(&mut timeout, 5, "5 分钟");
                    ui.selectable_value(&mut timeout, 10, "10 分钟");
                    ui.selectable_value(&mut timeout, 30, "30 分钟");
                    ui.selectable_value(&mut timeout, 0, "永不");
                }).response.changed() {
                app.settings.security.idle_timeout_mins = timeout;
                let _ = app.settings.save();
            }
        });
    });
    ui.add_space(24.0);

    // 切入后台时锁定
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("切入后台时锁定").size(14.0).color(egui::Color32::WHITE));
            ui.label(egui::RichText::new("当窗口最小化或切入后台时，立即锁定保险箱。").size(12.0).color(theme.text_secondary));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if render_switch(ui, &mut app.settings.security.lock_on_sleep).changed() {
                let _ = app.settings.save();
            }
        });
    });
    ui.add_space(20.0);

    // 分割线 #3A3A3F
    ui.painter().hline(ui.available_rect_before_wrap().x_range(), ui.next_widget_position().y, egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 58, 63)));
    ui.add_space(20.0);

    // 主密码管理 (Master Password)
    let is_vault_initialized = app.settings.security.vault.is_some();
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            let label = if is_vault_initialized { "修改主密码" } else { "配置主密码 (初始化)" };
            ui.label(egui::RichText::new(label).size(14.0).color(egui::Color32::WHITE));
            ui.horizontal(|ui| {
               let desc = if is_vault_initialized { 
                   "更改解密保险箱根密钥。强度:" 
               } else { 
                   "保险箱尚未加密。设置主密码以启用凭据保护。" 
               };
               ui.label(egui::RichText::new(desc).size(12.0).color(theme.text_secondary));
               if is_vault_initialized {
                   render_password_strength_bar(ui, 4); 
               }
            });
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgb(58, 58, 63); 
            let button_text = if is_vault_initialized { "前往修改" } else { "立即设置" };
            let button_size = egui::vec2(100.0, 32.0);
            if ui.add_sized(button_size, egui::Button::new(egui::RichText::new(button_text).size(13.0).color(egui::Color32::WHITE))).clicked() {
                app.password_change = Some(crate::app::PasswordChangeState::default());
            }
        });
    });
    ui.add_space(20.0);

    // 分割线 #3A3A3F
    ui.painter().hline(ui.available_rect_before_wrap().x_range(), ui.next_widget_position().y, egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 58, 63)));
    ui.add_space(24.0);

    // --- 2. 系统生物识别 (Biometrics) ---
    ui.label(egui::RichText::new("系统生物识别 (Biometrics)").size(18.0).strong().color(egui::Color32::WHITE));
    ui.add_space(16.0);

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("使用系统指纹或面容认证").size(14.0).color(egui::Color32::WHITE));
            ui.label(egui::RichText::new("支持 Touch ID, Windows Hello 或系统密钥环。").size(12.0).color(theme.text_secondary));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if render_switch(ui, &mut app.settings.security.use_biometrics).changed() {
                let _ = app.settings.save();
            }
        });
    });
    
    ui.add_space(20.0);

    // 正在认证的状态反馈
    if app.settings.security.use_biometrics {
        egui::Frame::default()
            .fill(theme.accent_base.gamma_multiply(0.1))
            .stroke(egui::Stroke::new(1.0, theme.accent_base))
            .inner_margin(12.0)
            .corner_radius(egui::CornerRadius::same(8))
            .show(ui, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width()/2.0 - 64.0);
                        ui.label(egui::RichText::new("正在进行生物识别确认...").size(13.0).color(theme.accent_base));
                    });
                });
            });
    }

    ui.add_space(24.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("󰒘 内存零化处理保护 (Privacy Zeroize) 已就绪").size(11.0).color(theme.text_secondary.gamma_multiply(0.5)));
    });
}

fn render_hosts(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new("主机指纹管理 (Known Hosts)").size(18.0).strong().color(egui::Color32::WHITE));
    ui.add_space(24.0);
    
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("连接验证策略").size(14.0).color(egui::Color32::WHITE));
            ui.label(egui::RichText::new("指纹不匹配或不存在时的处理方式").size(12.0).color(theme.text_secondary));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut policy = app.settings.security.host_key_policy;
            let policy_text = match policy {
                crate::settings::HostKeyPolicy::Strict => "严格 (Strict)",
                crate::settings::HostKeyPolicy::Ask => "询问 (Ask)",
                crate::settings::HostKeyPolicy::AcceptNew => "自动接受 (Accept New)",
            };
            
            ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgb(40, 40, 45);

            if egui::ComboBox::from_id_salt("host_policy_combo")
                .selected_text(egui::RichText::new(policy_text).size(13.0))
                .width(160.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut policy, crate::settings::HostKeyPolicy::Strict, "严格 (Strict)");
                    ui.selectable_value(&mut policy, crate::settings::HostKeyPolicy::Ask, "询问 (Ask)");
                    ui.selectable_value(&mut policy, crate::settings::HostKeyPolicy::AcceptNew, "自动接受 (Accept New)");
                }).response.changed() {
                app.settings.security.host_key_policy = policy;
                let _ = app.settings.save();
            }
        });
    });
    
    ui.add_space(24.0);
    
    ui.label(egui::RichText::new("已受信任的主机列表").size(13.0).strong());
    ui.add_space(12.0);
    
    let table_bg = egui::Color32::from_rgb(30,30,35);
    egui::Frame::default()
        .fill(table_bg)
        .inner_margin(0.0)
        .corner_radius(egui::CornerRadius::same(8))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 58, 63)))
        .show(ui, |ui| {
            let mut header_rect = ui.available_rect_before_wrap();
            header_rect.set_height(32.0);
            ui.scope_builder(egui::UiBuilder::new().max_rect(header_rect), |ui| {
                ui.painter().rect_filled(ui.available_rect_before_wrap(), egui::CornerRadius::ZERO, egui::Color32::from_rgb(40,40,45));
                ui.horizontal_centered(|ui| {
                    ui.add_space(12.0);
                    ui.set_width(ui.available_width());
                    egui::Grid::new("hosts_header").num_columns(4).spacing([12.0, 0.0]).show(ui, |ui| {
                        ui.add_sized([150.0, 20.0], egui::Label::new(egui::RichText::new("主机地址").size(11.0).color(theme.text_secondary)));
                        ui.add_sized([80.0, 20.0], egui::Label::new(egui::RichText::new("算法").size(11.0).color(theme.text_secondary)));
                        ui.add_sized([300.0, 20.0], egui::Label::new(egui::RichText::new("SHA256 指纹信息").size(11.0).color(theme.text_secondary)));
                        ui.label(""); 
                        ui.end_row();
                    });
                });
            });
            ui.add_space(32.0); 
            
            let mock_hosts = [
                ("10.0.0.1:22", "ED25519", "SHA256:vPxL2...Q8"),
                ("github.com", "RSA", "SHA256:nThbg...kI"),
            ];
            
            ui.vertical(|ui| {
                for (host, algo, thumb) in mock_hosts {
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        egui::Grid::new(host).num_columns(4).spacing([12.0, 16.0]).show(ui, |ui| {
                            ui.add_sized([150.0, 32.0], egui::Label::new(egui::RichText::new(host).size(13.0)));
                            ui.add_sized([80.0, 32.0], egui::Label::new(egui::RichText::new(algo).size(12.0).color(theme.text_secondary)));
                            ui.add_sized([300.0, 32.0], egui::Label::new(egui::RichText::new(thumb).size(12.0).monospace()));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("🗑").clicked() {
                                }
                            });
                            ui.end_row();
                        });
                    });
                    ui.painter().hline(ui.available_rect_before_wrap().x_range(), ui.next_widget_position().y, egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 58, 63)));
                }
            });
        });
}

fn render_password_strength_bar(ui: &mut egui::Ui, strength: usize) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for i in 0..5 {
            let color = if i < strength {
                if strength <= 1 { egui::Color32::from_rgb(255, 59, 48) } 
                else if strength <= 3 { egui::Color32::from_rgb(255, 204, 0) } 
                else { egui::Color32::from_rgb(0, 255, 128) } 
            } else {
                egui::Color32::from_rgba_premultiplied(255, 255, 255, 15) 
            };
            
            let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 6.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, egui::CornerRadius::same(2), color);
        }
    });
}
