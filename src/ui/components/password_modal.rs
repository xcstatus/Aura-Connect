use eframe::egui;
use secrecy::ExposeSecret;
use crate::app::RustSshApp;
use crate::ui::theme::Theme;
use crate::vault_utils::check_password_strength;

pub fn render_password_change_modal(app: &mut RustSshApp, ctx: &egui::Context, theme: &Theme) {
    if app.password_change.is_none() { return; }

    let content_rect = ctx.available_rect();
    
    egui::Area::new(egui::Id::new("password_modal_overlay"))
        .order(egui::Order::Foreground)
        .fixed_pos(content_rect.min)
        .show(ctx, |ui| {
            // 背景遮罩
            ui.painter().rect_filled(content_rect, egui::CornerRadius::ZERO, egui::Color32::from_black_alpha(200));
            
            let modal_width = 400.0;
            let modal_height = 420.0;
            let modal_rect = egui::Rect::from_center_size(content_rect.center(), egui::vec2(modal_width, modal_height));
            
            let frame = egui::Frame::new()
                .fill(egui::Color32::from_rgb(30, 30, 35))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 65)))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::same(24));

            ui.scope_builder(egui::UiBuilder::new().max_rect(modal_rect), |ui| {
                frame.show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading(egui::RichText::new("修改主密码").color(egui::Color32::WHITE));
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("更改用于加密所有存储凭据根密钥。请务必牢记新密码，一旦丢失将无法找回。").size(12.0).color(theme.text_secondary));
                        ui.add_space(20.0);

                        let mut close_modal = false;
                        if let Some(state) = &mut app.password_change {
                            let is_busy = state.is_busy;
                            
                            // 预提取数据并转换为拥有权的 String
                            let mut old_pwd = state.old_password.expose_secret().to_string();
                            let mut new_pwd = state.new_password.expose_secret().to_string();
                            let mut confirm_pwd = state.confirm_password.expose_secret().to_string();
                            let strength = check_password_strength(&new_pwd);

                            // 1. 原密码
                            ui.label(egui::RichText::new("当前主密码").size(13.0).strong());
                            ui.add_space(4.0);
                            if ui.add_enabled(!is_busy, egui::TextEdit::singleline(&mut old_pwd).password(true).desired_width(f32::INFINITY)).changed() {
                                state.old_password = secrecy::SecretString::from(old_pwd.clone());
                            }
                            
                            ui.add_space(16.0);

                            // 2. 新密码
                            ui.label(egui::RichText::new("设置新主密码").size(13.0).strong());
                            ui.add_space(4.0);
                            if ui.add_enabled(!is_busy, egui::TextEdit::singleline(&mut new_pwd).password(true).desired_width(f32::INFINITY)).changed() {
                                state.new_password = secrecy::SecretString::from(new_pwd.clone());
                            }
                            
                            // 动态强度显示
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("安全强度:").size(11.0).color(theme.text_secondary));
                                render_mini_strength_bar(ui, strength);
                            });
                            
                            ui.add_space(16.0);

                            // 3. 确认新密码
                            ui.label(egui::RichText::new("确认新主密码").size(13.0).strong());
                            ui.add_space(4.0);
                            if ui.add_enabled(!is_busy, egui::TextEdit::singleline(&mut confirm_pwd).password(true).desired_width(f32::INFINITY)).changed() {
                                state.confirm_password = secrecy::SecretString::from(confirm_pwd.clone());
                            }

                            // 密码不一致警告
                            if !new_pwd.is_empty() && !confirm_pwd.is_empty() && new_pwd != confirm_pwd {
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new("⚠ 两次输入的密码不一致").size(11.0).color(egui::Color32::from_rgb(255, 69, 0)));
                            }

                            ui.add_space(24.0);
                            ui.separator();
                            ui.add_space(16.0);

                            // 进度与操作
                            if is_busy {
                                ui.horizontal(|ui| {
                                    ui.add(egui::ProgressBar::new(state.progress).text("正在重加密数据...").animate(true));
                                });
                                state.progress += 0.01;
                                if state.progress >= 1.0 {
                                    // 核心业务逻辑：持久化 Vault 元数据
                                    let new_password = &state.new_password;
                                    if let Ok(new_meta) = crate::vault::VaultManager::setup_vault(new_password) {
                                        app.settings.security.vault = Some(new_meta);
                                        let _ = app.settings.save();
                                    }
                                    close_modal = true;
                                }
                                ctx.request_repaint();
                            } else {
                                let mut start_busy = false;
                                let can_confirm = {
                                    // 1. 基础校验 (使用借用，不移动所有权)
                                    let basic = !new_pwd.is_empty() && new_pwd == confirm_pwd && strength >= 2;
                                    
                                    // 2. 身份验证：如果已初始化，必须验证原密码
                                    let authenticated = if let Some(meta) = &app.settings.security.vault {
                                        let old_secret = secrecy::SecretString::from(old_pwd.clone());
                                        crate::vault::VaultManager::verify_password(&old_secret, meta)
                                    } else {
                                        true 
                                    };
                                    
                                    basic && authenticated
                                };

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.add_sized([100.0, 32.0], egui::Button::new("确认修改").fill(if can_confirm { theme.accent_base } else { theme.bg_secondary })).clicked() && can_confirm {
                                        start_busy = true;
                                    }
                                    ui.add_space(8.0);
                                    if ui.add_sized([80.0, 32.0], egui::Button::new("取消")).clicked() {
                                        close_modal = true;
                                    }
                                });
                                
                                if start_busy {
                                    state.is_busy = true;
                                    state.progress = 0.0;
                                }
                            }
                        }
                        
                        if close_modal {
                            app.password_change = None;
                        }
                    });
                });
            });
        });
}

fn render_mini_strength_bar(ui: &mut egui::Ui, strength: usize) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for i in 0..5 {
            let color = if i < strength {
                if strength <= 1 { egui::Color32::from_rgb(255, 59, 48) } 
                else if strength <= 3 { egui::Color32::from_rgb(255, 204, 0) } 
                else { egui::Color32::from_rgb(0, 255, 128) } 
            } else {
                egui::Color32::from_rgba_premultiplied(255, 255, 255, 15) 
            };
            let (rect, _) = ui.allocate_exact_size(egui::vec2(20.0, 4.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, egui::CornerRadius::same(1), color);
        }
    });
}
