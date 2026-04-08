use iced::Task;

use super::super::message::{Message, SettingsField};
use super::super::state::IcedState;
use super::super::terminal_viewport;

/// Handle SettingsDismiss message.
pub(crate) fn handle_settings_dismiss(state: &mut IcedState) -> Task<Message> {
    if state.settings_anim.phase == super::super::state::ModalAnimPhase::Closed {
        state.settings_modal_open = false;
    } else if state.settings_anim.phase != super::super::state::ModalAnimPhase::Closing {
        state.settings_anim = super::super::state::ModalAnimState::closing(state.tick_count);
    }
    Task::none()
}

/// Handle SettingsCategoryChanged message.
pub(crate) fn handle_settings_category(
    state: &mut IcedState,
    cat: crate::app::message::SettingsCategory,
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
                state.model.settings.save_with_log();
                state.model.status = state.model.i18n.tr("toast.biometrics.updated").to_string();
            }
            Err(e) => {
                state.model.status = state.model.i18n.tr(e.i18n_key()).to_string();
            }
        }
    } else {
        state.model.settings.security.use_biometrics = false;
        state.model.settings.save_with_log();
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
        SettingsField::TargetFps(v) => s.terminal.target_fps = v,
        SettingsField::ColorScheme(v) => s.terminal.color_scheme = v,
        SettingsField::TerminalFontSize(v) => s.terminal.font_size = v,
        SettingsField::LineHeight(v) => s.terminal.line_height = v,
        SettingsField::ApplyTerminalMetrics(v) => s.terminal.apply_terminal_metrics = v,
        SettingsField::FontFamily(v) => s.terminal.font_family = v,
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
        SettingsField::SingleSharedSession(_v) => {
            // No-op: all tabs now pump independently regardless of this setting.
            // Kept for backward compatibility (stored in settings file).
        }
        SettingsField::IdleTimeoutMins(v) => s.security.idle_timeout_mins = v,
        SettingsField::LockOnSleep(v) => s.security.lock_on_sleep = v,
        SettingsField::KdfMemoryLevel(v) => s.security.kdf_memory_level = v,
        SettingsField::HostKeyPolicy(p) => s.security.host_key_policy = p,
        SettingsField::ConnectionSearch(q) => {
            state.settings_connection_search = q;
            return;
        }
        SettingsField::AutoReconnect(v) => s.quick_connect.auto_reconnect = v,
        SettingsField::ReconnectMaxAttempts(v) => s.quick_connect.reconnect_max_attempts = v,
        SettingsField::ReconnectBaseDelay(v) => s.quick_connect.reconnect_base_delay_secs = v,
        SettingsField::ReconnectExponential(v) => s.quick_connect.reconnect_exponential = v,
        SettingsField::RestoreLastSession(v) => s.quick_connect.restore_last_session = v,
    }

    state.model.settings.save_with_log();

    if sync_layout {
        let window_size = state.window_size;
        let term_settings = state.model.settings.terminal.clone();
        for (i, pane) in state.tab_panes.iter_mut().enumerate() {
            let spec = terminal_viewport::terminal_viewport_spec_for_settings(&term_settings);
            let (cols, rows) =
                terminal_viewport::grid_from_window_size_with_spec(window_size, &spec);
            let prev_rows = pane.terminal.grid_size().1;
            if let Some(session) = state.tab_manager.session_mut(i) {
                let _ = pane.terminal.resize_and_sync_pty(session, cols, rows);
            } else {
                pane.terminal.resize(cols, rows);
            }
            if prev_rows != rows {
                pane.styled_row_cache.clear();
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
