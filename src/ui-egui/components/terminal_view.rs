use eframe::egui;
use std::path::{Path, PathBuf};
#[cfg(debug_assertions)]
use std::time::Instant;
use std::time::Duration;

use crate::backend::ssh_session::AsyncSession;
use crate::terminal::backend::{write_to_session, GhosttyVtBackend, TerminalBackend};
use crate::terminal::display_test;
use crate::terminal::session_bridge::TerminalSessionBridge;
use crate::terminal::vt_widget::{
    TERMINAL_CELL_WIDTH, TERMINAL_PADDING_X, TERMINAL_PADDING_Y, TERMINAL_ROW_HEIGHT,
};
use crate::ui_egui::theme::Theme;
use crate::settings::TerminalSettings;

pub struct TerminalView {
    backend_vt: GhosttyVtBackend,
    bridge: TerminalSessionBridge,
    last_cols: u16,
    last_rows: u16,
    last_pty: (u16, u16),
    fallback_text: String,
    osc_pending: Vec<u8>,
    cwd_hint: Option<String>,
    input_buffer: String,
    command_history: Vec<String>,
    history_nav: Option<usize>,
    history_matches: Vec<String>,
    history_seed_buffer: String,
    display_test_injected: bool,
    #[cfg(debug_assertions)]
    diag_last_has_focus: Option<bool>,
    #[cfg(debug_assertions)]
    diag_last_wants_keyboard: Option<bool>,
    #[cfg(debug_assertions)]
    diag_last_focused_id: Option<String>,
}

impl TerminalView {
    pub fn new() -> Self {
        let mut this = Self {
            backend_vt: GhosttyVtBackend::new(),
            bridge: TerminalSessionBridge::new(),
            last_cols: 0,
            last_rows: 0,
            last_pty: (0, 0),
            fallback_text: String::new(),
            osc_pending: Vec::new(),
            cwd_hint: None,
            input_buffer: String::new(),
            command_history: Vec::new(),
            history_nav: None,
            history_matches: Vec::new(),
            history_seed_buffer: String::new(),
            display_test_injected: false,
            #[cfg(debug_assertions)]
            diag_last_has_focus: None,
            #[cfg(debug_assertions)]
            diag_last_wants_keyboard: None,
            #[cfg(debug_assertions)]
            diag_last_focused_id: None,
        };
        // Improve responsiveness for high-throughput TUI apps (e.g. top/htop).
        this.bridge.set_budgets(256 * 1024, 256);
        this
    }

