use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use eframe::egui;

use crate::backend::ssh_session::AsyncSession;
use crate::backend::ghostty_vt::{GhosttyVtTerminal, VtStyledRow};
use crate::backend::ghostty_vt::ffi as vtffi;
use crate::settings::TerminalSettings;
#[cfg(feature = "term-prof")]
use crate::prof::term_counters;
use crate::terminal::diagnostics::compare_dump::write_compare_dump;
use crate::terminal::gpu_renderer::TerminalOffscreenClearCallback;

pub const TERMINAL_PADDING_X: f32 = 6.0;
pub const TERMINAL_PADDING_Y: f32 = 6.0;
pub const TERMINAL_ROW_HEIGHT: f32 = 18.0;
pub const TERMINAL_CELL_WIDTH: f32 = 8.5;

#[derive(Clone, Debug)]
struct TerminalRenderSettings {
    gpu_enabled: bool,
    line_height: f32,
    scrollback_limit: usize,
    gpu_font_path: Option<String>,
    gpu_font_face_index: Option<u32>,
}

impl Default for TerminalRenderSettings {
    fn default() -> Self {
        Self {
            gpu_enabled: true,
            line_height: 1.4,
            scrollback_limit: 10_000,
            gpu_font_path: None,
            gpu_font_face_index: None,
        }
    }
}

/// Monotonic sequence for `paint_terminal` calls (for `RUST_SSH_VT_LOG_FRAME` diagnostics).
static VT_PAINT_FRAME_SEQ: AtomicU64 = AtomicU64::new(0);

enum VtPaintFrameLogSpec {
    Single(u64),
    Range(u64, u64),
}

static VT_PAINT_FRAME_LOG_SPEC: OnceLock<Option<VtPaintFrameLogSpec>> = OnceLock::new();

fn vt_paint_frame_log_matches(frame: u64) -> bool {
    let spec = VT_PAINT_FRAME_LOG_SPEC.get_or_init(|| {
        let Ok(raw) = std::env::var("RUST_SSH_VT_LOG_FRAME") else {
            return None;
        };
        let s = raw.trim();
        if s.is_empty() {
            return None;
        }
        if let Some((a, b)) = s.split_once('-') {
            let lo = a.trim().parse::<u64>().ok()?;
            let hi = b.trim().parse::<u64>().ok()?;
            let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
            Some(VtPaintFrameLogSpec::Range(lo, hi))
        } else {
            Some(VtPaintFrameLogSpec::Single(s.parse().ok()?))
        }
    });
    match spec.as_ref() {
        None => false,
        Some(VtPaintFrameLogSpec::Single(n)) => frame == *n,
        Some(VtPaintFrameLogSpec::Range(lo, hi)) => frame >= *lo && frame <= *hi,
    }
}

fn ghostty_dec_mode(value: u16) -> vtffi::GhosttyMode {
    // Matches ghostty/vt/modes.h packing:
    // bits 0..14: value, bit 15: ansi flag (0 for DEC private).
    (value & 0x7fff) as vtffi::GhosttyMode
}

fn should_use_gpu_terminal_text(
    has_wgpu: bool,
    has_gpu_texture: bool,
    gpu_env_enabled: bool,
    atlas_viable: bool,
) -> bool {
    has_wgpu && has_gpu_texture && gpu_env_enabled && atlas_viable
}

pub struct VtTerminalWidget {
    vt: Option<GhosttyVtTerminal>,
    last_cols: u16,
    last_rows: u16,
    last_snapshot_err: Option<String>,
    cached_rows: Vec<VtStyledRow>,
    cached_has_content: bool,
    /// Cached per-row paint data (galleys + style) to avoid per-frame text layout.
    row_paint_cache: Vec<RowPaintCache>,
    updated_rows_tmp: Vec<usize>,
    /// True when terminal state changed and render_state needs a refresh.
    needs_render_update: bool,
    selection_start: Option<(u16, u16)>,
    selection_end: Option<(u16, u16)>,
    selecting: bool,
    pending_selection_anchor: Option<(u16, u16)>,
    keep_selection_highlight: bool,
    /// Fallback alternate-screen tracking from raw VT stream.
    /// Needed because mode_get can transiently desync around some resize flows.
    alt_screen_active: bool,
    alt_seq_pending: Vec<u8>,
    gpu: crate::terminal::gpu_renderer::TerminalGpuRenderer,
    compare_dump_done: bool,
    settings: TerminalRenderSettings,
    needs_vt_recreate: bool,
}

struct RunPaintCache {
    galley: std::sync::Arc<egui::epaint::text::Galley>,
    cols: u16,
    fg: egui::Color32,
    bg: egui::Color32,
    has_bg: bool,
    bold: bool,
    underline: bool,
    strikethrough: bool,
}

#[derive(Default)]
struct RowPaintCache {
    runs: Vec<RunPaintCache>,
}

impl VtTerminalWidget {
    pub fn new() -> Self {
        Self {
            vt: None,
            last_cols: 0,
            last_rows: 0,
            last_snapshot_err: None,
            cached_rows: Vec::new(),
            cached_has_content: false,
            row_paint_cache: Vec::new(),
            updated_rows_tmp: Vec::new(),
            needs_render_update: false,
            selection_start: None,
            selection_end: None,
            selecting: false,
            pending_selection_anchor: None,
            keep_selection_highlight: true,
            alt_screen_active: false,
            alt_seq_pending: Vec::new(),
            gpu: crate::terminal::gpu_renderer::TerminalGpuRenderer::new(),
            compare_dump_done: false,
            settings: TerminalRenderSettings::default(),
            needs_vt_recreate: false,
        }
    }

