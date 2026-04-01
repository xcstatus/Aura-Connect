use eframe::egui;
use secrecy::ExposeSecret;

use crate::app::{RustSshApp, Toast};
use crate::ui_egui::theme::Theme;

pub fn session_group(
    ui: &mut egui::Ui,
    theme: &Theme,
    border_subtle: egui::Color32,
    title: &str,
    add: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::new()
        .fill(theme.surface_1)
        .stroke(egui::Stroke::new(1.0, border_subtle))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(20))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(title)
                    .size(14.0)
                    .strong()
                    .color(egui::Color32::WHITE),
            );
            ui.add_space(12.0);
            add(ui);
        });
}

pub fn session_row(
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
        ui.add_sized(
            [label_w, 20.0],
            egui::Label::new(egui::RichText::new(l).size(12.0).color(label_color)),
        );
        add(ui)
    })
    .inner
}

pub fn push_toast(app: &mut RustSshApp, now: f64, message: impl Into<String>, is_error: bool) {
    app.session_toasts.push(Toast {
        message: message.into(),
        is_error,
        created_at: now,
    });
}

pub fn start_test_connection(app: &mut RustSshApp, now: f64) {
    app.session_test_in_progress = true;
    app.session_test_started_at = now;

    if let Some(state) = app.editing_session.as_ref() {
        let host = state.ssh_host.trim().to_string();
        let user = state.ssh_user.trim().to_string();
        let port = state.ssh_port.parse::<u16>().unwrap_or(22);
        if host.is_empty() || user.is_empty() {
            push_toast(app, now, "请先填写主机地址与用户名", true);
            app.session_test_in_progress = false;
            return;
        }

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

pub fn save_session_or_toast(app: &mut RustSshApp, now: f64, first_error_rect: &mut Option<egui::Rect>) {
    let mut err_msg: Option<String> = None;
    if let Some(state) = app.editing_session.as_ref() {
        if state.name.trim().is_empty() {
            err_msg = Some("名称为必填项".to_string());
        } else if state.ssh_host.trim().is_empty() {
            err_msg = Some("主机地址为必填项".to_string());
        } else if state.ssh_user.trim().is_empty() {
            err_msg = Some("用户名为必填项".to_string());
        } else if !state
            .ssh_port
            .parse::<u32>()
            .ok()
            .is_some_and(|p| p >= 1 && p <= 65535)
        {
            err_msg = Some("端口范围 1–65535".to_string());
        }
    }

    if let Some(msg) = err_msg {
        push_toast(app, now, msg, true);
        // scroll to first error is handled by caller (needs Ui)
        first_error_rect.take();
        return;
    }

    if let Some(state) = app.editing_session.take() {
        let mut sm = app.session_manager.clone();
        let id = state.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let name = state.name.trim().to_string();
        let folder = state.folder.trim().to_string();
        let host = state.ssh_host.trim().to_string();
        let user = state.ssh_user.trim().to_string();
        let port = state.ssh_port.parse().unwrap_or(22);
        let auth = match state.ssh_auth.clone() {
            crate::session::AuthMethod::Key { .. } => crate::session::AuthMethod::Key {
                private_key_path: state.ssh_private_key_path.clone(),
            },
            other => other,
        };

        // Vault integration is deprecated for egui UI; do not persist secrets here.
        let credential_id = None;
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
                credential_id,
            }),
        };

        // 先同步到内存，确保快速连接列表立即可见
        if let Some(existing) = app
            .session_manager
            .library
            .sessions
            .iter_mut()
            .find(|s| s.id == config.id)
        {
            *existing = config.clone();
        } else {
            app.session_manager.library.sessions.push(config.clone());
        }

        tokio::spawn(async move {
            let _ = sm.upsert_session(config).await;
        });
    }
}

pub fn render_toasts(ctx: &egui::Context, theme: &Theme, toasts: &[Toast]) {
    if toasts.is_empty() {
        return;
    }
    let accent = theme.accent_base;
    let error = egui::Color32::from_rgb(255, 59, 48);
    let text_primary = egui::Color32::WHITE;

    let toast_area = egui::Area::new(egui::Id::new("session_toasts_single_column"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 16.0));
    toast_area.show(ctx, |ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        for t in toasts {
            let bg = if t.is_error {
                error.gamma_multiply(0.2)
            } else {
                accent.gamma_multiply(0.15)
            };
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
}

