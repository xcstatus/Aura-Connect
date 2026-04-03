use iced::Task;

use super::super::message::{Message, SettingsField};
use super::super::state::IcedState;

/// Handle SettingsDismiss message.
pub(crate) fn handle_settings_dismiss(state: &mut IcedState) -> Task<Message> {
    state.settings_modal_open = false;
    Task::none()
}

/// Handle SettingsCategoryChanged message.
pub(crate) fn handle_settings_category(
    state: &mut IcedState,
    cat: crate::iced_app::message::SettingsCategory,
) -> Task<Message> {
    state.settings_category = cat;
    let i = cat as usize;
    state.settings_sub_tab[i] =
        super::super::settings_modal::clamp_sub_tab(cat, state.settings_sub_tab[i]);
    Task::none()
}

/// Handle SettingsSubTabChanged message.
pub(crate) fn handle_settings_sub_tab(state: &mut IcedState, sub: usize) -> Task<Message> {
    let cat = state.settings_category;
    let i = cat as usize;
    state.settings_sub_tab[i] = super::super::settings_modal::clamp_sub_tab(cat, sub);
    Task::none()
}

/// Handle SettingsFieldChanged message.
pub(crate) fn handle_settings_field(state: &mut IcedState, field: SettingsField) -> Task<Message> {
    apply_settings_field(state, field);
    Task::none()
}

/// Handle BiometricsToggle message.
pub(crate) fn handle_biometrics_toggle(state: &mut IcedState, desired: bool) -> Task<Message> {
    if desired {
        let reason = state
            .model
            .i18n
            .tr("settings.security.biometrics.reason.toggle");
        match crate::security::biometric_auth::authenticate_user_presence(reason) {
            Ok(()) => {
                state.model.settings.security.use_biometrics = true;
                let _ = state.model.settings.save();
                state.model.status = state.model.i18n.tr("toast.biometrics.updated").to_string();
            }
            Err(e) => {
                state.model.status = state.model.i18n.tr(e.i18n_key()).to_string();
            }
        }
    } else {
        state.model.settings.security.use_biometrics = false;
        let _ = state.model.settings.save();
    }
    Task::none()
}

/// Handle SettingsRestartAcknowledged message.
pub(crate) fn handle_settings_restart_ack(state: &mut IcedState) -> Task<Message> {
    state.settings_needs_restart = false;
    Task::none()
}

/// Handle SaveSettings message.
pub(crate) fn handle_save_settings(state: &mut IcedState) -> Task<Message> {
    state.model.status = if state.model.settings.save().is_ok() {
        super::super::update::sync_terminal_grid_to_session(state);
        for p in &mut state.tab_panes {
            p.terminal.refresh_terminal_snapshots();
        }
        "Settings saved".to_string()
    } else {
        "Settings save failed".to_string()
    };
    Task::none()
}

fn mark_restart_if_gpu_setting(state: &mut IcedState) {
    state.settings_needs_restart = true;
}

fn apply_settings_field(state: &mut IcedState, field: SettingsField) {
    let sync_layout = matches!(
        &field,
        SettingsField::LineHeight(_)
            | SettingsField::TerminalFontSize(_)
            | SettingsField::ApplyTerminalMetrics(_)
    );
    let sync_palette = matches!(&field, SettingsField::ColorScheme(_));
    let s = &mut state.model.settings;

    match field {
        SettingsField::Language(code) => {
            s.general.language = code.clone();
            let _ = s.save();
            state.model.set_ui_language(&code);
            return;
        }
        SettingsField::AutoCheckUpdate(v) => s.general.auto_check_update = v,
        SettingsField::Theme(v) => s.general.theme = v,
        SettingsField::AccentColor(v) => s.general.accent_color = v.trim().to_string(),
        SettingsField::FontSize(v) => s.general.font_size = v,
        SettingsField::GpuAcceleration(v) => {
            s.terminal.gpu_acceleration = v;
            mark_restart_if_gpu_setting(state);
        }
        SettingsField::TargetFps(v) => s.terminal.target_fps = v,
        SettingsField::AtlasResetOnPressure(v) => s.terminal.atlas_reset_on_pressure = v,
        SettingsField::ColorScheme(v) => s.terminal.color_scheme = v,
        SettingsField::TerminalFontSize(v) => s.terminal.font_size = v,
        SettingsField::LineHeight(v) => s.terminal.line_height = v,
        SettingsField::ApplyTerminalMetrics(v) => s.terminal.apply_terminal_metrics = v,
        SettingsField::FontFamily(v) => s.terminal.font_family = v,
        SettingsField::GpuFontPath(v) => {
            let t = v.trim().to_string();
            s.terminal.gpu_font_path = if t.is_empty() { None } else { Some(t) };
            mark_restart_if_gpu_setting(state);
        }
        SettingsField::GpuFontFaceIndex(v) => {
            let t = v.trim();
            s.terminal.gpu_font_face_index = if t.is_empty() { None } else { t.parse().ok() };
            mark_restart_if_gpu_setting(state);
        }
        SettingsField::RightClickPaste(v) => s.terminal.right_click_paste = v,
        SettingsField::BracketedPaste(v) => {
            s.terminal.bracketed_paste = v;
            for p in &mut state.tab_panes {
                p.terminal.set_bracketed_paste(v);
            }
        }
        SettingsField::KeepSelectionHighlight(v) => s.terminal.keep_selection_highlight = v,
        SettingsField::ScrollbackLimit(v) => s.terminal.scrollback_limit = v,
        SettingsField::HistorySearch(v) => s.terminal.history_search_enabled = v,
        SettingsField::PathCompletion(v) => s.terminal.local_path_completion_enabled = v,
        SettingsField::SingleSharedSession(v) => {
            s.quick_connect.single_shared_session = v;
            if v {
                let keep = state.active_tab;
                super::super::update::enforce_single_session_policy(state, Some(keep));
                super::super::update::sync_terminal_grid_to_session(state);
                super::super::terminal_host::TerminalHost::sync_focus_report(state);
            }
        }
        SettingsField::IdleTimeoutMins(v) => s.security.idle_timeout_mins = v,
        SettingsField::LockOnSleep(v) => s.security.lock_on_sleep = v,
        SettingsField::HostKeyPolicy(p) => s.security.host_key_policy = p,
        SettingsField::ConnectionSearch(q) => {
            state.settings_connection_search = q;
            return;
        }
    }

    let _ = state.model.settings.save();

    if sync_layout {
        if state.model.settings.quick_connect.single_shared_session {
            super::super::update::sync_terminal_grid_to_session(state);
        } else {
            let window_size = state.window_size;
            let term_settings = state.model.settings.terminal.clone();
            for p in &mut state.tab_panes {
                super::super::update::apply_terminal_grid_resize_for_pane(
                    window_size,
                    p,
                    &term_settings,
                );
            }
        }
    }

    if sync_palette {
        let scheme = state.model.settings.terminal.color_scheme.clone();
        for p in &mut state.tab_panes {
            p.terminal.apply_terminal_palette_for_scheme(&scheme);
        }
    }
}