    pub fn current_cwd(&self) -> Option<&str> {
        self.cwd_hint.as_deref()
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        _theme: &Theme,
        session: &mut Option<Box<dyn AsyncSession>>,
        terminal_settings: &TerminalSettings,
        keep_selection_highlight: bool,
        history_search_enabled: bool,
        local_path_completion_enabled: bool,
        frame: &mut eframe::Frame,
    ) {
        self.backend_vt.apply_settings(terminal_settings);
        self.backend_vt
            .set_keep_selection_highlight(keep_selection_highlight);

        let (rect, _alloc_response) = ui.allocate_at_least(ui.available_size(), egui::Sense::hover());
        let rect_visible = ui.is_rect_visible(rect);
        let terminal_focus_id = egui::Id::new("terminal_view_focus_global");
        let response = ui.interact(rect, terminal_focus_id, egui::Sense::click());
        #[cfg(debug_assertions)]
        {
            // If you can't see this red border + text, the terminal rect is not visible (clipped/zero/covered).
            ui.painter().rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgba_unmultiplied(255, 0, 0, 180)),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.left_top() + egui::vec2(TERMINAL_PADDING_X, TERMINAL_PADDING_Y),
                egui::Align2::LEFT_TOP,
                format!(
                    "TERM-RECT {:.0}x{:.0} active={} fallback_len={}",
                    rect.width(),
                    rect.height(),
                    self.backend_vt.is_active(),
                    self.fallback_text.len()
                ),
                egui::FontId::monospace(11.0),
                egui::Color32::from_rgb(255, 220, 220),
            );
        }

        if response.clicked() {
            ui.memory_mut(|mem| mem.request_focus(terminal_focus_id));
        }

        // --- 输入处理 ---
        let has_focus = ui.memory(|mem| mem.has_focus(terminal_focus_id));

        // Recommended egui way: when a widget has focus, set an EventFilter to
        // keep Tab/Arrow/Esc from triggering egui focus navigation.
        if has_focus {
            ui.memory_mut(|mem| {
                mem.set_focus_lock_filter(
                    terminal_focus_id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                );
            });
        }

        // Snapshot events BEFORE any mutation.
        let events = ui.input(|i| i.events.clone());
        let _modifiers = ui.input(|i| i.modifiers);
        #[cfg(debug_assertions)]
        {
            let wants_keyboard = ui.ctx().wants_keyboard_input();
            let focused_id = ui
                .ctx()
                .memory(|mem| mem.focused())
                .map(|id| format!("{id:?}"));
            let changed = self.diag_last_has_focus != Some(has_focus)
                || self.diag_last_wants_keyboard != Some(wants_keyboard)
                || self.diag_last_focused_id != focused_id;
            if changed {
                log::debug!(
                    target: "term-diag",
                    "[term-diag] focus_owner terminal_has_focus={} wants_keyboard={} focused_id={:?}",
                    has_focus,
                    wants_keyboard,
                    focused_id
                );
                self.diag_last_has_focus = Some(has_focus);
                self.diag_last_wants_keyboard = Some(wants_keyboard);
                self.diag_last_focused_id = focused_id;
            }
        }

        #[cfg(debug_assertions)]
        {
            for event in &events {
                if let egui::Event::Key { key, pressed, .. } = event {
                    if *pressed
                        && matches!(key, egui::Key::Tab | egui::Key::ArrowUp | egui::Key::ArrowDown)
                    {
                        let remote_ready = session.as_ref().is_some_and(|s| s.is_connected());
                        log::debug!(target: "term-diag",
                            "[term-diag] key_event key={:?} has_focus={} remote_ready={} input_len={} hist_nav={}",
                            key,
                            has_focus,
                            remote_ready,
                            self.input_buffer.len(),
                            self.history_nav.is_some()
                        );
                    }
                }
            }
        }

        let mut any_input_event = false;
        if has_focus {
            for event in &events {
                #[cfg(debug_assertions)]
                let event_started = Instant::now();
                match event {
                    egui::Event::WindowFocused(focused) => {
                        any_input_event = true;
                        let bytes = self.backend_vt.encode_focus_event(*focused);
                        let _ = write_to_session(session, &bytes);
                    }
                    egui::Event::Key { key, pressed, repeat: _repeat, .. } => {
                        if *pressed {
                            any_input_event = true;
                            let remote_ready = session.as_ref().is_some_and(|s| s.is_connected());

                            // Local completion is fallback-only and runs only when no remote session.
                            if !remote_ready
                                && local_path_completion_enabled
                                && *key == egui::Key::Tab
                            {
                                if self.handle_local_path_completion(session) {
                                    continue;
                                }
                            }

                            // Local history interception only when remote is unavailable.
                            if !remote_ready
                                && history_search_enabled
                                && self.handle_history_key(*key, session)
                            {
                                continue;
                            }

                            self.update_local_input_state_on_key(*key);
                        }
                        {
                            let bytes = self.backend_vt.on_key(*key, *pressed, _modifiers);
                            let _ = write_to_session(session, &bytes);
                        }
                        #[cfg(debug_assertions)]
                        {
                            let cost = event_started.elapsed().as_millis();
                            if cost > 24 {
                                log::warn!(target: "term-diag",
                                    "[term-diag] key_event slow key={:?} pressed={} cost={}ms",
                                    key,
                                    pressed,
                                    cost
                                );
                            }
                        }
                    }
                    egui::Event::Text(text) => {
                        any_input_event = true;
                        self.update_local_input_state_on_text(text);
                        {
                            let bytes = self.backend_vt.on_text(text);
                            let _ = write_to_session(session, &bytes);
                        }
                        #[cfg(debug_assertions)]
                        {
                            let cost = event_started.elapsed().as_millis();
                            if cost > 24 {
                                log::warn!(target: "term-diag",
                                    "[term-diag] text_event slow len={} cost={}ms",
                                    text.len(),
                                    cost
                                );
                            }
                        }
                    }
                    egui::Event::Paste(text) => {
                        any_input_event = true;
                        self.update_local_input_state_on_paste(text);
                        let bytes = self.backend_vt.encode_paste(text);
                        let _ = write_to_session(session, &bytes);
                        #[cfg(debug_assertions)]
                        {
                            let cost = event_started.elapsed().as_millis();
                            if cost > 24 {
                                log::warn!(target: "term-diag",
                                    "[term-diag] paste_event slow len={} cost={}ms",
                                    text.len(),
                                    cost
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // --- 计算并同步 PTY 大小 (2.2.5) ---
        let inner_width = (rect.width() - TERMINAL_PADDING_X * 2.0).max(1.0);
        let inner_height = (rect.height() - TERMINAL_PADDING_Y * 2.0).max(1.0);
        let cols = ((inner_width / TERMINAL_CELL_WIDTH).floor() as u16).max(1);
        let row_h = (TERMINAL_ROW_HEIGHT * terminal_settings.line_height).max(1.0);
        let rows = ((inner_height / row_h).floor() as u16).max(1);

        // Ensure the VT backend is created as early as possible so that incoming
        // session bytes are not dropped before the first resize event.
        if !self.backend_vt.is_active() {
            self.backend_vt.resize_cells(cols, rows);
        }

        if cols != self.last_cols || rows != self.last_rows {
            {
                self.backend_vt.resize_cells(cols, rows);
            }
            if let Some(s) = session.as_mut() {
                let _ = self.bridge.resize_pty_if_needed(s.as_mut(), &mut self.last_pty, cols, rows);
            }
            self.last_cols = cols;
            self.last_rows = rows;
            log::debug!("Ghostty Target Resized to: {}x{}", cols, rows);
        }

        // Optional local display fixture for GPU/CPU visual comparison without SSH traffic.
        // Enable with:
        // - RUST_SSH_TERM_DISPLAY_TEST=1 (includes combining marks; should force CPU fallback)
        // - RUST_SSH_TERM_DISPLAY_TEST=2 (omits combining marks; should allow GPU text)
        let can_inject_display_test = session
            .as_ref()
            .is_none_or(|s| !s.is_connected());
        let display_test_mode = std::env::var("RUST_SSH_TERM_DISPLAY_TEST").ok();
        if !self.display_test_injected
            && can_inject_display_test
            && matches!(display_test_mode.as_deref(), Some("1" | "2"))
        {
            let payload = display_test::test_pattern_bytes();
            log::info!(
                target: "term-diag",
                "display test injected: bytes={} rows={} cols_estimate={}",
                payload.len(),
                payload.iter().filter(|&&b| b == b'\n').count().saturating_add(1),
                self.last_cols
            );
            self.backend_vt.ingest_output(&payload);
            self.display_test_injected = true;
            ui.ctx().request_repaint();
        }

        let mut drained_n = 0usize;
        if let Some(s) = session.as_mut() {
            #[cfg(debug_assertions)]
            let drain_started = Instant::now();
            let mut drained_chunks = 0usize;
            let backend_vt = &mut self.backend_vt;
            let osc_pending = &mut self.osc_pending;
            let cwd_hint = &mut self.cwd_hint;
            let fallback_text = &mut self.fallback_text;
            let drained = self.bridge.drain_output(s.as_mut(), |chunk| {
                drained_chunks += 1;
                Self::capture_osc_cwd_into(osc_pending, cwd_hint, chunk);
                backend_vt.ingest_output(chunk);
                if !backend_vt.is_active() {
                    fallback_text.push_str(&String::from_utf8_lossy(chunk));
                    const MAX_FALLBACK_TEXT_BYTES: usize = 256 * 1024;
                    if fallback_text.len() > MAX_FALLBACK_TEXT_BYTES {
                        let keep = MAX_FALLBACK_TEXT_BYTES;
                        let cut = fallback_text.len().saturating_sub(keep);
                        let tail = fallback_text.split_off(cut);
                        *fallback_text = tail;
                    }
                }
            });
            drained_n = drained.ok().unwrap_or(0);
            if drained_n > 0 {
                ui.ctx().request_repaint();
            }
            #[cfg(debug_assertions)]
            {
                let cost = drain_started.elapsed().as_millis();
                if cost > 24 {
                    log::warn!(target: "term-diag",
                        "[term-diag] drain_output slow bytes={} chunks={} cost={}ms",
                        drained_n,
                        drained_chunks,
                        cost
                    );
                }
            }
        }

        // Occlusion: when terminal area is not visible, skip painting to pause GPU callbacks.
        if rect_visible {
            self.backend_vt.paint(ui, rect, frame);
        }
        let show_fallback = !self.backend_vt.is_active();

        if show_fallback {
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("terminal_fallback_scroll")
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(&self.fallback_text)
                                .monospace()
                                .size(13.0),
                        );
                    });
            });
        }

        // Idle throttling: if there's no output and no local input activity, reduce repaint rate.
        // Cursor blinking is handled inside `VtTerminalWidget` (it will schedule its own repaints).
        if rect_visible && drained_n == 0 && !any_input_event {
            ui.ctx().request_repaint_after(Duration::from_millis(250));
        }
    }

    fn handle_history_key(
        &mut self,
        key: egui::Key,
        session: &mut Option<Box<dyn AsyncSession>>,
    ) -> bool {
        if key != egui::Key::ArrowUp && key != egui::Key::ArrowDown {
            return false;
        }
        if self.command_history.is_empty() {
            return false;
        }

        if self.history_nav.is_none() {
            let query = self
                .input_buffer
                .strip_suffix('%')
                .map(str::trim)
                .map(str::to_string);
            self.history_seed_buffer = self.input_buffer.clone();

            self.history_matches = if let Some(q) = query {
                if q.is_empty() {
                    self.command_history.iter().rev().cloned().collect()
                } else {
                    self.command_history
                        .iter()
                        .rev()
                        .filter(|cmd| cmd.contains(&q))
                        .cloned()
                        .collect()
                }
            } else if self.input_buffer.trim().is_empty() {
                self.command_history.iter().rev().cloned().collect()
            } else {
                let seed = self.input_buffer.trim().to_string();
                self.command_history
                    .iter()
                    .rev()
                    .filter(|cmd| cmd.starts_with(&seed))
                    .cloned()
                    .collect()
            };
        }

        if self.history_matches.is_empty() {
            return false;
        }

        let current = self.history_nav.unwrap_or(usize::MAX);
        let next = if key == egui::Key::ArrowUp {
            if current == usize::MAX { 0 } else { (current + 1).min(self.history_matches.len() - 1) }
        } else if current == usize::MAX {
            return false;
        } else if current == 0 {
            self.history_nav = None;
            self.input_buffer = self.history_seed_buffer.clone();
            let bytes = self.rewrite_current_line_bytes(&self.input_buffer);
            let _ = write_to_session(session, &bytes);
            return true;
        } else {
            current - 1
        };

        self.history_nav = Some(next);
        let command = self.history_matches[next].clone();
        self.input_buffer = command.clone();
        let bytes = self.rewrite_current_line_bytes(&command);
        let _ = write_to_session(session, &bytes);
        true
    }

    fn rewrite_current_line_bytes(&self, command: &str) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(command.len() + 2);
        // Ctrl+U clears current prompt line for common shell keymaps.
        bytes.push(0x15);
        bytes.extend_from_slice(command.as_bytes());
        bytes
    }

    fn handle_local_path_completion(
        &mut self,
        session: &mut Option<Box<dyn AsyncSession>>,
    ) -> bool {
        let Some(token) = self.current_token() else {
            return false;
        };
        // Local fallback completion should only use local process cwd; do not
        // depend on remote OSC cwd to avoid accidental blocking path probes.
        let cwd = std::env::current_dir()
            .ok()
            .and_then(|p| p.to_str().map(str::to_string))
            .unwrap_or_else(|| ".".to_string());
        let (base_dir, prefix) = resolve_completion_input(&cwd, token);
        let Ok(entries) = std::fs::read_dir(&base_dir) else {
            return false;
        };

        let mut candidates: Vec<(String, bool)> = Vec::new();
        const MAX_SCAN_ENTRIES: usize = 512;
        for entry in entries.flatten().take(MAX_SCAN_ENTRIES) {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with(&prefix) {
                continue;
            }
            let is_dir = entry.file_type().ok().is_some_and(|ft| ft.is_dir());
            candidates.push((name, is_dir));
        }
        candidates.sort_by(|a, b| a.0.cmp(&b.0));

        if candidates.is_empty() {
            return false;
        }

        let completion = if candidates.len() == 1 {
            let (name, is_dir) = &candidates[0];
            let mut s = name[prefix.len()..].to_string();
            if *is_dir {
                s.push('/');
            } else {
                s.push(' ');
            }
            s
        } else {
            let names: Vec<&str> = candidates.iter().map(|(n, _)| n.as_str()).collect();
            let common = common_prefix(&names);
            if common.len() <= prefix.len() {
                return false;
            }
            common[prefix.len()..].to_string()
        };

        self.input_buffer.push_str(&completion);
        let mut bytes = Vec::with_capacity(completion.len());
        bytes.extend_from_slice(completion.as_bytes());
        let _ = write_to_session(session, &bytes);

        true
    }

    fn update_local_input_state_on_key(&mut self, key: egui::Key) {
        match key {
            egui::Key::Backspace => {
                self.input_buffer.pop();
                self.reset_history_nav();
            }
            egui::Key::Enter => {
                let cmd = self.input_buffer.trim().to_string();
                if !cmd.is_empty() && self.command_history.last().is_none_or(|last| last != &cmd) {
                    self.command_history.push(cmd);
                    const MAX_HISTORY: usize = 2000;
                    if self.command_history.len() > MAX_HISTORY {
                        let excess = self.command_history.len() - MAX_HISTORY;
                        self.command_history.drain(..excess);
                    }
                }
                self.input_buffer.clear();
                self.reset_history_nav();
            }
            egui::Key::Escape => {
                self.reset_history_nav();
            }
            _ => {}
        }
    }

    fn update_local_input_state_on_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if text.contains('\n') || text.contains('\r') {
            for line in text.lines() {
                let cmd = line.trim();
                if !cmd.is_empty() {
                    self.command_history.push(cmd.to_string());
                }
            }
            self.input_buffer.clear();
            self.reset_history_nav();
            return;
        }
        self.input_buffer.push_str(text);
        self.reset_history_nav();
    }

    fn update_local_input_state_on_paste(&mut self, text: &str) {
        if text.contains('\n') || text.contains('\r') {
            self.input_buffer.clear();
            self.reset_history_nav();
            return;
        }
        self.input_buffer.push_str(text);
        self.reset_history_nav();
    }

    fn current_token(&self) -> Option<&str> {
        let token = self.input_buffer.split_whitespace().last()?;
        if token.is_empty() || token.contains('*') || token.contains('?') {
            return None;
        }
        Some(token)
    }

    fn reset_history_nav(&mut self) {
        self.history_nav = None;
        self.history_matches.clear();
        self.history_seed_buffer.clear();
    }

    fn capture_osc_cwd_into(
        osc_pending: &mut Vec<u8>,
        cwd_hint: &mut Option<String>,
        bytes: &[u8],
    ) {
        if bytes.is_empty() {
            return;
        }

        // Fast path: most terminal chunks are plain text and do not contain OSC.
        // Avoid copying whole chunks unless we are already parsing an OSC payload
        // or we can actually see an OSC introducer.
        if osc_pending.is_empty() {
            if let Some(start) = bytes.windows(2).position(|w| w == [0x1b, b']']) {
                osc_pending.extend_from_slice(&bytes[start..]);
            } else if bytes.last() == Some(&0x1b) {
                // Keep a split ESC to join with a following `]` in the next chunk.
                osc_pending.push(0x1b);
                return;
            } else {
                return;
            }
        } else {
            // If previous chunk ended with a lone ESC but next chunk doesn't
            // continue OSC, discard the stale marker and re-check this chunk.
            if osc_pending.len() == 1 && osc_pending[0] == 0x1b && bytes[0] != b']' {
                osc_pending.clear();
                if let Some(start) = bytes.windows(2).position(|w| w == [0x1b, b']']) {
                    osc_pending.extend_from_slice(&bytes[start..]);
                } else if bytes.last() == Some(&0x1b) {
                    osc_pending.push(0x1b);
                    return;
                } else {
                    return;
                }
            } else {
                osc_pending.extend_from_slice(bytes);
            }
        }

        let mut idx = 0usize;
        while idx + 1 < osc_pending.len() {
            let rel_start = osc_pending[idx..]
                .windows(2)
                .position(|w| w == [0x1b, b']']);
            let Some(start_off) = rel_start else {
                break;
            };
            let start = idx + start_off;
            let payload_start = start + 2;
            let mut end = None;
            let mut term_len = 0usize;
            let mut j = payload_start;
            while j < osc_pending.len() {
                if osc_pending[j] == 0x07 {
                    end = Some(j);
                    term_len = 1;
                    break;
                }
                if j + 1 < osc_pending.len()
                    && osc_pending[j] == 0x1b
                    && osc_pending[j + 1] == b'\\'
                {
                    end = Some(j);
                    term_len = 2;
                    break;
                }
                j += 1;
            }

            let Some(end_idx) = end else {
                if start > 0 {
                    osc_pending.drain(..start);
                }
                if osc_pending.len() > 8192 {
                    let keep_from = osc_pending.len() - 8192;
                    osc_pending.drain(..keep_from);
                }
                return;
            };

            if let Ok(payload) = std::str::from_utf8(&osc_pending[payload_start..end_idx]) {
                match parse_cwd_from_osc_payload(payload) {
                    OscCwdUpdate::Set(cwd) => *cwd_hint = Some(cwd),
                    OscCwdUpdate::Clear => *cwd_hint = None,
                    OscCwdUpdate::Noop => {}
                }
            }
            idx = end_idx + term_len;
        }

        if idx > 0 {
            osc_pending.drain(..idx);
        } else if osc_pending.len() > 8192 {
            let keep_from = osc_pending.len() - 8192;
            osc_pending.drain(..keep_from);
        }
    }
}

