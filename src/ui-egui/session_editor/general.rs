use eframe::egui;

use crate::app::RustSshApp;
use crate::ui_egui::theme::Theme;

use super::shared::{session_group, session_row};

pub fn render(
    ui: &mut egui::Ui,
    app: &mut RustSshApp,
    theme: &Theme,
    border_subtle: egui::Color32,
    error: egui::Color32,
    first_error_rect: &mut Option<egui::Rect>,
) {
    let Some(state) = app.editing_session.as_mut() else { return; };
    let text_secondary = theme.text_secondary;

    session_group(ui, theme, border_subtle, "通用（General）", |ui| {
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
                egui::Label::new(egui::RichText::new("认证方式 *").size(12.0).color(text_secondary)),
            );
            let mut current = match &state.ssh_auth {
                crate::session::AuthMethod::Key { .. } => 1,
                _ => 0,
            };
            let pw = ui.radio_value(&mut current, 0, "密码");
            let key = ui.radio_value(&mut current, 1, "私钥");
            if pw.clicked() {
                state.ssh_auth = crate::session::AuthMethod::Password;
            }
            if key.clicked() {
                state.ssh_auth = crate::session::AuthMethod::Key {
                    private_key_path: state.ssh_private_key_path.clone(),
                };
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
                        if toggle.clicked() {
                            state.ui_show_password = !state.ui_show_password;
                        }
                        resp
                    })
                    .inner
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
                    })
                    .inner
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
                        if toggle.clicked() {
                            state.ui_show_passphrase = !state.ui_show_passphrase;
                        }
                        resp
                    })
                    .inner
                });
            }
            _ => {}
        }
    });
}

