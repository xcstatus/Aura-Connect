use iced::Task;
use iced::clipboard;
use iced::keyboard::key::{Named, Physical};
use iced::keyboard::{Key, Modifiers};
use iced::mouse::ScrollDelta;

use crate::terminal_core::{TerminalKey, TerminalModifiers};
use crate::theme::layout::terminal_scroll_hit_exclude_right_px;

use super::engine_adapter::EngineAdapterMut;
use super::message::Message;
use super::state::IcedState;
use super::terminal_event::TerminalEvent;
use super::terminal_viewport;

/// Thin bridge between Iced UI events and terminal core/controller.
///
/// Keeps VT/protocol branching out of `update.rs` so the bridge can be swapped later
/// (document 4.1: "薄封装层 / 可替换桥接层").
pub(crate) struct TerminalHost;

impl TerminalHost {
    #[inline]
    fn with_active_session_and_engine<R>(
        state: &mut IcedState,
        f: impl FnOnce(
            &mut dyn crate::backend::ssh_session::AsyncSession,
            &mut EngineAdapterMut<'_>,
        ) -> R,
    ) -> Option<R> {
        let tab = state.active_tab;
        let session = state.session_manager.session_mut(tab)?;
        Some(f(session, &mut EngineAdapterMut::new(&mut state.tab_panes[tab].terminal)))
    }

    #[inline]
    fn write_if_any(
        session: &mut dyn crate::backend::ssh_session::AsyncSession,
        bytes: Vec<u8>,
    ) -> bool {
        if bytes.is_empty() {
            return false;
        }
        let _ = session.write_stream(&bytes);
        true
    }

    #[inline]
    fn write_opt(
        session: &mut dyn crate::backend::ssh_session::AsyncSession,
        bytes: Option<Vec<u8>>,
    ) -> bool {
        if let Some(bytes) = bytes {
            let _ = session.write_stream(&bytes);
            return true;
        }
        false
    }

    fn dispatch_key_event_to_engine(
        session: &mut dyn crate::backend::ssh_session::AsyncSession,
        engine: &mut EngineAdapterMut<'_>,
        key: &Key,
        physical_key: Physical,
        modifiers: Modifiers,
        text: Option<&str>,
    ) -> bool {
        let mods_tm = terminal_mods(modifiers);

        // 1) IME / CJK committed text first.
        if let Some(t) = text {
            if !t.is_empty()
                && !t.is_ascii()
                && !modifiers.control()
                && !modifiers.logo()
                && !t.chars().all(char::is_control)
                && Self::write_opt(session, engine.encode_text_raw_committed_utf8(t))
            {
                return true;
            }
        }

        // 2) Physical key path (preferred for protocol correctness).
        if let Physical::Code(code) = physical_key {
            if Self::write_opt(
                session,
                engine.encode_keypress_from_physical(code, mods_tm, text),
            ) {
                return true;
            }
        }

        // 3) Named key fallback through controller path.
        if let Some(tk) = named_terminal_key(key) {
            let _ = engine.on_named_key(session, tk, mods_tm);
            return true;
        }

        // 4) Text fallback path.
        if let Some(t) = text {
            let chord = modifiers.control() || modifiers.logo();
            if !chord && !t.chars().all(char::is_control) {
                let bytes = if t.is_ascii() {
                    engine
                        .encode_unidentified_text_keypress(t)
                        .or_else(|| engine.encode_text_raw_fallback_whitelisted(t))
                } else {
                    // Non-ASCII committed text should already have been handled above.
                    engine.encode_text_raw_committed_utf8(t)
                };
                return Self::write_opt(session, bytes);
            }
        }
        false
    }

    #[inline]
    fn input_blocked_by_modal(state: &IcedState) -> bool {
        state.quick_connect_open || state.settings_modal_open
    }

    /// Whether terminal should receive keyboard/paste input (PTY) at all.
    #[inline]
    fn should_capture_pty_input(state: &IcedState) -> bool {
        !Self::input_blocked_by_modal(state) && state.model.terminal_capture_mode
    }

    #[inline]
    fn effective_terminal_focused(state: &IcedState) -> bool {
        Self::should_capture_pty_input(state) && state.window_focused
    }

    pub(crate) fn sync_focus_report_for_tab(state: &mut IcedState, tab: usize, eff: bool) {
        let Some(pane) = state.tab_panes.get_mut(tab) else {
            return;
        };
        if pane.last_terminal_focus_sent == Some(eff) {
            return;
        }
        if let Some(session) = state.session_manager.session_mut(tab) {
            let mut engine = EngineAdapterMut::new(&mut pane.terminal);
            let _ = Self::write_if_any(session, engine.encode_focus_event(eff));
        }
        pane.last_terminal_focus_sent = Some(eff);
    }