    pub fn apply_settings(&mut self, settings: &TerminalSettings) {
        let next = TerminalRenderSettings {
            gpu_enabled: settings.gpu_acceleration,
            line_height: settings.line_height,
            scrollback_limit: settings.scrollback_limit.max(1000),
            gpu_font_path: settings.gpu_font_path.clone().filter(|s| !s.trim().is_empty()),
            gpu_font_face_index: settings.gpu_font_face_index,
        };
        if next.scrollback_limit != self.settings.scrollback_limit {
            // Scrollback size is chosen at VT creation time.
            self.needs_vt_recreate = true;
        }
        self.settings = next.clone();
        self.gpu.apply_settings(
            next.gpu_font_path.clone(),
            next.gpu_font_face_index,
            settings.atlas_reset_on_pressure,
        );
    }

    pub fn set_keep_selection_highlight(&mut self, keep: bool) {
        self.keep_selection_highlight = keep;
        if !keep && !self.selecting {
            self.selection_start = None;
            self.selection_end = None;
            self.pending_selection_anchor = None;
        }
    }

    pub fn ensure_created(&mut self, cols: u16, rows: u16) {
        if cols == 0 || rows == 0 {
            return;
        }
        if self.vt.is_some() && !self.needs_vt_recreate {
            return;
        }

        if self.needs_vt_recreate {
            self.vt = None;
            self.cached_rows.clear();
            self.cached_has_content = false;
            self.row_paint_cache.clear();
            self.updated_rows_tmp.clear();
            self.needs_render_update = true;
            self.needs_vt_recreate = false;
        }

        if self.vt.is_none() {
            if let Ok(vt) = GhosttyVtTerminal::new(cols, rows, self.settings.scrollback_limit) {
                self.vt = Some(vt);
                self.needs_render_update = true;
            }
        }
    }

    pub fn on_resize_cells(&mut self, cols: u16, rows: u16) {
        if cols == 0 || rows == 0 {
            return;
        }
        if cols == self.last_cols && rows == self.last_rows {
            return;
        }
        if let Some(vt) = self.vt.as_mut() {
            let _ = vt.resize(cols, rows);
            self.needs_render_update = true;
        }
        self.last_cols = cols;
        self.last_rows = rows;
        self.cached_rows.clear();
        self.cached_has_content = false;
        self.row_paint_cache.clear();
        self.updated_rows_tmp.clear();
    }

    pub fn on_ssh_bytes(&mut self, bytes: &[u8]) {
        self.update_alt_screen_state_from_stream(bytes);
        if let Some(vt) = self.vt.as_mut() {
            vt.write_vt(bytes);
            self.needs_render_update = true;
        }
    }

    fn update_alt_screen_state_from_stream(&mut self, bytes: &[u8]) {
        // Keep a short tail to catch split sequences across chunks.
        const MAX_TAIL: usize = 32;
        if bytes.is_empty() {
            return;
        }
        // Reuse the pending buffer directly to avoid per-chunk merged allocations.
        self.alt_seq_pending.extend_from_slice(bytes);

        // Parse CSI private mode set/reset in stream order, so `...l...h` and
        // `...h...l` in the same chunk end with the correct final state.
        let mut i = 0usize;
        while i + 4 < self.alt_seq_pending.len() {
            if self.alt_seq_pending[i] != 0x1b
                || self.alt_seq_pending[i + 1] != b'['
                || self.alt_seq_pending[i + 2] != b'?'
            {
                i += 1;
                continue;
            }

            let mut j = i + 3;
            let mut mode: u16 = 0;
            let mut has_digit = false;
            while j < self.alt_seq_pending.len() && self.alt_seq_pending[j].is_ascii_digit() {
                has_digit = true;
                mode = mode
                    .saturating_mul(10)
                    .saturating_add((self.alt_seq_pending[j] - b'0') as u16);
                j += 1;
            }
            if !has_digit || j >= self.alt_seq_pending.len() {
                i += 1;
                continue;
            }

            let action = self.alt_seq_pending[j];
            let is_alt_mode = mode == 47 || mode == 1047 || mode == 1049;
            if is_alt_mode {
                if action == b'h' {
                    self.alt_screen_active = true;
                } else if action == b'l' {
                    self.alt_screen_active = false;
                }
            }

            i = j + 1;
        }

        if self.alt_seq_pending.len() > MAX_TAIL {
            let drop_n = self.alt_seq_pending.len() - MAX_TAIL;
            self.alt_seq_pending.drain(..drop_n);
        }
    }

