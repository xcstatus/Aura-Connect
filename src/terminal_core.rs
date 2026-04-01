use std::os::raw::c_char;

use anyhow::Result;
use iced::keyboard::key::Code;

use crate::backend::ghostty_vt::{ffi, CursorState, GhosttyVtTerminal, ScrollbarState, VtStyledRow};
use crate::backend::ssh_session::AsyncSession;
use crate::settings::{TerminalPlainTextUpdate, TerminalRenderMode, TerminalSettings};
use crate::terminal_selection::TerminalSelection;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScrollState {
    /// Total scrollback rows (including visible viewport).
    pub total_rows: u64,
    /// Current top row offset into the scrollback.
    pub offset_rows: u64,
    /// Visible viewport height in rows.
    pub viewport_rows: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EngineSnapshot {
    pub cols: u16,
    pub rows: u16,
    pub cursor: Option<(u16, u16)>,
    pub scroll: ScrollState,
    pub selection: Option<((u16, u16), (u16, u16))>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalKey {
    Enter,
    Backspace,
    Tab,
    Escape,
    Space,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    /// `n` in 1..=25 → `F1`..=`F25` for libghostty.
    Function(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerminalModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub logo: bool,
}

pub struct TerminalSessionBridge {
    buf: [u8; 4096],
    pending: Vec<u8>,
    pending_off: usize,
    max_bytes_per_frame: usize,
    max_reads_per_frame: usize,
}

impl Default for TerminalSessionBridge {
    fn default() -> Self {
        Self {
            buf: [0; 4096],
            pending: Vec::new(),
            pending_off: 0,
            max_bytes_per_frame: 32 * 1024,
            max_reads_per_frame: 64,
        }
    }
}

impl TerminalSessionBridge {
    pub fn drain_output(
        &mut self,
        session: &mut dyn AsyncSession,
        mut on_bytes: impl FnMut(&[u8]),
    ) -> Result<usize> {
        let mut total = 0usize;
        if self.pending_off < self.pending.len() {
            let rem = &self.pending[self.pending_off..];
            let take = rem.len().min(self.max_bytes_per_frame);
            if take > 0 {
                on_bytes(&rem[..take]);
                total += take;
                self.pending_off += take;
            }
            if self.pending_off >= self.pending.len() {
                self.pending.clear();
                self.pending_off = 0;
            }
        }

        for _ in 0..self.max_reads_per_frame {
            if total >= self.max_bytes_per_frame {
                break;
            }
            let n = session.read_stream(&mut self.buf)?;
            if n == 0 {
                break;
            }
            let left = self.max_bytes_per_frame - total;
            if n <= left {
                on_bytes(&self.buf[..n]);
                total += n;
            } else {
                on_bytes(&self.buf[..left]);
                total += left;
                self.pending.clear();
                self.pending.extend_from_slice(&self.buf[left..n]);
                self.pending_off = 0;
                break;
            }
        }
        Ok(total)
    }

    /// Best-effort resize propagation to remote PTY.
    ///
    /// **SSOT note:** Iced app must size PTY only through
    /// `terminal_viewport::{terminal_viewport_spec_for_settings, grid_from_window_size_with_spec}`
    /// then call [`TerminalController::resize_and_sync_pty`]. Avoid ad-hoc `resize_pty` calls.
    pub fn resize_pty_if_needed(
        &mut self,
        session: &mut dyn AsyncSession,
        last: &mut (u16, u16),
        cols: u16,
        rows: u16,
    ) -> Result<bool> {
        let cols = cols.max(1);
        let rows = rows.max(1);
        if *last == (cols, rows) {
            return Ok(false);
        }
        session.resize_pty(cols, rows)?;
        *last = (cols, rows);
        Ok(true)
    }
}

pub struct TerminalController {
    vt: GhosttyVtTerminal,
    bridge: TerminalSessionBridge,
    lines: Vec<String>,
    styled_rows: Vec<VtStyledRow>,
    styled_row_gen: Vec<u64>,
    styled_dirty_rows_tmp: Vec<usize>,
    styled_fragments: Vec<Vec<StyledFragment>>,
    pub(crate) selection: TerminalSelection,
    last_pty_size: (u16, u16),
    cols: u16,
    rows: u16,
    render_mode: TerminalRenderMode,
    /// User preference: only wrap paste when remote DEC 2004 is on (see [`Self::encode_paste`]).
    bracketed_paste: bool,
    cursor_cache: Option<CursorState>,
    /// Plain mode: cached `lines.join("\n")` (UTF-8 byte offsets per row in [`Self::plain_line_byte_offsets`]).
    plain_join_cache: String,
    plain_line_byte_offsets: Vec<usize>,
    plain_text_update: TerminalPlainTextUpdate,
    scratch_plain_dirty_rows: Vec<usize>,
    plain_join_rebuilds: u64,
    key_fallback_named: u64,
    key_fallback_text: u64,
}

impl TerminalController {
    fn vt_scrollbar_state(&self) -> ScrollbarState {
        self.vt.scrollbar_state().unwrap_or_default()
    }

    /// Viewport scrollback geometry for UI: total rows, current offset, viewport height.
    pub fn scroll_state(&self) -> ScrollState {
        let sb = self.vt_scrollbar_state();
        ScrollState {
            total_rows: sb.total_rows,
            offset_rows: sb.offset_rows,
            viewport_rows: sb.viewport_rows,
        }
    }

    /// True when the viewport is not following the bottom (user is in scrollback).
    pub fn is_in_scrollback(&self) -> bool {
        let s = self.scroll_state();
        let total = s.total_rows;
        let len = s.viewport_rows.min(total);
        let bottom_off = total.saturating_sub(len);
        s.offset_rows < bottom_off
    }

    pub fn is_following_bottom(&self) -> bool {
        !self.is_in_scrollback()
    }

    fn bump_all_styled_row_gen(&mut self) {
        for g in &mut self.styled_row_gen {
            *g = g.saturating_add(1);
        }
    }

    fn rebuild_all_styled_fragments(&mut self) {
        let rows = self.styled_rows.len().min(self.styled_fragments.len());
        for y in 0..rows {
            self.styled_fragments[y] = build_row_fragments(&self.styled_rows[y]);
        }
    }

    fn rebuild_dirty_styled_fragments(&mut self, dirty_rows: &[usize]) {
        for &y in dirty_rows {
            if y < self.styled_rows.len() && y < self.styled_fragments.len() {
                self.styled_fragments[y] = build_row_fragments(&self.styled_rows[y]);
            }
        }
    }

    pub fn new(settings: &TerminalSettings) -> Self {
        let cols = 120;
        let rows = 36;
        let scrollback = settings.scrollback_limit.max(256);
        let vt = GhosttyVtTerminal::new(cols, rows, scrollback)
            .expect("failed to create ghostty terminal");
        let render_mode = settings.terminal_render_mode;
        let bracketed_paste = settings.bracketed_paste;
        let mut s = Self {
            vt,
            bridge: TerminalSessionBridge::default(),
            lines: vec![String::new(); rows as usize],
            styled_rows: vec![VtStyledRow::default(); rows as usize],
            styled_row_gen: vec![0; rows as usize],
            styled_dirty_rows_tmp: Vec::new(),
            styled_fragments: vec![Vec::new(); rows as usize],
            selection: TerminalSelection::default(),
            last_pty_size: (0, 0),
            cols,
            rows,
            render_mode,
            bracketed_paste,
            cursor_cache: None,
            plain_join_cache: String::new(),
            plain_line_byte_offsets: Vec::new(),
            plain_text_update: settings.plain_text_update,
            scratch_plain_dirty_rows: Vec::new(),
            plain_join_rebuilds: 0,
            key_fallback_named: 0,
            key_fallback_text: 0,
        };
        if render_mode == TerminalRenderMode::Plain {
            s.rebuild_plain_join_full();
        }
        s
    }

    pub fn set_bracketed_paste(&mut self, enabled: bool) {
        self.bracketed_paste = enabled;
    }

    pub fn set_plain_text_update(&mut self, mode: TerminalPlainTextUpdate) {
        if self.plain_text_update == mode {
            return;
        }
        self.plain_text_update = mode;
        if self.render_mode == TerminalRenderMode::Plain
            && mode == TerminalPlainTextUpdate::Incremental
        {
            self.rebuild_plain_join_full();
        }
    }

    /// Plain UI string when [`TerminalPlainTextUpdate::Incremental`] is enabled (kept in sync in [`Self::pump_output`] / resize / refresh).
    #[inline]
    pub fn plain_text_for_view(&self) -> &str {
        &self.plain_join_cache
    }

    /// Drain number of plain join full-rebuilds since last call.
    pub fn take_plain_join_rebuilds(&mut self) -> u64 {
        let n = self.plain_join_rebuilds;
        self.plain_join_rebuilds = 0;
        n
    }

    /// Drain key encoding fallback counts since last call.
    pub fn take_key_fallback_counts(&mut self) -> (u64, u64) {
        let n_named = self.key_fallback_named;
        let n_text = self.key_fallback_text;
        self.key_fallback_named = 0;
        self.key_fallback_text = 0;
        (n_named, n_text)
    }

    #[inline]
    pub fn render_mode(&self) -> TerminalRenderMode {
        self.render_mode
    }

    #[inline]
    pub fn grid_size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    /// Engine-side query snapshot used by UI/render adapters.
    pub fn query_snapshot(&self) -> EngineSnapshot {
        let cursor = self
            .cursor_cache
            .as_ref()
            .and_then(|c| (c.visible && c.has_pos).then_some((c.x, c.y)));
        EngineSnapshot {
            cols: self.cols,
            rows: self.rows,
            cursor,
            scroll: self.scroll_state(),
            selection: self.selection.range,
        }
    }

    #[inline]
    pub fn selection_range(&self) -> Option<((u16, u16), (u16, u16))> {
        self.selection.range
    }

    #[inline]
    pub fn selection_mut(&mut self) -> &mut TerminalSelection {
        &mut self.selection
    }

    pub fn set_render_mode(&mut self, mode: TerminalRenderMode) {
        self.render_mode = mode;
    }

    /// Pull render-state cursor (styled UI / blink). Cheap vs full snapshot.
    pub fn on_frame_tick(&mut self) {
        if self.render_mode != TerminalRenderMode::Styled {
            return;
        }
        let _ = self.vt.update_render_state();
        // Cursor policy: when the viewport is in scrollback (not following bottom),
        // hide the cursor so the screen reads like a typical terminal "paused" view.
        self.cursor_cache = if self.is_in_scrollback() {
            None
        } else {
            self.vt.cursor_state().ok()
        };
    }

    /// After changing [`TerminalRenderMode`] (e.g. settings save), rebuild local snapshots.
    pub fn refresh_terminal_snapshots(&mut self) {
        let _ = self.vt.update_render_state();
        match self.render_mode {
            TerminalRenderMode::Plain => {
                let _ = self
                    .vt
                    .update_dirty_plain_lines_and_clear_dirty(&mut self.lines);
                self.rebuild_plain_join_full();
                self.cursor_cache = None;
            }
            TerminalRenderMode::Styled => {
                self.styled_dirty_rows_tmp.clear();
                let _ = self.vt.update_dirty_styled_rows_and_clear_dirty_collect(
                    &mut self.styled_rows,
                    false,
                    Some(&mut self.styled_dirty_rows_tmp),
                );
                self.cursor_cache = if self.is_in_scrollback() {
                    None
                } else {
                    self.vt.cursor_state().ok()
                };
                self.bump_all_styled_row_gen();
                self.rebuild_all_styled_fragments();
            }
        }
    }

    /// OSC 4 palette into the **local** libghostty VT (not SSH). Refreshes snapshots so Iced styled rows match.
    pub fn apply_terminal_palette_for_scheme(&mut self, scheme: &str) {
        let bytes = crate::terminal_palette::scheme_vt_bytes(scheme);
        self.vt.write_vt(&bytes);
        self.refresh_terminal_snapshots();
    }

    /// Snapshot rows for Iced `rich_text` (viewport height = `rows`).
    #[inline]
    pub fn styled_rows(&self) -> &[VtStyledRow] {
        &self.styled_rows
    }

    #[inline]
    pub fn styled_row_generation(&self, row: usize) -> u64 {
        self.styled_row_gen.get(row).copied().unwrap_or(0)
    }

    #[inline]
    pub fn styled_row_fragments(&self, row: usize) -> &[StyledFragment] {
        self.styled_fragments.get(row).map(|v| v.as_slice()).unwrap_or(&[])
    }

    #[inline]
    pub fn cursor_snapshot(&self) -> Option<&CursorState> {
        self.cursor_cache.as_ref()
    }

    /// Resize local VT and push the same size to the SSH PTY when it changes.
    /// Returns `Ok(true)` if a new `resize_pty` was sent to the session.
    pub fn resize_and_sync_pty(
        &mut self,
        session: &mut dyn AsyncSession,
        cols: u16,
        rows: u16,
    ) -> Result<bool> {
        self.resize(cols, rows);
        self.bridge.resize_pty_if_needed(
            session,
            &mut self.last_pty_size,
            self.cols,
            self.rows,
        )
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        if self.cols == cols && self.rows == rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        let _ = self.vt.resize(cols, rows);
        let _ = self.vt.update_render_state();
        self.lines.resize(rows as usize, String::new());
        self.styled_row_gen.resize(rows as usize, 0);
        self.styled_fragments.resize_with(rows as usize, Vec::new);
        match self.render_mode {
            TerminalRenderMode::Plain => {
                let _ = self
                    .vt
                    .update_dirty_plain_lines_and_clear_dirty(&mut self.lines);
                self.rebuild_plain_join_full();
                self.cursor_cache = None;
            }
            TerminalRenderMode::Styled => {
                self.styled_dirty_rows_tmp.clear();
                let _ = self.vt.update_dirty_styled_rows_and_clear_dirty_collect(
                    &mut self.styled_rows,
                    false,
                    Some(&mut self.styled_dirty_rows_tmp),
                );
                self.cursor_cache = if self.is_in_scrollback() {
                    None
                } else {
                    self.vt.cursor_state().ok()
                };
                self.bump_all_styled_row_gen();
                self.rebuild_all_styled_fragments();
            }
        }
    }

    pub fn attach_session(&mut self, session: &mut dyn AsyncSession) -> Result<()> {
        self.bridge
            .resize_pty_if_needed(session, &mut self.last_pty_size, self.cols, self.rows)?;
        Ok(())
    }

    pub fn pump_output(&mut self, session: &mut dyn AsyncSession) -> Result<usize> {
        let n = self.bridge.drain_output(session, |bytes| self.vt.write_vt(bytes))?;
        if n > 0 {
            let _ = self.vt.update_render_state();
            match self.render_mode {
                TerminalRenderMode::Plain => {
                    match self.plain_text_update {
                        TerminalPlainTextUpdate::Full => {
                            let _ = self
                                .vt
                                .update_dirty_plain_lines_and_clear_dirty(&mut self.lines);
                            self.rebuild_plain_join_full();
                        }
                        TerminalPlainTextUpdate::Incremental => {
                            let _ = self.vt.update_dirty_plain_lines_collect_dirty_rows(
                                &mut self.lines,
                                &mut self.scratch_plain_dirty_rows,
                            );
                            if !self.scratch_plain_dirty_rows.is_empty() {
                                if self.plain_join_cache.is_empty()
                                    || self.plain_line_byte_offsets.len() != self.lines.len()
                                {
                                    self.rebuild_plain_join_full();
                                } else {
                                    let dirty =
                                        std::mem::take(&mut self.scratch_plain_dirty_rows);
                                    self.patch_plain_join_incremental(&dirty);
                                }
                            }
                        }
                    }
                }
                TerminalRenderMode::Styled => {
                    self.styled_dirty_rows_tmp.clear();
                    let _ = self.vt.update_dirty_styled_rows_and_clear_dirty_collect(
                        &mut self.styled_rows,
                        true,
                        Some(&mut self.styled_dirty_rows_tmp),
                    );
                    self.cursor_cache = if self.is_in_scrollback() {
                        None
                    } else {
                        self.vt.cursor_state().ok()
                    };
                    for &y in &self.styled_dirty_rows_tmp {
                        if let Some(g) = self.styled_row_gen.get_mut(y) {
                            *g = g.saturating_add(1);
                        }
                    }
                    let dirty = self.styled_dirty_rows_tmp.clone();
                    self.rebuild_dirty_styled_fragments(&dirty);
                }
            }
        }
        Ok(n)
    }

    /// Inject local output into the VT (system messages / connection prompts).
    ///
    /// This updates render state the same way as remote output, but without a live session.
    pub fn inject_local_output(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let _ = self.vt.write_vt(bytes);
        let _ = self.vt.update_render_state();
        match self.render_mode {
            TerminalRenderMode::Plain => {
                match self.plain_text_update {
                    TerminalPlainTextUpdate::Full => {
                        let _ = self
                            .vt
                            .update_dirty_plain_lines_and_clear_dirty(&mut self.lines);
                        self.rebuild_plain_join_full();
                    }
                    TerminalPlainTextUpdate::Incremental => {
                        let _ = self.vt.update_dirty_plain_lines_collect_dirty_rows(
                            &mut self.lines,
                            &mut self.scratch_plain_dirty_rows,
                        );
                        if !self.scratch_plain_dirty_rows.is_empty() {
                            if self.plain_join_cache.is_empty()
                                || self.plain_line_byte_offsets.len() != self.lines.len()
                            {
                                self.rebuild_plain_join_full();
                            } else {
                                let dirty = std::mem::take(&mut self.scratch_plain_dirty_rows);
                                self.patch_plain_join_incremental(&dirty);
                            }
                        }
                    }
                }
            }
            TerminalRenderMode::Styled => {
                self.styled_dirty_rows_tmp.clear();
                let _ = self.vt.update_dirty_styled_rows_and_clear_dirty_collect(
                    &mut self.styled_rows,
                    true,
                    Some(&mut self.styled_dirty_rows_tmp),
                );
                self.cursor_cache = if self.is_in_scrollback() {
                    None
                } else {
                    self.vt.cursor_state().ok()
                };
                for &y in &self.styled_dirty_rows_tmp {
                    if let Some(g) = self.styled_row_gen.get_mut(y) {
                        *g = g.saturating_add(1);
                    }
                }
                let dirty = self.styled_dirty_rows_tmp.clone();
                self.rebuild_dirty_styled_fragments(&dirty);
            }
        }
    }

    pub fn inject_local_lines(&mut self, lines: &[&str]) {
        if lines.is_empty() {
            return;
        }
        let mut s = String::new();
        for l in lines {
            s.push_str(l);
            s.push_str("\r\n");
        }
        self.inject_local_output(s.as_bytes());
    }

    /// Clear the on-screen buffer from local UI (e.g. Vault banners) before real session output.
    ///
    /// Safe when the PTY is not yet exchanging user-visible bytes: `[rustssh] …` lines are removed so
    /// they do not stack with post-unlock status and later SSH output. Also drops any active selection
    /// tied to the old grid.
    pub fn clear_local_preconnect_ui(&mut self) {
        self.selection.clear();
        // ED2: erase display; CUP: cursor to home — standard VT reset for a blank viewport.
        self.inject_local_output(b"\x1b[2J\x1b[H");
    }

    /// Scroll terminal viewport by delta rows (negative = up).
    ///
    /// This affects what the render state exposes as the current viewport; call updates snapshots so
    /// Iced rendering stays in sync.
    pub fn scroll_viewport_delta_rows(&mut self, delta_rows: isize) {
        if delta_rows == 0 {
            return;
        }
        self.vt.scroll_viewport_delta_rows(delta_rows);
        let _ = self.vt.update_render_state();
        match self.render_mode {
            TerminalRenderMode::Plain => {
                match self.plain_text_update {
                    TerminalPlainTextUpdate::Full => {
                        let _ = self
                            .vt
                            .update_dirty_plain_lines_and_clear_dirty(&mut self.lines);
                        self.rebuild_plain_join_full();
                    }
                    TerminalPlainTextUpdate::Incremental => {
                        let _ = self.vt.update_dirty_plain_lines_collect_dirty_rows(
                            &mut self.lines,
                            &mut self.scratch_plain_dirty_rows,
                        );
                        if !self.scratch_plain_dirty_rows.is_empty() {
                            if self.plain_join_cache.is_empty()
                                || self.plain_line_byte_offsets.len() != self.lines.len()
                            {
                                self.rebuild_plain_join_full();
                            } else {
                                let dirty = std::mem::take(&mut self.scratch_plain_dirty_rows);
                                self.patch_plain_join_incremental(&dirty);
                            }
                        }
                    }
                }
            }
            TerminalRenderMode::Styled => {
                self.styled_dirty_rows_tmp.clear();
                let _ = self.vt.update_dirty_styled_rows_and_clear_dirty_collect(
                    &mut self.styled_rows,
                    true,
                    Some(&mut self.styled_dirty_rows_tmp),
                );
                self.cursor_cache = if self.is_in_scrollback() {
                    None
                } else {
                    self.vt.cursor_state().ok()
                };
                for &y in &self.styled_dirty_rows_tmp {
                    if let Some(g) = self.styled_row_gen.get_mut(y) {
                        *g = g.saturating_add(1);
                    }
                }
                let dirty = self.styled_dirty_rows_tmp.clone();
                self.rebuild_dirty_styled_fragments(&dirty);
            }
        }
    }

    pub fn on_key(
        &mut self,
        session: &mut dyn AsyncSession,
        key: TerminalKey,
        modifiers: TerminalModifiers,
    ) -> Result<()> {
        let bytes = self.encode_key(key, modifiers);
        if !bytes.is_empty() {
            session.write_stream(&bytes)?;
        }
        Ok(())
    }

    /// Physical key + modifiers + optional text → libghostty bytes (single path for letters, arrows, chords).
    pub fn encode_keypress_from_physical(
        &mut self,
        code: Code,
        modifiers: TerminalModifiers,
        text: Option<&str>,
    ) -> Option<Vec<u8>> {
        let gk = code_to_ghostty_key(code)?;
        self.encode_ghostty_press_key_utf8(gk, modifiers, text)
    }

    /// IME / unknown scancode: `GHOSTTY_KEY_UNIDENTIFIED` + UTF-8 payload (DEC-style via encoder).
    pub fn encode_unidentified_text_keypress(&mut self, text: &str) -> Option<Vec<u8>> {
        if text.is_empty() || text.chars().all(|c| c.is_control()) {
            return None;
        }
        self.encode_ghostty_press_key_utf8(
            ffi::GhosttyKey_GHOSTTY_KEY_UNIDENTIFIED,
            TerminalModifiers::default(),
            Some(text),
        )
    }

    #[inline]
    fn encode_ghostty_press_key_utf8(
        &mut self,
        key: ffi::GhosttyKey,
        modifiers: TerminalModifiers,
        text: Option<&str>,
    ) -> Option<Vec<u8>> {
        let mods = mods_to_ffi(modifiers);
        let utf8 = text.and_then(|t| {
            if t.is_empty() {
                None
            } else if t.chars().all(|c| c.is_control()) {
                None
            } else {
                Some(t)
            }
        });
        let seq = self
            .vt
            .encode_key_with_utf8(
                ffi::GhosttyKeyAction_GHOSTTY_KEY_ACTION_PRESS,
                key,
                mods,
                utf8,
            )
            .ok()?;
        if seq.is_empty() {
            None
        } else {
            Some(seq)
        }
    }

    /// Clear PTY `(cols, rows)` latch so the next session always receives `resize_pty`.
    pub fn clear_pty_resize_anchor(&mut self) {
        self.last_pty_size = (0, 0);
    }

    /// Last-resort UTF-8 bytes when libghostty’s unidentified encode yields nothing.
    /// Whitelist: no control characters, bounded length — logged at `warn` (`target: term_key_fallback`).
    pub fn encode_text_raw_fallback_whitelisted(&mut self, text: &str) -> Option<Vec<u8>> {
        const MAX_BYTES: usize = 16 * 1024;
        if text.is_empty() || text.len() > MAX_BYTES {
            return None;
        }
        if !text.chars().all(|c| !c.is_control()) {
            return None;
        }
        log::warn!(
            target: "term_key_fallback",
            "text key: libghostty empty; whitelisted raw UTF-8 bytes={}",
            text.len()
        );
        self.key_fallback_text = self.key_fallback_text.saturating_add(1);
        Some(text.as_bytes().to_vec())
    }

    /// Raw UTF-8 bytes for IME/non-ASCII committed text.
    ///
    /// Same safety whitelist as [`Self::encode_text_raw_fallback_whitelisted`] but **no warn log**
    /// (IME commits are frequent; logging here creates noise).
    pub fn encode_text_raw_committed_utf8(&self, text: &str) -> Option<Vec<u8>> {
        const MAX_BYTES: usize = 16 * 1024;
        if text.is_empty() || text.len() > MAX_BYTES {
            return None;
        }
        if !text.chars().all(|c| !c.is_control()) {
            return None;
        }
        Some(text.as_bytes().to_vec())
    }

    /// DECSET 1004 focus reporting: CSI I / CSI O when the mode is enabled in the VT parser.
    pub fn encode_focus_event(&mut self, focused: bool) -> Vec<u8> {
        let mode = dec_private_mode(1004);
        let Ok(true) = self.vt.mode_get(mode) else {
            return Vec::new();
        };

        let ev = if focused {
            ffi::GhosttyFocusEvent_GHOSTTY_FOCUS_GAINED
        } else {
            ffi::GhosttyFocusEvent_GHOSTTY_FOCUS_LOST
        };

        let mut buf = [0u8; 8];
        let mut written: usize = 0;
        let res = unsafe {
            ffi::ghostty_focus_encode(
                ev,
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
                &mut written,
            )
        };
        if res == ffi::GhosttyResult_GHOSTTY_SUCCESS {
            return buf[..written].to_vec();
        }
        if res == ffi::GhosttyResult_GHOSTTY_OUT_OF_SPACE && written > 0 {
            let mut v = vec![0u8; written];
            let mut written2: usize = 0;
            let res2 = unsafe {
                ffi::ghostty_focus_encode(
                    ev,
                    v.as_mut_ptr() as *mut c_char,
                    v.len(),
                    &mut written2,
                )
            };
            if res2 == ffi::GhosttyResult_GHOSTTY_SUCCESS {
                v.truncate(written2);
                return v;
            }
        }
        Vec::new()
    }

    /// Paste: raw UTF-8 by default. Optional bracketed paste (DECSET 2004) only when the user enables
    /// it **and** the local VT state reports mode 2004 (usually after the remote sent `?2004h`).
    ///
    /// Unconditional bracketed paste is unsafe for vim and many SSH peers: the wrapper begins with
    /// ESC (`\e[200~`), which exits insert mode if the app does not consume the sequence atomically.
    pub fn encode_paste(&mut self, text: &str) -> Vec<u8> {
        let mode = dec_private_mode(2004);
        let remote_2004 = self.vt.mode_get(mode).unwrap_or(false);
        let bracketed = self.bracketed_paste && remote_2004;

        if bracketed {
            let mut out = Vec::with_capacity(text.len().saturating_add(16));
            out.extend_from_slice(b"\x1b[200~");
            out.extend_from_slice(text.as_bytes());
            out.extend_from_slice(b"\x1b[201~");
            return out;
        }
        let _safe = unsafe {
            ffi::ghostty_paste_is_safe(text.as_ptr() as *const c_char, text.len())
        };
        text.as_bytes().to_vec()
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Visible viewport text for clipboard copy.
    ///
    /// - Plain: uses the same cached/joined text as the view.
    /// - Styled: reconstructs text from `VtStyledRow` cells, respecting grid columns.
    pub fn visible_text_for_copy(&self) -> String {
        match self.render_mode {
            TerminalRenderMode::Plain => self.plain_join_cache.clone(),
            TerminalRenderMode::Styled => {
                let mut out = String::new();
                for (y, row) in self.styled_rows.iter().enumerate() {
                    if y > 0 {
                        out.push('\n');
                    }
                    if row.runs.is_empty() {
                        continue;
                    }
                    for run in &row.runs {
                        for cell in &run.cells {
                            // Continuation cells are the second half of wide glyphs; do not emit
                            // visible placeholder spaces into clipboard.
                            if cell.continuation {
                                continue;
                            }
                            if cell.text.is_empty() {
                                out.push(' ');
                            } else {
                                out.push_str(&cell.text);
                            }
                        }
                    }
                }
                out
            }
        }
    }

    fn rebuild_plain_line_offsets(&mut self) {
        let rows = self.lines.len();
        self.plain_line_byte_offsets.resize(rows, 0);
        let mut off = 0usize;
        for i in 0..rows {
            self.plain_line_byte_offsets[i] = off;
            off += self.lines[i].len();
            if i + 1 < rows {
                off += 1;
            }
        }
    }

    fn rebuild_plain_join_full(&mut self) {
        self.plain_join_rebuilds = self.plain_join_rebuilds.saturating_add(1);
        #[cfg(debug_assertions)]
        log::debug!(target: "term-plain-cache", "plain join: full rebuild rows={}", self.lines.len());
        self.plain_join_cache.clear();
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                self.plain_join_cache.push('\n');
            }
            self.plain_join_cache.push_str(line);
        }
        self.rebuild_plain_line_offsets();
    }

    /// Replace newline-delimited segments for dirty viewport rows (descending `y` so offsets stay valid).
    fn patch_plain_join_incremental(&mut self, dirty_rows: &[usize]) {
        let rows = self.lines.len();
        if rows == 0 {
            self.plain_join_cache.clear();
            self.plain_line_byte_offsets.clear();
            return;
        }
        if self.plain_line_byte_offsets.len() != rows {
            self.rebuild_plain_join_full();
            return;
        }

        let mut sorted: Vec<usize> = dirty_rows.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        if sorted.is_empty() {
            return;
        }

        for &y in sorted.iter().rev() {
            if y >= rows {
                continue;
            }
            let start = self.plain_line_byte_offsets[y];
            let end = if y + 1 < rows {
                self.plain_line_byte_offsets[y + 1]
            } else {
                self.plain_join_cache.len()
            };
            if end < start || end > self.plain_join_cache.len() {
                self.rebuild_plain_join_full();
                return;
            }
            let mut replacement =
                String::with_capacity(self.lines[y].len().saturating_add(1));
            replacement.push_str(&self.lines[y]);
            if y + 1 < rows {
                replacement.push('\n');
            }
            self.plain_join_cache.replace_range(start..end, &replacement);

            let mut off = self.plain_line_byte_offsets[y] + self.lines[y].len();
            if y + 1 < rows {
                off += 1;
            }
            for i in (y + 1)..rows {
                self.plain_line_byte_offsets[i] = off;
                off += self.lines[i].len();
                if i + 1 < rows {
                    off += 1;
                }
            }
            if off != self.plain_join_cache.len() {
                log::debug!(
                    target: "term-plain-cache",
                    "plain join offset drift after row {}; rebuilding",
                    y
                );
                self.rebuild_plain_join_full();
                return;
            }
        }
    }

    fn encode_key(&mut self, key: TerminalKey, modifiers: TerminalModifiers) -> Vec<u8> {
        let k = match key {
            TerminalKey::Enter => ffi::GhosttyKey_GHOSTTY_KEY_ENTER,
            TerminalKey::Backspace => ffi::GhosttyKey_GHOSTTY_KEY_BACKSPACE,
            TerminalKey::Tab => ffi::GhosttyKey_GHOSTTY_KEY_TAB,
            TerminalKey::Escape => ffi::GhosttyKey_GHOSTTY_KEY_ESCAPE,
            TerminalKey::Space => ffi::GhosttyKey_GHOSTTY_KEY_SPACE,
            TerminalKey::Home => ffi::GhosttyKey_GHOSTTY_KEY_HOME,
            TerminalKey::End => ffi::GhosttyKey_GHOSTTY_KEY_END,
            TerminalKey::PageUp => ffi::GhosttyKey_GHOSTTY_KEY_PAGE_UP,
            TerminalKey::PageDown => ffi::GhosttyKey_GHOSTTY_KEY_PAGE_DOWN,
            TerminalKey::Insert => ffi::GhosttyKey_GHOSTTY_KEY_INSERT,
            TerminalKey::Delete => ffi::GhosttyKey_GHOSTTY_KEY_DELETE,
            TerminalKey::ArrowUp => ffi::GhosttyKey_GHOSTTY_KEY_ARROW_UP,
            TerminalKey::ArrowDown => ffi::GhosttyKey_GHOSTTY_KEY_ARROW_DOWN,
            TerminalKey::ArrowLeft => ffi::GhosttyKey_GHOSTTY_KEY_ARROW_LEFT,
            TerminalKey::ArrowRight => ffi::GhosttyKey_GHOSTTY_KEY_ARROW_RIGHT,
            TerminalKey::Function(n) => ghostty_function_key(n)
                .unwrap_or(ffi::GhosttyKey_GHOSTTY_KEY_UNIDENTIFIED),
        };
        let mods = mods_to_ffi(modifiers);
        match self.vt.encode_key(
            ffi::GhosttyKeyAction_GHOSTTY_KEY_ACTION_PRESS,
            k,
            mods,
        ) {
            Ok(seq) if !seq.is_empty() => seq,
            Ok(_) | Err(_) => {
                if mods == 0 {
                    if let Some(seq) = whitelisted_minimal_named_key_fallback(key) {
                        self.key_fallback_named = self.key_fallback_named.saturating_add(1);
                        log::warn!(
                            target: "term_key_fallback",
                            "named key: libghostty empty; minimal CSI/byte whitelist key={key:?}"
                        );
                        seq
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// Extract selected viewport text (inclusive grid coords).
    pub fn selection_text_for_copy(&mut self, start: (u16, u16), end: (u16, u16)) -> String {
        // SSOT: text extraction comes from libghostty's render_state/grid view.
        // This avoids the UI caching layer (styled_fragments) becoming a "text truth".
        if self.vt.update_render_state().is_ok() {
            if let Ok(s) = self.vt.extract_viewport_text(start, end) {
                return s;
            }
        }
        let ((sx, sy), (ex, ey)) = normalize_sel(start, end);
        let max_row = self.rows.saturating_sub(1);
        let max_col = self.cols.saturating_sub(1);
        let sy = sy.min(max_row);
        let ey = ey.min(max_row);
        let sx = sx.min(max_col);
        let ex = ex.min(max_col);
        if sy > ey {
            return String::new();
        }

        match self.render_mode {
            TerminalRenderMode::Plain => {
                let mut out = String::new();
                for y in sy..=ey {
                    if (y as usize) >= self.lines.len() {
                        break;
                    }
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    let (lo, hi) = cols_for_row_range(sx, ex, sy, ey, y);
                    out.push_str(slice_by_cols(&self.lines[y as usize], lo, hi));
                }
                out
            }
            TerminalRenderMode::Styled => {
                let mut out = String::new();
                for y in sy..=ey {
                    if (y as usize) >= self.styled_rows.len() {
                        break;
                    }
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    let (lo, hi) = cols_for_row_range(sx, ex, sy, ey, y);
                    out.push_str(&styled_row_slice(&self.styled_rows[y as usize], lo, hi));
                }
                out
            }
        }
    }
}

fn normalize_sel(a: (u16, u16), b: (u16, u16)) -> ((u16, u16), (u16, u16)) {
    if (a.1, a.0) <= (b.1, b.0) {
        (a, b)
    } else {
        (b, a)
    }
}

fn cols_for_row_range(sx: u16, ex: u16, sy: u16, ey: u16, y: u16) -> (u16, u16) {
    if sy == ey {
        (sx, ex)
    } else if y == sy {
        (sx, u16::MAX)
    } else if y == ey {
        (0, ex)
    } else {
        (0, u16::MAX)
    }
}

fn slice_by_cols(s: &str, lo: u16, hi: u16) -> &str {
    // `lines` are built per-cell with spaces; treat each char as 1 column.
    if s.is_empty() {
        return "";
    }
    let lo = lo as usize;
    let hi = hi as usize;
    let mut start_b = None;
    let mut end_b = None;
    let mut col = 0usize;
    for (b, ch) in s.char_indices() {
        if start_b.is_none() && col >= lo {
            start_b = Some(b);
        }
        col += 1;
        if col > hi + 1 {
            end_b = Some(b);
            break;
        }
        if ch == '\n' {
            break;
        }
    }
    let start = start_b.unwrap_or(s.len());
    let end = end_b.unwrap_or(s.len());
    if start >= end {
        ""
    } else {
        &s[start..end]
    }
}

fn styled_row_slice(row: &crate::backend::ghostty_vt::VtStyledRow, lo: u16, hi: u16) -> String {
    let mut out = String::new();
    let mut x: u16 = 0;
    for run in &row.runs {
        for cell in &run.cells {
            if x > hi {
                return out;
            }
            if x >= lo {
                if cell.continuation {
                    // Skip visible placeholder.
                } else if cell.text.is_empty() {
                    out.push(' ');
                } else {
                    out.push_str(&cell.text);
                }
            }
            x = x.saturating_add(1);
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct StyledFragment {
    pub text: String,
    pub fg: ffi::GhosttyColorRgb,
    pub has_bg: bool,
    pub bg: ffi::GhosttyColorRgb,
    pub bold: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
    pub invisible: bool,
}

fn build_row_fragments(row: &VtStyledRow) -> Vec<StyledFragment> {
    let mut out: Vec<StyledFragment> = Vec::new();
    for run in &row.runs {
        for cell in &run.cells {
            let text = if cell.text.is_empty() {
                " ".to_string()
            } else {
                cell.text.clone()
            };
            out.push(StyledFragment {
                text,
                fg: run.fg,
                has_bg: run.has_bg,
                bg: run.bg,
                bold: run.bold,
                underline: run.underline,
                strikethrough: run.strikethrough,
                dim: run.dim,
                invisible: cell.continuation,
            });
        }
    }
    out
}

/// Named keys that still need VT‑compatible bytes when libghostty returns empty (no modifiers).
/// **Not** function keys — those must stay on the encoder path only.
fn whitelisted_minimal_named_key_fallback(key: TerminalKey) -> Option<Vec<u8>> {
    let bytes: &[u8] = match key {
        TerminalKey::Enter => b"\r",
        TerminalKey::Backspace => &[0x7f],
        TerminalKey::Tab => b"\t",
        TerminalKey::Escape => b"\x1b",
        TerminalKey::Space => b" ",
        TerminalKey::ArrowUp => b"\x1b[A",
        TerminalKey::ArrowDown => b"\x1b[B",
        TerminalKey::ArrowRight => b"\x1b[C",
        TerminalKey::ArrowLeft => b"\x1b[D",
        TerminalKey::Home => b"\x1b[H",
        TerminalKey::End => b"\x1b[F",
        TerminalKey::PageUp => b"\x1b[5~",
        TerminalKey::PageDown => b"\x1b[6~",
        TerminalKey::Insert => b"\x1b[2~",
        TerminalKey::Delete => b"\x1b[3~",
        TerminalKey::Function(_) => return None,
    };
    Some(bytes.to_vec())
}

#[inline]
fn ghostty_function_key(n: u8) -> Option<ffi::GhosttyKey> {
    if n == 0 || n > 25 {
        return None;
    }
    Some(ffi::GhosttyKey_GHOSTTY_KEY_F1 + (n as u32) - 1)
}

fn code_to_ghostty_key(code: Code) -> Option<ffi::GhosttyKey> {
    use Code::*;
    Some(match code {
        KeyA => ffi::GhosttyKey_GHOSTTY_KEY_A,
        KeyB => ffi::GhosttyKey_GHOSTTY_KEY_B,
        KeyC => ffi::GhosttyKey_GHOSTTY_KEY_C,
        KeyD => ffi::GhosttyKey_GHOSTTY_KEY_D,
        KeyE => ffi::GhosttyKey_GHOSTTY_KEY_E,
        KeyF => ffi::GhosttyKey_GHOSTTY_KEY_F,
        KeyG => ffi::GhosttyKey_GHOSTTY_KEY_G,
        KeyH => ffi::GhosttyKey_GHOSTTY_KEY_H,
        KeyI => ffi::GhosttyKey_GHOSTTY_KEY_I,
        KeyJ => ffi::GhosttyKey_GHOSTTY_KEY_J,
        KeyK => ffi::GhosttyKey_GHOSTTY_KEY_K,
        KeyL => ffi::GhosttyKey_GHOSTTY_KEY_L,
        KeyM => ffi::GhosttyKey_GHOSTTY_KEY_M,
        KeyN => ffi::GhosttyKey_GHOSTTY_KEY_N,
        KeyO => ffi::GhosttyKey_GHOSTTY_KEY_O,
        KeyP => ffi::GhosttyKey_GHOSTTY_KEY_P,
        KeyQ => ffi::GhosttyKey_GHOSTTY_KEY_Q,
        KeyR => ffi::GhosttyKey_GHOSTTY_KEY_R,
        KeyS => ffi::GhosttyKey_GHOSTTY_KEY_S,
        KeyT => ffi::GhosttyKey_GHOSTTY_KEY_T,
        KeyU => ffi::GhosttyKey_GHOSTTY_KEY_U,
        KeyV => ffi::GhosttyKey_GHOSTTY_KEY_V,
        KeyW => ffi::GhosttyKey_GHOSTTY_KEY_W,
        KeyX => ffi::GhosttyKey_GHOSTTY_KEY_X,
        KeyY => ffi::GhosttyKey_GHOSTTY_KEY_Y,
        KeyZ => ffi::GhosttyKey_GHOSTTY_KEY_Z,
        Digit0 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_0,
        Digit1 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_1,
        Digit2 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_2,
        Digit3 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_3,
        Digit4 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_4,
        Digit5 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_5,
        Digit6 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_6,
        Digit7 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_7,
        Digit8 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_8,
        Digit9 => ffi::GhosttyKey_GHOSTTY_KEY_DIGIT_9,
        Backquote => ffi::GhosttyKey_GHOSTTY_KEY_BACKQUOTE,
        Minus => ffi::GhosttyKey_GHOSTTY_KEY_MINUS,
        Equal => ffi::GhosttyKey_GHOSTTY_KEY_EQUAL,
        BracketLeft => ffi::GhosttyKey_GHOSTTY_KEY_BRACKET_LEFT,
        BracketRight => ffi::GhosttyKey_GHOSTTY_KEY_BRACKET_RIGHT,
        Backslash => ffi::GhosttyKey_GHOSTTY_KEY_BACKSLASH,
        Semicolon => ffi::GhosttyKey_GHOSTTY_KEY_SEMICOLON,
        Quote => ffi::GhosttyKey_GHOSTTY_KEY_QUOTE,
        IntlBackslash => ffi::GhosttyKey_GHOSTTY_KEY_INTL_BACKSLASH,
        IntlRo => ffi::GhosttyKey_GHOSTTY_KEY_INTL_RO,
        IntlYen => ffi::GhosttyKey_GHOSTTY_KEY_INTL_YEN,
        Comma => ffi::GhosttyKey_GHOSTTY_KEY_COMMA,
        Period => ffi::GhosttyKey_GHOSTTY_KEY_PERIOD,
        Slash => ffi::GhosttyKey_GHOSTTY_KEY_SLASH,
        Space => ffi::GhosttyKey_GHOSTTY_KEY_SPACE,
        Enter => ffi::GhosttyKey_GHOSTTY_KEY_ENTER,
        Tab => ffi::GhosttyKey_GHOSTTY_KEY_TAB,
        Backspace => ffi::GhosttyKey_GHOSTTY_KEY_BACKSPACE,
        Escape => ffi::GhosttyKey_GHOSTTY_KEY_ESCAPE,
        ArrowDown => ffi::GhosttyKey_GHOSTTY_KEY_ARROW_DOWN,
        ArrowLeft => ffi::GhosttyKey_GHOSTTY_KEY_ARROW_LEFT,
        ArrowRight => ffi::GhosttyKey_GHOSTTY_KEY_ARROW_RIGHT,
        ArrowUp => ffi::GhosttyKey_GHOSTTY_KEY_ARROW_UP,
        Home => ffi::GhosttyKey_GHOSTTY_KEY_HOME,
        End => ffi::GhosttyKey_GHOSTTY_KEY_END,
        PageUp => ffi::GhosttyKey_GHOSTTY_KEY_PAGE_UP,
        PageDown => ffi::GhosttyKey_GHOSTTY_KEY_PAGE_DOWN,
        Insert => ffi::GhosttyKey_GHOSTTY_KEY_INSERT,
        Delete => ffi::GhosttyKey_GHOSTTY_KEY_DELETE,
        Numpad0 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_0,
        Numpad1 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_1,
        Numpad2 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_2,
        Numpad3 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_3,
        Numpad4 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_4,
        Numpad5 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_5,
        Numpad6 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_6,
        Numpad7 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_7,
        Numpad8 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_8,
        Numpad9 => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_9,
        NumpadAdd => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_ADD,
        NumpadSubtract => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_SUBTRACT,
        NumpadMultiply => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_MULTIPLY,
        NumpadDivide => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_DIVIDE,
        NumpadDecimal => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_DECIMAL,
        NumpadEqual => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_EQUAL,
        NumpadEnter => ffi::GhosttyKey_GHOSTTY_KEY_NUMPAD_ENTER,
        _ => return None,
    })
}

#[inline]
fn dec_private_mode(value: u16) -> ffi::GhosttyMode {
    (value & 0x7fff) as ffi::GhosttyMode
}

fn mods_to_ffi(m: TerminalModifiers) -> ffi::GhosttyMods {
    let mut mods: ffi::GhosttyMods = 0;
    if m.shift {
        mods |= ffi::GHOSTTY_MODS_SHIFT as ffi::GhosttyMods;
    }
    if m.ctrl {
        mods |= ffi::GHOSTTY_MODS_CTRL as ffi::GhosttyMods;
    }
    if m.alt {
        mods |= ffi::GHOSTTY_MODS_ALT as ffi::GhosttyMods;
    }
    if m.logo {
        mods |= ffi::GHOSTTY_MODS_SUPER as ffi::GhosttyMods;
    }
    mods
}

#[cfg(test)]
mod tests {
    use super::*;

    fn join_lines(lines: &[String]) -> String {
        lines.join("\n")
    }

    #[test]
    fn plain_incremental_patch_matches_full_join_single_row() {
        let settings = TerminalSettings::default();
        let mut t = TerminalController::new(&settings);
        t.render_mode = TerminalRenderMode::Plain;
        t.plain_text_update = TerminalPlainTextUpdate::Incremental;

        t.lines = vec!["a".into(), "bb".into(), "ccc".into()];
        t.rebuild_plain_join_full();

        t.lines[1] = "B".into();
        t.patch_plain_join_incremental(&[1]);

        assert_eq!(t.plain_join_cache, join_lines(&t.lines));
    }

    #[test]
    fn plain_incremental_patch_matches_full_join_multiple_rows_unsorted() {
        let settings = TerminalSettings::default();
        let mut t = TerminalController::new(&settings);
        t.render_mode = TerminalRenderMode::Plain;
        t.plain_text_update = TerminalPlainTextUpdate::Incremental;

        t.lines = vec!["0".into(), "1".into(), "2".into(), "3".into()];
        t.rebuild_plain_join_full();

        t.lines[0] = "zero".into();
        t.lines[3] = "three".into();
        t.patch_plain_join_incremental(&[3, 0, 3]);

        assert_eq!(t.plain_join_cache, join_lines(&t.lines));
    }

    #[test]
    fn plain_incremental_patch_rebuilds_when_offsets_invalid() {
        let settings = TerminalSettings::default();
        let mut t = TerminalController::new(&settings);
        t.render_mode = TerminalRenderMode::Plain;
        t.plain_text_update = TerminalPlainTextUpdate::Incremental;

        t.lines = vec!["a".into(), "bb".into(), "ccc".into()];
        t.rebuild_plain_join_full();

        // Corrupt offsets to simulate drift/bug.
        t.plain_line_byte_offsets = vec![999, 999, 999];
        t.lines[1] = "B".into();
        t.patch_plain_join_incremental(&[1]);

        assert_eq!(t.plain_join_cache, join_lines(&t.lines));
        assert_eq!(t.plain_line_byte_offsets.len(), t.lines.len());
        assert_eq!(t.plain_line_byte_offsets[0], 0);
    }
}