    pub(crate) fn sync_focus_report(state: &mut IcedState) {
        let eff = Self::effective_terminal_focused(state);
        Self::sync_focus_report_for_tab(state, state.active_tab, eff);
    }

    pub(crate) fn handle_event(state: &mut IcedState, ev: TerminalEvent) -> Task<Message> {
        let now = crate::settings::unix_time_ms();
        let mut activity = false;

        match ev {
            TerminalEvent::FocusChanged(eff) => {
                // Focus reporting is protocol-visible but still must follow modal/capture gating.
                let eff = eff && Self::should_capture_pty_input(state);
                Self::sync_focus_report_for_tab(state, state.active_tab, eff);
            }
            TerminalEvent::Paste(t) => {
                if !Self::should_capture_pty_input(state) || t.is_empty() {
                    return Task::none();
                }
                let wrote = Self::with_active_session_and_engine(state, |session, engine| {
                    Self::write_if_any(session, engine.encode_paste(&t))
                })
                .unwrap_or(false);
                activity |= wrote;
            }
            TerminalEvent::KeyPressed {
                key,
                physical_key,
                modifiers,
                text,
            } => {
                if !Self::should_capture_pty_input(state) {
                    return Task::none();
                }
                if copy_shortcut(&key, modifiers) {
                    let pane = state.active_pane_mut();
                    let mut engine = EngineAdapterMut::new(&mut pane.terminal);
                    let text = if let Some((a, b)) = engine.selection_range() {
                        engine.selection_text_for_copy(a, b)
                    } else {
                        engine.visible_text_for_copy()
                    };
                    return clipboard::write(text).map(|_: ()| Message::ClipboardWriteDone);
                }
                if paste_shortcut(&key, modifiers) {
                    return clipboard::read().map(Message::ClipboardPaste);
                }

                let wrote = Self::with_active_session_and_engine(state, |session, engine| {
                    Self::dispatch_key_event_to_engine(
                        session,
                        engine,
                        &key,
                        physical_key,
                        modifiers,
                        text.as_deref(),
                    )
                })
                .unwrap_or(false);
                activity |= wrote;
            }
            TerminalEvent::MouseLeftDown(p) => {
                if Self::input_blocked_by_modal(state) {
                    return Task::none();
                }
                // Don't allow selection on an empty/unconnected terminal pane.
                if !state.session_manager.has_session(state.active_tab) {
                    return Task::none();
                }
                state.last_cursor_pos = Some(p);
                let spec = terminal_viewport::terminal_viewport_spec_for_settings(
                    &state.model.settings.terminal,
                );
                let (cols, rows): (u16, u16) = {
                    let pane = state.active_pane_mut();
                    let engine = EngineAdapterMut::new(&mut pane.terminal);
                    engine.grid_size()
                };
                let (x0, y0, w, h) =
                    terminal_viewport::terminal_scroll_area_rect(state.window_size, &spec);
                let local_x = p.x - x0;
                let local_y = p.y - y0;
                let ch = spec.term_cell_h().max(1.0);
                let metric_cw = spec.term_cell_w().max(1.0);
                let gutter = terminal_scroll_hit_exclude_right_px();
                let exclude_r = if w > gutter + 8.0 && metric_cw > 2.0 {
                    gutter.min((metric_cw - 1.0).max(0.0))
                } else {
                    0.0
                };
                let w_map = (w - exclude_r).max(1.0);
                let cell_w = w_map / cols.max(1) as f32;
                let content_h = rows as f32 * ch;
                let slack_y = (h - content_h).max(0.0);

                if let Some((c, r)) = terminal_viewport::window_point_to_grid_with_dims(
                    state.window_size,
                    &spec,
                    p,
                    cols,
                    rows,
                ) {
                    let ideal_row_f = (local_y / ch).floor();
                    let ideal_col_f = (local_x / cell_w).floor();
                    let hr = if rows > 0 { h / (rows as f32) } else { 0.0 };
                    if crate::term_diag::enabled("HIT_TEST") {
                        log::debug!(
                            target: "term_hit_test",
                            "MouseLeftDown hit p=({:.2},{:.2}) origin=({:.2},{:.2}) local=({:.2},{:.2}) cell=({},{}) ideal_f=({:.2},{:.2}) cell_w={:.2} metric_cw={:.2} ch={:.2} rect_wh=({:.2},{:.2}) h/rows={:.2} rows*ch={:.2} slack_y={:.2}",
                            p.x,
                            p.y,
                            x0,
                            y0,
                            local_x,
                            local_y,
                            c,
                            r,
                            ideal_col_f,
                            ideal_row_f,
                            cell_w,
                            metric_cw,
                            ch,
                            w,
                            h,
                            hr,
                            content_h,
                            slack_y,
                        );
                    }
                    let pane = state.active_pane_mut();
                    // Click arms selection; highlight only after drag movement.
                    let mut engine = EngineAdapterMut::new(&mut pane.terminal);
                    engine.selection_begin((c, r));
                } else if local_x >= 0.0 && local_y >= 0.0 && local_x < w && local_y < h {
                    if crate::term_diag::enabled("HIT_TEST") {
                        log::debug!(
                            target: "term_hit_test",
                            "MouseLeftDown miss inside rect p=({:.2},{:.2}) local=({:.2},{:.2}) (e.g. scrollbar strip) rect_wh=({:.2},{:.2})",
                            p.x,
                            p.y,
                            local_x,
                            local_y,
                            w,
                            h,
                        );
                    }
                }
            }
            TerminalEvent::MouseMoved(p) => {
                state.last_cursor_pos = Some(p);
                let spec = terminal_viewport::terminal_viewport_spec_for_settings(
                    &state.model.settings.terminal,
                );
                let window = state.window_size;
                let (cols, rows): (u16, u16) = {
                    let pane = state.active_pane_mut();
                    let engine = EngineAdapterMut::new(&mut pane.terminal);
                    engine.grid_size()
                };
                if let Some((c, r)) =
                    terminal_viewport::window_point_to_grid_with_dims(window, &spec, p, cols, rows)
                {
                    let pane = state.active_pane_mut();
                    // Start selection only after the pointer actually moved to a different cell.
                    let mut engine = EngineAdapterMut::new(&mut pane.terminal);
                    engine.selection_update((c, r));
                }
            }
            TerminalEvent::MouseLeftUp(p) => {
                state.last_cursor_pos = Some(p);
                let keep = state.model.settings.terminal.keep_selection_highlight;
                let pane = state.active_pane_mut();
                let mut engine = EngineAdapterMut::new(&mut pane.terminal);
                engine.selection_end(keep);
            }
            TerminalEvent::MouseRightClick(p) => {
                state.last_cursor_pos = Some(p);
                if !Self::should_capture_pty_input(state) {
                    return Task::none();
                }
                if !state.model.settings.terminal.right_click_paste {
                    return Task::none();
                }
                let spec = terminal_viewport::terminal_viewport_spec_for_settings(
                    &state.model.settings.terminal,
                );
                let (cols, rows): (u16, u16) = {
                    let pane = state.active_pane_mut();
                    let engine = EngineAdapterMut::new(&mut pane.terminal);
                    engine.grid_size()
                };
                if terminal_viewport::window_point_to_grid_with_dims(
                    state.window_size,
                    &spec,
                    p,
                    cols,
                    rows,
                )
                .is_none()
                {
                    return Task::none();
                }
                // Reuse clipboard paste pipeline.
                return clipboard::read().map(Message::ClipboardPaste);
            }
            TerminalEvent::MouseWheel { delta, at } => {
                if Self::input_blocked_by_modal(state) {
                    return Task::none();
                }
                state.last_cursor_pos = Some(at);
                let spec = terminal_viewport::terminal_viewport_spec_for_settings(
                    &state.model.settings.terminal,
                );
                let (cols, rows): (u16, u16) = {
                    let pane = state.active_pane_mut();
                    let engine = EngineAdapterMut::new(&mut pane.terminal);
                    engine.grid_size()
                };
                if terminal_viewport::window_point_to_grid_with_dims(
                    state.window_size,
                    &spec,
                    at,
                    cols,
                    rows,
                )
                .is_none()
                {
                    return Task::none();
                }

                // Policy (P0 usability): allow "scrollback + select + copy".
                // Scrolling ends active drag, but keeps the existing selection highlight so copy remains WYSIWYG.
                {
                    let pane = state.active_pane_mut();
                    let mut engine = EngineAdapterMut::new(&mut pane.terminal);
                    engine.selection_end_drag_keep();
                }

                let ch = spec.term_cell_h().max(1.0);
                let delta_rows: isize = match delta {
                    ScrollDelta::Lines { y, .. } => (y as isize) * -3,
                    ScrollDelta::Pixels { y, .. } => ((y / ch).round() as isize) * -1,
                };
                if delta_rows != 0 {
                    let pane = state.active_pane_mut();
                    let mut engine = EngineAdapterMut::new(&mut pane.terminal);
                    engine.scroll_viewport_delta_rows(delta_rows);
                }
            }
        }

        if activity {
            state.last_activity_ms = now;
        }
        Task::none()
    }
}