    pub fn on_key_event(
        &mut self,
        key: egui::Key,
        pressed: bool,
        modifiers: egui::Modifiers,
        session: &mut Option<Box<dyn AsyncSession>>,
    ) {
        if !pressed {
            return;
        }
        let (Some(vt), Some(s)) = (self.vt.as_mut(), session.as_mut()) else {
            return;
        };

        let mods: vtffi::GhosttyMods = {
            let mut m: u16 = 0;
            if modifiers.shift { m |= vtffi::GHOSTTY_MODS_SHIFT as u16; }
            if modifiers.ctrl { m |= vtffi::GHOSTTY_MODS_CTRL as u16; }
            if modifiers.alt { m |= vtffi::GHOSTTY_MODS_ALT as u16; }
            if modifiers.command { m |= vtffi::GHOSTTY_MODS_SUPER as u16; }
            m
        };

        let vt_key: Option<vtffi::GhosttyKey> = match key {
            egui::Key::Enter => Some(vtffi::GhosttyKey_GHOSTTY_KEY_ENTER),
            egui::Key::Backspace => Some(vtffi::GhosttyKey_GHOSTTY_KEY_BACKSPACE),
            egui::Key::Tab => Some(vtffi::GhosttyKey_GHOSTTY_KEY_TAB),
            egui::Key::Escape => Some(vtffi::GhosttyKey_GHOSTTY_KEY_ESCAPE),
            egui::Key::ArrowUp => Some(vtffi::GhosttyKey_GHOSTTY_KEY_ARROW_UP),
            egui::Key::ArrowDown => Some(vtffi::GhosttyKey_GHOSTTY_KEY_ARROW_DOWN),
            egui::Key::ArrowLeft => Some(vtffi::GhosttyKey_GHOSTTY_KEY_ARROW_LEFT),
            egui::Key::ArrowRight => Some(vtffi::GhosttyKey_GHOSTTY_KEY_ARROW_RIGHT),
            egui::Key::Home => Some(vtffi::GhosttyKey_GHOSTTY_KEY_HOME),
            egui::Key::End => Some(vtffi::GhosttyKey_GHOSTTY_KEY_END),
            egui::Key::PageUp => Some(vtffi::GhosttyKey_GHOSTTY_KEY_PAGE_UP),
            egui::Key::PageDown => Some(vtffi::GhosttyKey_GHOSTTY_KEY_PAGE_DOWN),
            egui::Key::Insert => Some(vtffi::GhosttyKey_GHOSTTY_KEY_INSERT),
            egui::Key::Delete => Some(vtffi::GhosttyKey_GHOSTTY_KEY_DELETE),
            _ => None,
        };

        if let Some(vk) = vt_key {
            if let Ok(seq) =
                vt.encode_key(vtffi::GhosttyKeyAction_GHOSTTY_KEY_ACTION_PRESS, vk, mods)
            {
                if !seq.is_empty() {
                    let _ = s.write_stream(&seq);
                }
            }
        }
    }

    pub fn encode_paste(&mut self, text: &str) -> Vec<u8> {
        let Some(vt) = self.vt.as_mut() else {
            return text.as_bytes().to_vec();
        };

        let bracketed = vt
            .mode_get(ghostty_dec_mode(2004))
            .unwrap_or(false);

        // Safety check (best-effort): still allow paste, but prefer bracketed
        // paste when enabled by the remote application.
        let safe = unsafe { vtffi::ghostty_paste_is_safe(text.as_ptr() as *const _, text.len()) };

        if bracketed {
            let mut out = Vec::with_capacity(text.len() + 16);
            out.extend_from_slice(b"\x1b[200~");
            out.extend_from_slice(text.as_bytes());
            out.extend_from_slice(b"\x1b[201~");
            out
        } else if safe {
            text.as_bytes().to_vec()
        } else {
            // If not safe and bracketed paste is not enabled, we still forward
            // the paste (matching common terminal behavior). A future UX
            // improvement could prompt the user for confirmation here.
            text.as_bytes().to_vec()
        }
    }

