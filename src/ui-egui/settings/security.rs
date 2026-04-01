use eframe::egui;
use crate::app::RustSshApp;
use crate::ui_egui::theme::Theme;
use crate::ui_egui::settings::render_switch;

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
    ui.label(
        egui::RichText::new(app.i18n.tr("settings.security.vault.title"))
            .size(18.0)
            .strong()
            .color(egui::Color32::WHITE),
    );
    ui.add_space(20.0);

    // 自动锁定超时
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(app.i18n.tr("settings.security.auto_lock.label"))
                    .size(14.0)
                    .color(egui::Color32::WHITE),
            );
            ui.label(
                egui::RichText::new(app.i18n.tr("settings.security.auto_lock.help"))
                    .size(12.0)
                    .color(theme.text_secondary),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut timeout = app.settings.security.idle_timeout_mins;
            let text = match timeout {
                0 => app.i18n.tr("settings.security.timeout.never"),
                1 => app.i18n.tr("settings.security.timeout.minute_1"),
                5 => app.i18n.tr("settings.security.timeout.minute_5"),
                10 => app.i18n.tr("settings.security.timeout.minute_10"),
                30 => app.i18n.tr("settings.security.timeout.minute_30"),
                _ => app.i18n.tr("settings.security.timeout.never"),
            };
            
            // 精确对齐原型图 ComboBox 样式
            ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgb(40, 40, 45); // #28282D
            ui.visuals_mut().widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(40, 40, 45);
            ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 58, 63)); // #3A3A3F

            if egui::ComboBox::from_id_salt("timeout_v2")
                .selected_text(egui::RichText::new(text).size(13.0).color(egui::Color32::WHITE))
                .width(224.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut timeout,
                        1,
                        app.i18n.tr("settings.security.timeout.minute_1"),
                    );
                    ui.selectable_value(
                        &mut timeout,
                        5,
                        app.i18n.tr("settings.security.timeout.minute_5"),
                    );
                    ui.selectable_value(
                        &mut timeout,
                        10,
                        app.i18n.tr("settings.security.timeout.minute_10"),
                    );
                    ui.selectable_value(
                        &mut timeout,
                        30,
                        app.i18n.tr("settings.security.timeout.minute_30"),
                    );
                    ui.selectable_value(
                        &mut timeout,
                        0,
                        app.i18n.tr("settings.security.timeout.never"),
                    );
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
            ui.label(
                egui::RichText::new(app.i18n.tr("settings.security.lock_on_sleep.label"))
                    .size(14.0)
                    .color(egui::Color32::WHITE),
            );
            ui.label(
                egui::RichText::new(app.i18n.tr("settings.security.lock_on_sleep.help"))
                    .size(12.0)
                    .color(theme.text_secondary),
            );
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
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(app.i18n.tr("settings.security.vault.title"))
                    .size(14.0)
                    .color(egui::Color32::WHITE),
            );
            ui.label(
                egui::RichText::new("Vault 功能已迁移至 Iced（此处已废弃）")
                .size(12.0)
                .color(theme.text_secondary),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgb(58, 58, 63); 
            let button_size = egui::vec2(100.0, 32.0);
            let _ = ui
                .add_sized(
                    button_size,
                    egui::Button::new(
                        egui::RichText::new("已废弃")
                            .size(13.0)
                            .color(egui::Color32::WHITE),
                    ),
                )
                .on_hover_text("请使用 Iced 安全中心进行 Vault 初始化/改密");
        });
    });
    ui.add_space(20.0);

    // 分割线 #3A3A3F
    ui.painter().hline(ui.available_rect_before_wrap().x_range(), ui.next_widget_position().y, egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 58, 63)));
    ui.add_space(24.0);

    // --- 2. 系统生物识别 (Biometrics) ---
    ui.label(
        egui::RichText::new(app.i18n.tr("settings.security.biometrics.title"))
            .size(18.0)
            .strong()
            .color(egui::Color32::WHITE),
    );
    ui.add_space(16.0);

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(app.i18n.tr("settings.security.biometrics.label"))
                    .size(14.0)
                    .color(egui::Color32::WHITE),
            );
            ui.label(
                egui::RichText::new(app.i18n.tr("settings.security.biometrics.help"))
                    .size(12.0)
                    .color(theme.text_secondary),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut on = app.settings.security.use_biometrics;
            let resp = ui
                .add_enabled_ui(!app.biometrics_auth_in_progress, |ui| render_switch(ui, &mut on))
                .inner;
            if resp.changed() && on != app.settings.security.use_biometrics {
                let now = ui.ctx().input(|i| i.time);
                // Debounce: block repeated auth requests in a short interval.
                if app.biometrics_auth_in_progress || now - app.biometrics_auth_start_time < 1.2 {
                    app.session_toasts.push(crate::app::Toast {
                        message: app.i18n.tr("toast.biometrics.in_progress").to_string(),
                        is_error: true,
                        created_at: now,
                    });
                    return;
                }

                app.biometrics_auth_in_progress = true;
                let _auth_guard_read = app.biometrics_auth_in_progress;
                let auth = crate::security::biometric_auth::authenticate_user_presence(
                    app.i18n.tr("settings.security.biometrics.reason.toggle"),
                );
                app.biometrics_auth_start_time = ui.ctx().input(|i| i.time);
                app.biometrics_auth_in_progress = false;

                match auth {
                    Ok(_) => {
                        app.settings.security.use_biometrics = on;
                        let _ = app.settings.save();
                        app.session_toasts.push(crate::app::Toast {
                            message: app.i18n.tr("toast.biometrics.updated").to_string(),
                            is_error: false,
                            created_at: ui.ctx().input(|i| i.time),
                        });
                    }
                    Err(e) => {
                        let message = if e.code == crate::security::biometric_auth::BiometricErrorCode::Unknown
                            && !e.detail.is_empty()
                        {
                            app.i18n
                                .tr_fmt("biometric.error.unknown_with_detail", &[("detail", e.detail.trim())])
                        } else {
                            app.i18n.tr(e.i18n_key()).to_string()
                        };
                        app.session_toasts.push(crate::app::Toast {
                            message,
                            is_error: true,
                            created_at: ui.ctx().input(|i| i.time),
                        });
                    }
                }
            }
        });
    });
    
    ui.add_space(20.0);

    ui.add_space(24.0);
}