fn resolve_completion_input<'a>(
    cwd: &str,
    token: &'a str,
) -> (PathBuf, String) {
    let expanded = if token == "~" || token.starts_with("~/") {
        std::env::var("HOME")
            .ok()
            .map(|home| {
                if token == "~" {
                    home
                } else {
                    format!("{home}/{}", &token[2..])
                }
            })
            .unwrap_or_else(|| token.to_string())
    } else {
        token.to_string()
    };
    let path = Path::new(&expanded);
    let (base_dir, prefix) = if expanded.ends_with('/') {
        (normalize_base_dir(cwd, path), String::new())
    } else {
        let parent = path.parent().unwrap_or_else(|| Path::new(""));
        let base_dir = if parent.as_os_str().is_empty() {
            PathBuf::from(cwd)
        } else {
            normalize_base_dir(cwd, parent)
        };
        let prefix = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        (base_dir, prefix)
    };
    (base_dir, prefix)
}

fn normalize_base_dir(cwd: &str, raw: &Path) -> PathBuf {
    if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        Path::new(cwd).join(raw)
    }
}

fn common_prefix(names: &[&str]) -> String {
    let Some(first) = names.first() else {
        return String::new();
    };
    let mut end = first.len();
    for name in names.iter().skip(1) {
        end = end.min(name.len());
        let bytes_a = first.as_bytes();
        let bytes_b = name.as_bytes();
        let mut i = 0usize;
        while i < end && bytes_a[i] == bytes_b[i] {
            i += 1;
        }
        end = i;
        if end == 0 {
            break;
        }
    }
    first[..end].to_string()
}