    pub fn encode_focus_event(&mut self, focused: bool) -> Vec<u8> {
        let Some(vt) = self.vt.as_mut() else {
            return Vec::new();
        };

        let focus_mode = vt
            .mode_get(ghostty_dec_mode(1004))
            .unwrap_or(false);
        if !focus_mode {
            return Vec::new();
        }

        let ev = if focused {
            vtffi::GhosttyFocusEvent_GHOSTTY_FOCUS_GAINED
        } else {
            vtffi::GhosttyFocusEvent_GHOSTTY_FOCUS_LOST
        };

        // CSI I / CSI O are tiny, but handle OUT_OF_SPACE anyway.
        let mut buf = [0u8; 8];
        let mut written: usize = 0;
        let res = unsafe {
            vtffi::ghostty_focus_encode(
                ev,
                buf.as_mut_ptr() as *mut _,
                buf.len(),
                &mut written,
            )
        };
        if res == vtffi::GhosttyResult_GHOSTTY_SUCCESS {
            buf[..written].to_vec()
        } else if res == vtffi::GhosttyResult_GHOSTTY_OUT_OF_SPACE && written > 0 {
            let mut v = vec![0u8; written];
            let mut written2: usize = 0;
            let res2 = unsafe {
                vtffi::ghostty_focus_encode(
                    ev,
                    v.as_mut_ptr() as *mut _,
                    v.len(),
                    &mut written2,
                )
            };
            if res2 == vtffi::GhosttyResult_GHOSTTY_SUCCESS {
                v.truncate(written2);
                v
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    pub fn paint_terminal(&mut self, ui: &mut egui::Ui, rect: egui::Rect, frame: &mut eframe::Frame) {
        let paint_frame_seq = VT_PAINT_FRAME_SEQ.fetch_add(1, Ordering::Relaxed);

        #[cfg(feature = "term-prof")]
        let _span = tracing::info_span!(
            "vt.paint_terminal",
            cols = tracing::field::Empty,
            rows = tracing::field::Empty
        )
        .entered();

        let Some(vt) = self.vt.as_mut() else { return; };
        #[cfg(feature = "term-prof")]
        tracing::Span::current().record("cols", vt.cols());
        #[cfg(feature = "term-prof")]
        tracing::Span::current().record("rows", vt.rows());

        // --- WGPU hook proof: ensure we can see wgpu state from here.
        // This is the first step for render-to-texture.
        if frame.wgpu_render_state().is_some() {
            // Create / resize an offscreen texture and register it as a TextureId.
            let px_per_pt = ui.ctx().pixels_per_point();
            let w_px = (rect.width() * px_per_pt).round() as u32;
            let h_px = (rect.height() * px_per_pt).round() as u32;
            let _ = self.gpu.ensure_texture(frame, w_px, h_px);

            // Enqueue a wgpu callback that renders into the offscreen texture.
            if let (Some(texture), Some(view)) =
                (self.gpu.offscreen_texture(), self.gpu.offscreen_view())
            {
                let cb = TerminalOffscreenClearCallback::new(
                    texture,
                    view,
                    self.gpu.shared_frame_data(),
                    self.gpu.glyph_atlas_shared(),
                );
                let paint_cb = egui_wgpu::Callback::new_paint_callback(rect, cb);
                ui.painter().add(egui::Shape::Callback(paint_cb));
            }
        }

        // If GPU texture exists, paint it behind text for now.
        if let Some(tid) = self.gpu.texture_id {
            ui.painter().add(egui::Shape::image(
                tid,
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            ));
        }

        // Refresh render_state snapshot at most once per frame even if multiple writes happened.
        // This must happen before any cursor/selection reads from render-state.
        let refreshed_render_state_this_frame = self.needs_render_update;
        if self.needs_render_update {
            if let Err(e) = vt.update_render_state() {
                self.last_snapshot_err = Some(e.to_string());
            } else {
                self.last_snapshot_err = None;
            }
            self.needs_render_update = false;
        }

        // Selection (mouse drag in viewport).
        let pointer_pos = ui.input(|i| i.pointer.interact_pos());
        let pointer_over = pointer_pos.map(|p| rect.contains(p)).unwrap_or(false);

        let padding = egui::vec2(TERMINAL_PADDING_X, TERMINAL_PADDING_Y);
        let line_h = (TERMINAL_ROW_HEIGHT * self.settings.line_height).max(1.0);
        let char_w = TERMINAL_CELL_WIDTH;

        let to_cell = |p: egui::Pos2| -> Option<(u16, u16)> {
            if !rect.contains(p) {
                return None;
            }
            let x = p.x - (rect.left() + padding.x);
            let y = p.y - (rect.top() + padding.y);
            if x < 0.0 || y < 0.0 {
                return None;
            }
            let cx = (x / char_w).floor() as i32;
            let cy = (y / line_h).floor() as i32;
            if cx < 0 || cy < 0 {
                return None;
            }
            let cx = (cx as u16).min(vt.cols().saturating_sub(1));
            let cy = (cy as u16).min(vt.rows().saturating_sub(1));
            Some((cx, cy))
        };

        if pointer_over && ui.input(|i| i.pointer.primary_pressed()) {
            if let Some(p) = pointer_pos {
                if let Some(cell) = to_cell(p) {
                    // Do not show selection on single click; arm selection and
                    // only start highlighting after actual drag movement.
                    self.pending_selection_anchor = Some(cell);
                    self.selecting = false;
                }
            }
        }

        if ui.input(|i| i.pointer.primary_down()) {
            if let Some(p) = pointer_pos {
                if let Some(cell) = to_cell(p) {
                    if !self.selecting {
                        if let Some(anchor) = self.pending_selection_anchor {
                            if anchor != cell {
                                self.selection_start = Some(anchor);
                                self.selection_end = Some(cell);
                                self.selecting = true;
                                ui.ctx().request_repaint();
                            }
                        }
                    } else if self.selection_end != Some(cell) {
                        self.selection_end = Some(cell);
                        // Only repaint when the selection cell actually changes.
                        ui.ctx().request_repaint();
                    }
                }
            }
        }

        if ui.input(|i| i.pointer.primary_released()) {
            if self.selecting {
                self.selecting = false;
                // Ensure we capture the final cell even if the pointer moved quickly.
                if let Some(p) = pointer_pos {
                    if let Some(cell) = to_cell(p) {
                        self.selection_end = Some(cell);
                    }
                }
                if let (Some(a), Some(b)) = (self.selection_start, self.selection_end) {
                    if let Ok(text) = vt.extract_viewport_text(a, b) {
                        if !text.is_empty() {
                            let preview: String = text.chars().take(80).collect();
                            tracing::debug!(
                                target: "term-diag",
                                copied_len = text.len(),
                                preview = %preview.replace('\n', "\\n"),
                                "terminal selection copied text"
                            );
                            ui.ctx().copy_text(text);
                        }
                    }
                }
                if !self.keep_selection_highlight {
                    self.selection_start = None;
                    self.selection_end = None;
                }
            } else if self.pending_selection_anchor.is_some() {
                // Click without dragging should clear any previous selection.
                if self.selection_start.is_some() || self.selection_end.is_some() {
                    self.selection_start = None;
                    self.selection_end = None;
                    ui.ctx().request_repaint();
                }
            }
            self.pending_selection_anchor = None;
        }

        // Mouse wheel scrolling (delegate to libghostty viewport scrolling).
        if pointer_over {
            let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_y.abs() > 0.0 {
                // Full-screen TUIs (top/htop/vim etc.) are usually on alternate
                // screen and should not expose scrollback viewport scrolling.
                let in_alternate_screen_by_mode = vt.mode_get(ghostty_dec_mode(1049)).unwrap_or(false)
                    || vt.mode_get(ghostty_dec_mode(1047)).unwrap_or(false)
                    || vt.mode_get(ghostty_dec_mode(47)).unwrap_or(false);
                let in_alternate_screen = in_alternate_screen_by_mode || self.alt_screen_active;
                if !in_alternate_screen {
                    // egui: positive y usually means scrolling up; libghostty: up is negative.
                    let delta_rows = (-(scroll_y / line_h)).round() as isize;
                    if delta_rows != 0 {
                        vt.scroll_viewport_delta_rows(delta_rows);
                        self.needs_render_update = true;
                        // Selection is stored in viewport cell coordinates. After changing the viewport
                        // offset, keeping the old selection highlight is misleading. Clear on scroll.
                        self.selecting = false;
                        self.pending_selection_anchor = None;
                        self.selection_start = None;
                        self.selection_end = None;
                        ui.ctx().request_repaint();
                    }
                }
            }
        }

        #[cfg(debug_assertions)]
        {
            // Show that we entered VT paint even if frame is empty.
            ui.painter().rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgba_unmultiplied(0, 255, 0, 180)),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.left_top() + egui::vec2(TERMINAL_PADDING_X, TERMINAL_PADDING_Y + 16.0),
                egui::Align2::LEFT_TOP,
                format!("VT-PAINT cols={} rows={}", vt.cols(), vt.rows()),
                egui::FontId::monospace(11.0),
                egui::Color32::from_rgb(200, 255, 200),
            );
        }

        // Pull styled rows when libghostty marks dirty, OR when we just synced terminal→render_state.
        // `ghostty_render_state_update` can leave global DIRTY_FALSE while row content already
        // changed (row-level dirty not set). Using only `dirty()` then skips all rows → stale
        // `cached_rows` while `cursor_state()` still updates → "cursor moves, no text" (see
        // doc/终端区域无显示问题复盘.md).
        let lib_dirty = match vt.dirty().ok() {
            Some(vtffi::GhosttyRenderStateDirty_GHOSTTY_RENDER_STATE_DIRTY_FALSE) => false,
            Some(_) => true,
            None => true,
        };
        let needs_row_pull = lib_dirty || refreshed_render_state_this_frame;
        // Incremental row walk only when the library reports dirty; otherwise scan all rows once
        // after a fresh sync so we never paint an empty grid after SSH/PTY bytes.
        let only_dirty_rows = !refreshed_render_state_this_frame || lib_dirty;

        if !self.cached_has_content {
            match vt.snapshot_styled_rows_and_clear_dirty() {
                Ok(rows) => {
                    self.last_snapshot_err = None;
                    self.cached_has_content = true;
                    self.cached_rows = rows;
                    // Rebuild paint cache for all rows.
                    self.row_paint_cache.clear();
                    self.row_paint_cache
                        .resize_with(self.cached_rows.len(), RowPaintCache::default);
                    self.updated_rows_tmp.clear();
                    self.updated_rows_tmp.extend(0..self.cached_rows.len());
                }
                Err(e) => {
                    self.last_snapshot_err = Some(e.to_string());
                }
            }
        } else if needs_row_pull {
            self.updated_rows_tmp.clear();
            if let Err(e) = vt.update_dirty_styled_rows_and_clear_dirty_collect(
                &mut self.cached_rows,
                only_dirty_rows,
                Some(&mut self.updated_rows_tmp),
            ) {
                self.last_snapshot_err = Some(e.to_string());
            } else {
                self.last_snapshot_err = None;
            }
        }

        if vt_paint_frame_log_matches(paint_frame_seq) {
            tracing::debug!(
                target: "vt.paint",
                paint_frame = paint_frame_seq,
                refreshed_render_state_this_frame,
                lib_dirty,
                needs_row_pull,
                only_dirty_rows,
                updated_rows_len = self.updated_rows_tmp.len(),
                cached_rows_len = self.cached_rows.len(),
                cached_has_content = self.cached_has_content,
                snapshot_err = self.last_snapshot_err.is_some(),
                cols = vt.cols(),
                rows = vt.rows(),
                "vt paint snapshot state"
            );
        }

        let x = rect.left() + padding.x;

        // Update cell grid + (optional) GPU instances. Must run every frame so the callback sees fresh data.
        // IMPORTANT: gpu/CPU mode decision must happen AFTER updating the cell grid so we don't render
        // "one bad frame" when a complex grapheme arrives.
        self.gpu
            .update_cells_from_rows(vt.cols(), vt.rows(), &self.cached_rows, &self.updated_rows_tmp);

        // When WGPU + offscreen texture are active, terminal pixels come from the GPU pass
        // (`Callback` + `image` above); skip the CPU backdrop so it does not cover the texture.
        let has_wgpu = frame.wgpu_render_state().is_some();
        let has_gpu_texture = self.gpu.texture_id.is_some();
        let env_forces_off = std::env::var("RUST_SSH_TERMINAL_GPU")
            .ok()
            .as_deref()
            == Some("0");
        let gpu_env_enabled = !env_forces_off && self.settings.gpu_enabled;
        let atlas_viable = self.gpu.gpu_glyph_raster_viable() && self.gpu.gpu_text_safe();
        let gpu_terminal_text =
            should_use_gpu_terminal_text(has_wgpu, has_gpu_texture, gpu_env_enabled, atlas_viable);

        let display_test_mode = std::env::var("RUST_SSH_TERM_DISPLAY_TEST")
            .ok()
            .as_deref()
            == Some("1");
        if paint_frame_seq % 120 == 0 {
            tracing::debug!(
                target: "term-diag",
                paint_frame = paint_frame_seq,
                gpu_terminal_text,
                has_wgpu,
                has_gpu_texture,
                gpu_env_enabled,
                atlas_viable,
                "terminal paint mode decision"
            );
        }

        if has_wgpu {
            let px_per_pt = ui.ctx().pixels_per_point();
            let viewport_px = (
                (rect.width() * px_per_pt).round() as u32,
                (rect.height() * px_per_pt).round() as u32,
            );
            let cell_size_px = (
                TERMINAL_CELL_WIDTH * px_per_pt,
                line_h * px_per_pt,
            );
            let origin_px = (TERMINAL_PADDING_X * px_per_pt, TERMINAL_PADDING_Y * px_per_pt);
            self.gpu
                .update_viewport_params(viewport_px, cell_size_px, origin_px, self.settings.line_height);
            if gpu_terminal_text {
                self.gpu.build_bg_instances(vt.cols(), vt.rows());
                self.gpu.build_glyph_instances(vt.cols(), vt.rows());
            }
        }

        if gpu_terminal_text && paint_frame_seq % 60 == 0 {
            let (bg_n, glyph_n) = self.gpu.instance_counts();
            let build_diag = self.gpu.glyph_build_diag();
            let atlas_diag = self.gpu.atlas_diag_stats();
            let (off_w, off_h) = self.gpu.offscreen_size_px();
            let px_per_pt = ui.ctx().pixels_per_point();
            let draw_w_px = (rect.width() * px_per_pt).max(1.0);
            let draw_h_px = (rect.height() * px_per_pt).max(1.0);
            let offscreen_draw_w_ratio = (off_w as f32) / draw_w_px;
            let offscreen_draw_h_ratio = (off_h as f32) / draw_h_px;
            let cell_h = TERMINAL_ROW_HEIGHT.max(1.0);
            // Current GPU pipeline draws one quad per cell-sized glyph instance.
            let quad_h = TERMINAL_ROW_HEIGHT;
            let ratio = quad_h / cell_h;
            if display_test_mode {
                tracing::debug!(
                    target: "term-diag",
                    paint_frame = paint_frame_seq,
                    gpu_terminal_text,
                    quad_height = quad_h,
                    cell_height = cell_h,
                    quad_to_cell_ratio = ratio,
                    bg_instances = bg_n,
                    glyph_instances = glyph_n,
                    non_space_cells = build_diag.non_space_cells,
                    zero_slot_cells = build_diag.zero_slot_cells,
                    underline_space_quads = build_diag.underline_space_quads,
                    font_mono_advance = build_diag.font_mono_advance,
                    cell_width_px = build_diag.cell_width_px,
                    bbox_w_over_slot_min = build_diag.bbox_w_over_slot_min,
                    bbox_w_over_slot_mean = build_diag.bbox_w_over_slot_mean,
                    bbox_w_over_slot_max = build_diag.bbox_w_over_slot_max,
                    offscreen_draw_w_ratio,
                    offscreen_draw_h_ratio,
                    off_y_stddev = build_diag.off_y_stddev,
                    bbox_top_drift = build_diag.bbox_top_drift,
                    bbox_bottom_drift = build_diag.bbox_bottom_drift,
                    mapped_chars = atlas_diag.chars_mapped,
                    failed_cached_chars = atlas_diag.failed_chars_cached,
                    failed_cache_hits = atlas_diag.failed_cache_hits,
                    zero_pixel_failures = atlas_diag.zero_pixel_failures,
                    fallback_successes = atlas_diag.fallback_successes,
                    "GPU paint geometry check"
                );
            } else {
                tracing::debug!(
                    target: "term-diag",
                    paint_frame = paint_frame_seq,
                    gpu_terminal_text,
                    quad_height = quad_h,
                    cell_height = cell_h,
                    quad_to_cell_ratio = ratio,
                    bg_instances = bg_n,
                    glyph_instances = glyph_n,
                    non_space_cells = build_diag.non_space_cells,
                    zero_slot_cells = build_diag.zero_slot_cells,
                    underline_space_quads = build_diag.underline_space_quads,
                    font_mono_advance = build_diag.font_mono_advance,
                    cell_width_px = build_diag.cell_width_px,
                    bbox_w_over_slot_min = build_diag.bbox_w_over_slot_min,
                    bbox_w_over_slot_mean = build_diag.bbox_w_over_slot_mean,
                    bbox_w_over_slot_max = build_diag.bbox_w_over_slot_max,
                    offscreen_draw_w_ratio,
                    offscreen_draw_h_ratio,
                    off_y_stddev = build_diag.off_y_stddev,
                    bbox_top_drift = build_diag.bbox_top_drift,
                    bbox_bottom_drift = build_diag.bbox_bottom_drift,
                    mapped_chars = atlas_diag.chars_mapped,
                    failed_cached_chars = atlas_diag.failed_chars_cached,
                    failed_cache_hits = atlas_diag.failed_cache_hits,
                    zero_pixel_failures = atlas_diag.zero_pixel_failures,
                    fallback_successes = atlas_diag.fallback_successes,
                    "GPU paint geometry check"
                );
            }
            if ratio < 0.30 {
                tracing::warn!(
                    target: "term-diag",
                    paint_frame = paint_frame_seq,
                    quad_height = quad_h,
                    cell_height = cell_h,
                    quad_to_cell_ratio = ratio,
                    "GPU glyph quad height is below 30% of cell height"
                );
            }
        }

        // Ensure cache is sized (CPU text path only).
        if !gpu_terminal_text && self.row_paint_cache.len() != self.cached_rows.len() {
            self.row_paint_cache
                .resize_with(self.cached_rows.len(), RowPaintCache::default);
        }

        // Background (CPU fallback only; GPU clear uses the same RGB in `gpu_renderer`).
        #[cfg(feature = "term-prof")]
        let _bg_span = tracing::info_span!("vt.paint.background").entered();
        if !gpu_terminal_text {
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_rgb(10, 10, 15));
        }
        #[cfg(feature = "term-prof")]
        drop(_bg_span);

        // Selection highlight (CPU path: behind egui text; GPU path: over the terminal image).
        #[cfg(feature = "term-prof")]
        let _sel_span = tracing::info_span!(
            "vt.paint.selection",
            active = (self.selection_start.is_some() && self.selection_end.is_some())
        )
        .entered();
        if let (Some(a), Some(b)) = (self.selection_start, self.selection_end) {
            let (mut x0, mut y0) = a;
            let (mut x1, mut y1) = b;
            if (y1, x1) < (y0, x0) {
                std::mem::swap(&mut x0, &mut x1);
                std::mem::swap(&mut y0, &mut y1);
            }

            // If the selection is a single point, still show a 1-cell highlight.
            let max_x = vt.cols().saturating_sub(1);
            let max_y = vt.rows().saturating_sub(1);
            x0 = x0.min(max_x);
            x1 = x1.min(max_x);
            y0 = y0.min(max_y);
            y1 = y1.min(max_y);

            // Selection color: GitHub blue for consistency with terminal standards
            let sel_color = egui::Color32::from_rgba_unmultiplied(56, 139, 253, 70);
            let draw_rect = |ui: &mut egui::Ui, x0: u16, x1: u16, y: u16, h_rows: u16| {
                let px = rect.left() + padding.x + (x0 as f32) * char_w;
                let py = rect.top() + padding.y + (y as f32) * line_h;
                let w = ((x1.saturating_sub(x0) + 1) as f32) * char_w;
                let h = (h_rows as f32) * line_h;
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(egui::pos2(px, py), egui::vec2(w, h)),
                    0.0,
                    sel_color,
                );
            };

            if y0 == y1 {
                draw_rect(ui, x0, x1, y0, 1);
            } else {
                // First row: x0..max_x
                draw_rect(ui, x0, max_x, y0, 1);

                // Middle block: 0..max_x across (y0+1 .. y1-1)
                if y1 > y0 + 1 {
                    draw_rect(ui, 0, max_x, y0 + 1, (y1 - y0 - 1) as u16);
                }

                // Last row: 0..x1
                draw_rect(ui, 0, x1, y1, 1);
            }
        }
        #[cfg(feature = "term-prof")]
        drop(_sel_span);