fn terminal_mods(m: Modifiers) -> TerminalModifiers {
    TerminalModifiers {
        shift: m.shift(),
        ctrl: m.control(),
        alt: m.alt(),
        logo: m.logo(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::ssh_session::AsyncSession;
    use iced::Point;
    use iced::mouse::ScrollDelta;

    #[derive(Default)]
    struct DummySession {
        writes: Vec<u8>,
        connected: bool,
    }

    impl AsyncSession for DummySession {
        fn read_stream(&mut self, _buffer: &mut [u8]) -> anyhow::Result<usize> {
            Ok(0)
        }
        fn write_stream(&mut self, data: &[u8]) -> anyhow::Result<()> {
            self.writes.extend_from_slice(data);
            Ok(())
        }
        fn resize_pty(&mut self, _cols: u16, _rows: u16) -> anyhow::Result<()> {
            Ok(())
        }
        fn is_connected(&self) -> bool {
            self.connected
        }
    }

    fn attach_dummy_session(state: &mut IcedState) {
        state
            .session_manager
            .attach_session(state.active_tab, Box::new(DummySession {
                writes: Vec::new(),
                connected: true,
            }));
    }

    #[test]
    fn scroll_ends_drag_but_keeps_selection() {
        let (mut state, _t) = crate::iced_app::state::boot();
        attach_dummy_session(&mut state);

        {
            let pane = state.active_pane_mut();
            pane.terminal.selection_mut().range = Some(((1, 1), (5, 3)));
            pane.terminal.selection_mut().selecting = true;
        }

        let window = state.window_size;
        let spec =
            terminal_viewport::terminal_viewport_spec_for_settings(&state.model.settings.terminal);
        let (x0, y0, w, h) = terminal_viewport::terminal_scroll_area_rect(window, &spec);
        let at = Point::new(x0 + w * 0.5, y0 + h * 0.5);

        let _ = TerminalHost::handle_event(
            &mut state,
            TerminalEvent::MouseWheel {
                delta: ScrollDelta::Lines { x: 0.0, y: -1.0 },
                at,
            },
        );

        let pane = state.active_pane();
        assert!(
            !pane.terminal.selection.is_selecting(),
            "scroll must end drag selecting"
        );
        assert!(
            pane.terminal.selection_range().is_some(),
            "scroll keeps selection so copy remains WYSIWYG in scrollback"
        );
    }

    #[test]
    fn click_without_drag_clears_existing_selection() {
        let (mut state, _t) = crate::iced_app::state::boot();
        attach_dummy_session(&mut state);
        state.active_pane_mut().terminal.selection_mut().range = Some(((2, 2), (6, 4)));

        let window = state.window_size;
        let spec =
            terminal_viewport::terminal_viewport_spec_for_settings(&state.model.settings.terminal);
        let (x0, y0, w, h) = terminal_viewport::terminal_scroll_area_rect(window, &spec);
        let p = Point::new(x0 + w * 0.4, y0 + h * 0.3);

        let _ = TerminalHost::handle_event(&mut state, TerminalEvent::MouseLeftDown(p));
        let _ = TerminalHost::handle_event(&mut state, TerminalEvent::MouseLeftUp(p));

        assert!(
            state.active_pane().terminal.selection_range().is_none(),
            "single click should clear previous selection"
        );
    }

    #[test]
    fn drag_starts_selection_after_anchor_move() {
        let (mut state, _t) = crate::iced_app::state::boot();
        attach_dummy_session(&mut state);

        let window = state.window_size;
        let spec =
            terminal_viewport::terminal_viewport_spec_for_settings(&state.model.settings.terminal);
        let (x0, y0, w, h) = terminal_viewport::terminal_scroll_area_rect(window, &spec);
        let p0 = Point::new(x0 + w * 0.20, y0 + h * 0.20);
        let p1 = Point::new(x0 + w * 0.35, y0 + h * 0.32);

        let _ = TerminalHost::handle_event(&mut state, TerminalEvent::MouseLeftDown(p0));
        assert!(
            state.active_pane().terminal.selection_range().is_none(),
            "anchor only, no visible selection yet"
        );
        let _ = TerminalHost::handle_event(&mut state, TerminalEvent::MouseMoved(p1));
        let _ = TerminalHost::handle_event(&mut state, TerminalEvent::MouseLeftUp(p1));

        assert!(
            state.active_pane().terminal.selection_range().is_some(),
            "drag should create selection range"
        );
    }
}

fn named_terminal_key(key: &Key) -> Option<TerminalKey> {
    match key {
        Key::Named(Named::Enter) => Some(TerminalKey::Enter),
        Key::Named(Named::Backspace) => Some(TerminalKey::Backspace),
        Key::Named(Named::Tab) => Some(TerminalKey::Tab),
        Key::Named(Named::Escape) => Some(TerminalKey::Escape),
        Key::Named(Named::Space) => Some(TerminalKey::Space),
        Key::Named(Named::ArrowUp) => Some(TerminalKey::ArrowUp),
        Key::Named(Named::ArrowDown) => Some(TerminalKey::ArrowDown),
        Key::Named(Named::ArrowLeft) => Some(TerminalKey::ArrowLeft),
        Key::Named(Named::ArrowRight) => Some(TerminalKey::ArrowRight),
        Key::Named(Named::Home) => Some(TerminalKey::Home),
        Key::Named(Named::End) => Some(TerminalKey::End),
        Key::Named(Named::PageUp) => Some(TerminalKey::PageUp),
        Key::Named(Named::PageDown) => Some(TerminalKey::PageDown),
        Key::Named(Named::Insert) => Some(TerminalKey::Insert),
        Key::Named(Named::Delete) => Some(TerminalKey::Delete),
        Key::Named(Named::F1) => Some(TerminalKey::Function(1)),
        Key::Named(Named::F2) => Some(TerminalKey::Function(2)),
        Key::Named(Named::F3) => Some(TerminalKey::Function(3)),
        Key::Named(Named::F4) => Some(TerminalKey::Function(4)),
        Key::Named(Named::F5) => Some(TerminalKey::Function(5)),
        Key::Named(Named::F6) => Some(TerminalKey::Function(6)),
        Key::Named(Named::F7) => Some(TerminalKey::Function(7)),
        Key::Named(Named::F8) => Some(TerminalKey::Function(8)),
        Key::Named(Named::F9) => Some(TerminalKey::Function(9)),
        Key::Named(Named::F10) => Some(TerminalKey::Function(10)),
        Key::Named(Named::F11) => Some(TerminalKey::Function(11)),
        Key::Named(Named::F12) => Some(TerminalKey::Function(12)),
        Key::Named(Named::F13) => Some(TerminalKey::Function(13)),
        Key::Named(Named::F14) => Some(TerminalKey::Function(14)),
        Key::Named(Named::F15) => Some(TerminalKey::Function(15)),
        Key::Named(Named::F16) => Some(TerminalKey::Function(16)),
        Key::Named(Named::F17) => Some(TerminalKey::Function(17)),
        Key::Named(Named::F18) => Some(TerminalKey::Function(18)),
        Key::Named(Named::F19) => Some(TerminalKey::Function(19)),
        Key::Named(Named::F20) => Some(TerminalKey::Function(20)),
        Key::Named(Named::F21) => Some(TerminalKey::Function(21)),
        Key::Named(Named::F22) => Some(TerminalKey::Function(22)),
        Key::Named(Named::F23) => Some(TerminalKey::Function(23)),
        Key::Named(Named::F24) => Some(TerminalKey::Function(24)),
        Key::Named(Named::F25) => Some(TerminalKey::Function(25)),
        _ => None,
    }
}

fn paste_shortcut(key: &Key, modifiers: Modifiers) -> bool {
    match key {
        Key::Named(Named::Paste) => true,
        Key::Character(s) => {
            s.chars()
                .next()
                .is_some_and(|c| c.eq_ignore_ascii_case(&'v'))
                && (modifiers.control() || modifiers.logo())
        }
        _ => false,
    }
}

fn copy_shortcut(key: &Key, modifiers: Modifiers) -> bool {
    match key {
        Key::Named(Named::Copy) => true,
        Key::Character(s) => {
            s.chars()
                .next()
                .is_some_and(|c| c.eq_ignore_ascii_case(&'c'))
                && (modifiers.control() || modifiers.logo())
        }
        _ => false,
    }
}