enum OscCwdUpdate {
    Set(String),
    Clear,
    Noop,
}

fn parse_cwd_from_osc_payload(payload: &str) -> OscCwdUpdate {
    if let Some(rest) = payload.strip_prefix("7;") {
        return extract_cwd_from_osc7(rest)
            .map(OscCwdUpdate::Set)
            .unwrap_or(OscCwdUpdate::Noop);
    }

    if let Some(rest) = payload.strip_prefix("633;") {
        for part in rest.split(';') {
            if let Some(cwd) = part.strip_prefix("Cwd=") {
                let decoded = percent_decode(cwd);
                if decoded.is_empty() {
                    return OscCwdUpdate::Clear;
                }
                return OscCwdUpdate::Set(decoded);
            }
        }
        if let Some(cwd) = rest.strip_prefix("P;Cwd=") {
            let decoded = percent_decode(cwd);
            if decoded.is_empty() {
                return OscCwdUpdate::Clear;
            }
            return OscCwdUpdate::Set(decoded);
        }
    }

    OscCwdUpdate::Noop
}

fn extract_cwd_from_osc7(rest: &str) -> Option<String> {
    let uri = rest.trim();
    let without_scheme = uri.strip_prefix("file://")?;
    let slash = without_scheme.find('/').unwrap_or(without_scheme.len());
    let path = if slash >= without_scheme.len() {
        "/"
    } else {
        &without_scheme[slash..]
    };
    let decoded = percent_decode(path);
    if decoded.is_empty() {
        None
    } else {
        Some(decoded)
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h1 = bytes[i + 1];
            let h2 = bytes[i + 2];
            let hex = |c: u8| -> Option<u8> {
                match c {
                    b'0'..=b'9' => Some(c - b'0'),
                    b'a'..=b'f' => Some(c - b'a' + 10),
                    b'A'..=b'F' => Some(c - b'A' + 10),
                    _ => None,
                }
            };
            if let (Some(a), Some(b)) = (hex(h1), hex(h2)) {
                out.push((a << 4) | b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_view_initialization() {
        let view = TerminalView::new();
        assert!(view.last_cols == 0);
        assert!(view.last_rows == 0);
    }
}