        if gpu_terminal_text
            && !self.compare_dump_done
            && std::env::var("RUST_SSH_TERM_DUMP_COMPARE")
                .ok()
                .as_deref()
                == Some("1")
        {
            let (gpu_snap, atlas_snap) = self.gpu.dump_snapshot();
            match write_compare_dump(&self.cached_rows, &gpu_snap, &atlas_snap) {
                Ok(r) => {
                    tracing::info!(
                        target: "term-diag",
                        dir = %r.dir.display(),
                        cpu = %r.cpu_path.display(),
                        gpu = %r.gpu_path.display(),
                        diff = %r.diff_path.display(),
                        compare = %r.compare_path.display(),
                        used_slots = %r.used_slots_path.display(),
                        used_slots_manifest = %r.used_slots_manifest_path.display(),
                        mean_abs_diff = r.mean_abs_diff,
                        diff_ratio = r.diff_ratio,
                        gpu_cov_mean = r.gpu_cov_mean,
                        gpu_cov_nonzero_ratio = r.gpu_cov_nonzero_ratio,
                        "terminal compare dump generated"
                    );
                    self.compare_dump_done = true;
                }
                Err(e) => {
                    tracing::warn!(
                        target: "term-diag",
                        error = %e,
                        "terminal compare dump failed"
                    );
                }
            }
        }

