use std::collections::BTreeMap;

use eframe::egui;

use crate::app::{DetailedConfigState, RustSshApp};
use crate::session::{AuthMethod, ProtocolType, SessionProfile, TransportConfig};
use crate::ui_egui::theme::Theme;

pub fn render(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme, tab_idx: usize) {
    match tab_idx {
        0 => render_protocol_page(app, ui, theme, ProtocolType::SSH, "搜索 SSH 会话..."),
        1 => render_protocol_page(app, ui, theme, ProtocolType::Telnet, "搜索 TELNET 会话..."),
        2 => render_protocol_page(app, ui, theme, ProtocolType::Serial, "搜索 SERIAL 会话..."),
        3 => render_advanced(ui, theme),
        _ => {}
    }
}

fn render_protocol_page(
    app: &mut RustSshApp,
    ui: &mut egui::Ui,
    theme: &Theme,
    protocol: ProtocolType,
    search_hint: &str,
) {
    let title = match protocol {
        ProtocolType::SSH => "SSH 连接信息管理",
        ProtocolType::Telnet => "TELNET 连接信息管理",
        ProtocolType::Serial => "SERIAL 连接信息管理",
    };
    ui.add_space(8.0);
    ui.label(egui::RichText::new(title).size(18.0).strong().color(egui::Color32::WHITE));
    ui.add_space(16.0);

    ui.horizontal(|ui| {
        ui.add_sized(
            [ui.available_width() - 90.0, 30.0],
            egui::TextEdit::singleline(&mut app.settings_search_text)
                .hint_text(search_hint)
                .desired_width(f32::INFINITY),
        );
        if ui.button("＋ 新增").clicked() {
            let mut state = DetailedConfigState::default();
            state.protocol = protocol;
            app.editing_session = Some(state);
            app.show_settings = false;
        }
    });
    ui.add_space(12.0);

    let search = app.settings_search_text.trim().to_lowercase();
    let mut grouped: BTreeMap<String, Vec<SessionProfile>> = BTreeMap::new();
    for s in &app.session_manager.library.sessions {
        if !matches_protocol(&s.transport, protocol) {
            continue;
        }
        if !matches_search(s, &search) {
            continue;
        }
        let group_name = s
            .folder
            .as_ref()
            .map(|g| g.trim().to_string())
            .filter(|g| !g.is_empty())
            .unwrap_or_else(|| "Default".to_string());
        grouped.entry(group_name).or_default().push(s.clone());
    }
    for sessions in grouped.values_mut() {
        sessions.sort_by_key(|s| s.name.to_lowercase());
    }

    if grouped.is_empty() {
        ui.label(egui::RichText::new("暂无匹配会话").color(theme.text_secondary));
        return;
    }

    let mut pending_delete_id: Option<String> = None;
    let mut pending_edit: Option<DetailedConfigState> = None;

    for (group, sessions) in grouped {
        let default_group = group == "Default" || group == "未分类";
        if default_group {
            render_group_rows(ui, &sessions, &mut pending_edit, &mut pending_delete_id);
        } else {
            egui::CollapsingHeader::new(group)
                .default_open(true)
                .show(ui, |ui| {
                    render_group_rows(ui, &sessions, &mut pending_edit, &mut pending_delete_id);
                });
        }
    }

    if let Some(edit_state) = pending_edit {
        app.editing_session = Some(edit_state);
        app.show_settings = false;
    }

    if let Some(id) = pending_delete_id {
        app.session_manager.library.sessions.retain(|s| s.id != id);
        let mut sm = app.session_manager.clone();
        tokio::spawn(async move {
            let _ = sm.delete_session(&id).await;
        });
    }
}

fn render_group_rows(
    ui: &mut egui::Ui,
    sessions: &[SessionProfile],
    pending_edit: &mut Option<DetailedConfigState>,
    pending_delete_id: &mut Option<String>,
) {
    for s in sessions {
        ui.horizontal(|ui| {
            ui.set_min_height(32.0);
            ui.label(egui::RichText::new(&s.name).color(egui::Color32::WHITE));
            ui.add_space(12.0);
            ui.label(egui::RichText::new(target_of(s)).monospace().color(egui::Color32::from_gray(160)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("删除").clicked() {
                    *pending_delete_id = Some(s.id.clone());
                }
                if ui.small_button("编辑").clicked() {
                    *pending_edit = Some(to_edit_state(s));
                }
            });
        });
        ui.separator();
    }
}

fn render_advanced(ui: &mut egui::Ui, theme: &Theme) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new("连接库高级设置").size(18.0).strong().color(egui::Color32::WHITE));
    ui.add_space(16.0);
    ui.label(egui::RichText::new("导入/导出、备份、默认协议等能力预留。").color(theme.text_secondary));
}

fn matches_protocol(transport: &TransportConfig, protocol: ProtocolType) -> bool {
    matches!(
        (transport, protocol),
        (TransportConfig::Ssh(_), ProtocolType::SSH)
            | (TransportConfig::Telnet(_), ProtocolType::Telnet)
            | (TransportConfig::Serial(_), ProtocolType::Serial)
    )
}

fn matches_search(s: &SessionProfile, search: &str) -> bool {
    if search.is_empty() {
        return true;
    }
    let name_match = s.name.to_lowercase().contains(search);
    let target_match = target_of(s).to_lowercase().contains(search);
    name_match || target_match
}

fn target_of(s: &SessionProfile) -> String {
    match &s.transport {
        TransportConfig::Ssh(ssh) => format!("{}:{}", ssh.host, ssh.port),
        TransportConfig::Telnet(telnet) => format!("{}:{}", telnet.host, telnet.port),
        TransportConfig::Serial(serial) => serial.port.clone(),
    }
}

fn to_edit_state(s: &SessionProfile) -> DetailedConfigState {
    let mut state = DetailedConfigState::default();
    state.id = Some(s.id.clone());
    state.name = s.name.clone();
    state.folder = s.folder.clone().unwrap_or_else(|| "Default".to_string());
    if let Some(color) = s.color_tag {
        state.color_tag = color;
    }

    match &s.transport {
        TransportConfig::Ssh(ssh) => {
            state.protocol = ProtocolType::SSH;
            state.ssh_host = ssh.host.clone();
            state.ssh_port = ssh.port.to_string();
            state.ssh_user = ssh.user.clone();
            match &ssh.auth {
                AuthMethod::Key { private_key_path } => {
                    state.ssh_private_key_path = private_key_path.clone();
                    state.ssh_auth = AuthMethod::Key {
                        private_key_path: private_key_path.clone(),
                    };
                }
                a => state.ssh_auth = a.clone(),
            }
        }
        TransportConfig::Telnet(telnet) => {
            state.protocol = ProtocolType::Telnet;
            state.telnet_host = telnet.host.clone();
            state.telnet_port = telnet.port.to_string();
        }
        TransportConfig::Serial(serial) => {
            state.protocol = ProtocolType::Serial;
            state.serial_port = serial.port.clone();
            state.serial_baud = serial.baud_rate.to_string();
        }
    }
    state
}