fn render_hosts(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme) {
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(app.i18n.tr("settings.security.hosts.title"))
            .size(18.0)
            .strong()
            .color(egui::Color32::WHITE),
    );
    ui.add_space(24.0);
    
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(app.i18n.tr("settings.security.hosts.policy.label"))
                    .size(14.0)
                    .color(egui::Color32::WHITE),
            );
            ui.label(
                egui::RichText::new(app.i18n.tr("settings.security.hosts.policy.help"))
                    .size(12.0)
                    .color(theme.text_secondary),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut policy = app.settings.security.host_key_policy;
            let policy_text = match policy {
                crate::settings::HostKeyPolicy::Strict => {
                    app.i18n.tr("settings.security.hosts.policy.strict")
                }
                crate::settings::HostKeyPolicy::Ask => {
                    app.i18n.tr("settings.security.hosts.policy.ask")
                }
                crate::settings::HostKeyPolicy::AcceptNew => {
                    app.i18n.tr("settings.security.hosts.policy.accept_new")
                }
            };
            
            ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgb(40, 40, 45);

            if egui::ComboBox::from_id_salt("host_policy_combo")
                .selected_text(egui::RichText::new(policy_text).size(13.0))
                .width(160.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut policy,
                        crate::settings::HostKeyPolicy::Strict,
                        app.i18n.tr("settings.security.hosts.policy.strict"),
                    );
                    ui.selectable_value(
                        &mut policy,
                        crate::settings::HostKeyPolicy::Ask,
                        app.i18n.tr("settings.security.hosts.policy.ask"),
                    );
                    ui.selectable_value(
                        &mut policy,
                        crate::settings::HostKeyPolicy::AcceptNew,
                        app.i18n.tr("settings.security.hosts.policy.accept_new"),
                    );
                }).response.changed() {
                app.settings.security.host_key_policy = policy;
                let _ = app.settings.save();
            }
        });
    });
    
    ui.add_space(24.0);
    
    ui.label(
        egui::RichText::new(app.i18n.tr("settings.security.hosts.table.title"))
            .size(13.0)
            .strong(),
    );
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
                        ui.add_sized(
                            [150.0, 20.0],
                            egui::Label::new(
                                egui::RichText::new(app.i18n.tr("settings.security.hosts.table.col.host"))
                                    .size(11.0)
                                    .color(theme.text_secondary),
                            ),
                        );
                        ui.add_sized(
                            [80.0, 20.0],
                            egui::Label::new(
                                egui::RichText::new(app.i18n.tr("settings.security.hosts.table.col.algorithm"))
                                    .size(11.0)
                                    .color(theme.text_secondary),
                            ),
                        );
                        ui.add_sized(
                            [300.0, 20.0],
                            egui::Label::new(
                                egui::RichText::new(app.i18n.tr("settings.security.hosts.table.col.fingerprint"))
                                    .size(11.0)
                                    .color(theme.text_secondary),
                            ),
                        );
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

// (prototype v2) no password-strength bar here