        if !gpu_terminal_text {
            // Styled text rendering (fg/bg/bold/underline) — CPU fallback when no GPU texture.
            #[cfg(feature = "term-prof")]
            let text_span = tracing::info_span!(
                "vt.paint.text_runs",
                rows_drawn = tracing::field::Empty,
                runs_drawn = tracing::field::Empty,
                glyphs = tracing::field::Empty
            );
            #[cfg(feature = "term-prof")]
            let _text_enter = text_span.enter();
            let font = egui::FontId::monospace(14.0);
            let mut y = rect.top() + padding.y;
            let max_y = rect.bottom();

            #[cfg(feature = "term-prof")]
            let mut rows_drawn: u64 = 0;
            #[cfg(feature = "term-prof")]
            let mut runs_drawn: u64 = 0;
            #[cfg(feature = "term-prof")]
            let mut cells_drawn: u64 = 0;
            #[cfg(feature = "term-prof")]
            let mut text_calls: u64 = 0;
            #[cfg(feature = "term-prof")]
            let mut bg_rects: u64 = 0;

            // Rebuild galleys only for updated rows.
            if !self.updated_rows_tmp.is_empty() {
                let painter = ui.painter().clone();
                for &row_idx in &self.updated_rows_tmp {
                    if row_idx >= self.cached_rows.len() {
                        continue;
                    }
                    let row = &self.cached_rows[row_idx];
                    let cache_row = &mut self.row_paint_cache[row_idx];
                    cache_row.runs.clear();
                    cache_row.runs.reserve(row.runs.len());
                    for run in &row.runs {
                        if run.text.is_empty() {
                            continue;
                        }
                        let fg0 = egui::Color32::from_rgb(run.fg.r, run.fg.g, run.fg.b);
                        let fg = if run.dim {
                            // Simple dim: scale RGB down (keep alpha=255).
                            egui::Color32::from_rgb(
                                (run.fg.r as u16 * 160 / 255) as u8,
                                (run.fg.g as u16 * 160 / 255) as u8,
                                (run.fg.b as u16 * 160 / 255) as u8,
                            )
                        } else {
                            fg0
                        };
                        let bg = egui::Color32::from_rgb(run.bg.r, run.bg.g, run.bg.b);
                        let galley = painter.layout_no_wrap(run.text.clone(), font.clone(), fg);
                        cache_row.runs.push(RunPaintCache {
                            galley,
                            cols: run.cols,
                            fg,
                            bg,
                            has_bg: run.has_bg,
                            bold: run.bold,
                            underline: run.underline,
                            strikethrough: run.strikethrough,
                        });
                    }
                }
            }

            // Paint from cache.
            for (row_idx, row_cache) in self.row_paint_cache.iter().enumerate() {
                if y + line_h > max_y {
                    break;
                }
                #[cfg(feature = "term-prof")]
                {
                    rows_drawn = rows_drawn.saturating_add(1);
                }
                let mut x_cur = x;
                for run in &row_cache.runs {
                    let run_w = (run.cols as f32) * char_w;
                    #[cfg(feature = "term-prof")]
                    {
                        runs_drawn = runs_drawn.saturating_add(1);
                        cells_drawn = cells_drawn.saturating_add(run.cols as u64);
                    }

                    if run.has_bg {
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(egui::pos2(x_cur, y), egui::vec2(run_w, line_h)),
                            0.0,
                            run.bg,
                        );
                        #[cfg(feature = "term-prof")]
                        {
                            bg_rects = bg_rects.saturating_add(1);
                        }
                    }

                    ui.painter().galley(egui::pos2(x_cur, y), run.galley.clone(), run.fg);
                    #[cfg(feature = "term-prof")]
                    {
                        text_calls = text_calls.saturating_add(1);
                    }
                    if run.bold {
                        ui.painter()
                            .galley(egui::pos2(x_cur + 0.7, y), run.galley.clone(), run.fg);
                        #[cfg(feature = "term-prof")]
                        {
                            text_calls = text_calls.saturating_add(1);
                        }
                    }
                    if run.underline {
                        let y_ul = y + line_h - 3.0;
                        ui.painter().line_segment(
                            [egui::pos2(x_cur, y_ul), egui::pos2(x_cur + run_w, y_ul)],
                            egui::Stroke::new(1.0, run.fg),
                        );
                    }
                    if run.strikethrough {
                        let y_mid = y + line_h * 0.55;
                        ui.painter().line_segment(
                            [egui::pos2(x_cur, y_mid), egui::pos2(x_cur + run_w, y_mid)],
                            egui::Stroke::new(1.0, run.fg),
                        );
                    }

                    x_cur += run_w;
                }
                y += line_h;
                let _ = row_idx;
            }
            #[cfg(feature = "term-prof")]
            {
                tracing::Span::current().record("rows_drawn", rows_drawn);
                tracing::Span::current().record("runs_drawn", runs_drawn);
                tracing::Span::current().record("glyphs", cells_drawn);
                term_counters::add_rows(rows_drawn);
                term_counters::add_runs(runs_drawn);
                term_counters::add_cells(cells_drawn);
                term_counters::add_text_calls(text_calls);
                term_counters::add_bg_rects(bg_rects);
            }
            #[cfg(feature = "term-prof")]
            drop(_text_enter);
        }

        // Cursor rendering (shape/blink/color/viewport position from render-state).
        #[cfg(feature = "term-prof")]
        let _cur_span = tracing::info_span!("vt.paint.cursor").entered();
        if let Ok(cursor) = vt.cursor_state() {
            if cursor.visible && cursor.has_pos {
                let blink_on = if cursor.blinking {
                    let t = ui.input(|i| i.time) as f64;
                    (t * 2.0).floor() as i64 % 2 == 0
                } else {
                    true
                };
                // If cursor is blinking, we must keep a minimal repaint cadence even when idle,
                // otherwise the cursor will "freeze" on screen.
                if cursor.blinking {
                    ui.ctx().request_repaint_after(Duration::from_millis(250));
                }
                if blink_on {
                    let cx = cursor.x as f32;
                    let cy = cursor.y as f32;
                    let cur_x = x + cx * char_w;
                    let cur_y = rect.top() + padding.y + cy * line_h;
                    let col = egui::Color32::from_rgba_unmultiplied(
                        cursor.color.r,
                        cursor.color.g,
                        cursor.color.b,
                        160,
                    );

                    match cursor.visual_style {
                        vtffi::GhosttyRenderStateCursorVisualStyle_GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_UNDERLINE => {
                            let y_ul = cur_y + line_h - 2.0;
                            ui.painter().line_segment(
                                [egui::pos2(cur_x, y_ul), egui::pos2(cur_x + char_w, y_ul)],
                                egui::Stroke::new(2.0, col),
                            );
                        }
                        vtffi::GhosttyRenderStateCursorVisualStyle_GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BAR => {
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    egui::pos2(cur_x, cur_y),
                                    egui::vec2(2.0, line_h),
                                ),
                                0.0,
                                col,
                            );
                        }
                        _ => {
                            // Block / hollow block: draw a filled block.
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    egui::pos2(cur_x, cur_y),
                                    egui::vec2(char_w, line_h),
                                ),
                                0.0,
                                col,
                            );
                        }
                    }
                }
            }
        }
        #[cfg(feature = "term-prof")]
        drop(_cur_span);

        #[cfg(debug_assertions)]
        {
            if let Some(err) = &self.last_snapshot_err {
                ui.painter().text(
                    rect.left_top() + egui::vec2(TERMINAL_PADDING_X, TERMINAL_PADDING_Y + 50.0),
                    egui::Align2::LEFT_TOP,
                    format!("VT-SNAPSHOT-ERR {}", err),
                    egui::FontId::monospace(11.0),
                    egui::Color32::from_rgb(255, 120, 120),
                );
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.vt.is_some()
    }

    pub fn snapshot_plain_preview(&mut self, max_lines: usize) -> Option<String> {
        let vt = self.vt.as_mut()?;
        let lines = vt.snapshot_plain_lines_and_clear_dirty().ok()?;
        if lines.is_empty() {
            return None;
        }
        let start = lines.len().saturating_sub(max_lines);
        Some(lines[start..].join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_can_create_and_receive_bytes() {
        let mut w = VtTerminalWidget::new();
        w.ensure_created(10, 5);
        assert!(w.vt.is_some(), "vt should be created in tests");
        w.on_resize_cells(11, 5);
        w.on_ssh_bytes(b"hello");
    }

    #[test]
    fn gpu_mode_decision_requires_all_conditions() {
        assert!(should_use_gpu_terminal_text(true, true, true, true));
        assert!(!should_use_gpu_terminal_text(false, true, true, true));
        assert!(!should_use_gpu_terminal_text(true, false, true, true));
        assert!(!should_use_gpu_terminal_text(true, true, false, true));
        assert!(!should_use_gpu_terminal_text(true, true, true, false));
    }
}

