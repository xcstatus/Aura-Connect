//! libghostty FFI bindings for terminal VT parsing and rendering.
//!
//! # FFI Safety Notes
//!
//! This module wraps C APIs from the ghostty terminal emulator via bindgen-generated
//! FFI bindings. The following invariants must be maintained:
//!
//! - All `ghostty_*_new` functions allocate resources that must be freed via their
//!   corresponding `ghostty_*_free` functions in `Drop` implementation.
//! - Out-parameters (e.g., `&mut ptr`) are initialized by the C library; reading them
//!   before the library writes to them is UB.
//! - Pointer validity: all `*_ptr` fields must remain valid for the lifetime of
//!   `GhosttyVtTerminal`. They are internal to the C library and must not be aliased
//!   from Rust code.
//! - `GhosttyCell` and `GhosttyStyle` are FFI structs with no Drop semantics; they
//!   are passed by value and must be initialized before use via the library's getter
//!   functions.
//!
//! All FFI calls are wrapped in `unsafe {}` blocks. Callers must ensure the preconditions
//! documented above are met.

use std::ffi::c_void;
#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(debug_assertions)]
use std::time::Instant;

#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[allow(dead_code)]
pub(crate) mod ffi {
    include!(concat!(env!("OUT_DIR"), "/ghostty_vt_bindings.rs"));
}

pub struct GhosttyVtTerminal {
    terminal: ffi::GhosttyTerminal_ptr,
    render_state: ffi::GhosttyRenderState_ptr,
    row_iter: ffi::GhosttyRenderStateRowIterator_ptr,
    row_cells: ffi::GhosttyRenderStateRowCells_ptr,
    key_encoder: ffi::GhosttyKeyEncoder_ptr,
    key_event: ffi::GhosttyKeyEvent_ptr,
    cols: u16,
    rows: u16,
    /// Stack buffer for small graphemes (≤16 codepoints), avoids heap allocation.
    grapheme_buf: [u32; 16],
    /// Valid length in grapheme_buf.
    grapheme_len: usize,
    /// Fallback heap buffer for graphemes exceeding the stack buffer size.
    grapheme_scratch: Vec<u32>,
}

#[derive(Clone, Copy, Debug)]
pub struct CursorState {
    pub visible: bool,
    pub blinking: bool,
    pub has_pos: bool,
    pub x: u16,
    pub y: u16,
    pub visual_style: ffi::GhosttyRenderStateCursorVisualStyle,
    pub color: ffi::GhosttyColorRgb,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ScrollbarState {
    pub total_rows: u64,
    pub offset_rows: u64,
    pub viewport_rows: u64,
}

/// Helper to pack DEC private mode values.
/// Bits 0..14: value, bit 15: ansi flag (0 for DEC private).
fn ghostty_dec_mode(value: u16) -> ffi::GhosttyMode {
    (value & 0x7fff) as ffi::GhosttyMode
}

#[derive(Clone, Debug)]
pub struct VtStyledCell {
    /// Grapheme for the base cell. For empty/continuation cells this will be `" "`.
    pub text: String,
    /// True when this cell is a continuation slot of a wide glyph.
    pub continuation: bool,
}

impl Default for VtStyledCell {
    fn default() -> Self {
        Self {
            text: " ".to_string(),
            continuation: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct VtStyledRun {
    /// Per-cell payload for this run. `cells.len()` defines the column span.
    pub cells: Vec<VtStyledCell>,
    pub fg: ffi::GhosttyColorRgb,
    pub bg: ffi::GhosttyColorRgb,
    pub has_bg: bool,
    pub bold: bool,
    pub underline: bool,
    pub dim: bool,
    pub strikethrough: bool,
}

impl Default for VtStyledRun {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            fg: ffi::GhosttyColorRgb {
                r: 230,
                g: 230,
                b: 230,
            },
            bg: ffi::GhosttyColorRgb {
                r: 10,
                g: 10,
                b: 15,
            },
            has_bg: false,
            bold: false,
            underline: false,
            dim: false,
            strikethrough: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct VtStyledRow {
    pub runs: Vec<VtStyledRun>,
}

impl GhosttyVtTerminal {
    pub fn new(cols: u16, rows: u16, max_scrollback: usize) -> anyhow::Result<Self> {
        unsafe {
            let mut terminal: ffi::GhosttyTerminal_ptr = std::ptr::null_mut();
            let options = ffi::GhosttyTerminalOptions {
                cols,
                rows,
                max_scrollback,
            };
            let res = ffi::ghostty_terminal_new(std::ptr::null(), &mut terminal, options);
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                anyhow::bail!("ghostty_terminal_new failed: {}", res);
            }

            let mut render_state: ffi::GhosttyRenderState_ptr = std::ptr::null_mut();
            let res = ffi::ghostty_render_state_new(std::ptr::null(), &mut render_state);
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                ffi::ghostty_terminal_free(terminal);
                anyhow::bail!("ghostty_render_state_new failed: {}", res);
            }

            let mut row_iter: ffi::GhosttyRenderStateRowIterator_ptr = std::ptr::null_mut();
            let res = ffi::ghostty_render_state_row_iterator_new(std::ptr::null(), &mut row_iter);
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                ffi::ghostty_render_state_free(render_state);
                ffi::ghostty_terminal_free(terminal);
                anyhow::bail!("ghostty_render_state_row_iterator_new failed: {}", res);
            }

            let mut row_cells: ffi::GhosttyRenderStateRowCells_ptr = std::ptr::null_mut();
            let res = ffi::ghostty_render_state_row_cells_new(std::ptr::null(), &mut row_cells);
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                ffi::ghostty_render_state_row_iterator_free(row_iter);
                ffi::ghostty_render_state_free(render_state);
                ffi::ghostty_terminal_free(terminal);
                anyhow::bail!("ghostty_render_state_row_cells_new failed: {}", res);
            }

            let mut s = Self {
                terminal,
                render_state,
                row_iter,
                row_cells,
                key_encoder: std::ptr::null_mut(),
                key_event: std::ptr::null_mut(),
                cols,
                rows,
                grapheme_buf: [0; 16],
                grapheme_len: 0,
                grapheme_scratch: Vec::new(),
            };
            s.init_key_encoder()?;
            s.update_render_state()?;
            Ok(s)
        }
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// Write VT bytes into the terminal state machine.
    ///
    /// Performance note: this does **not** update the render state snapshot.
    /// Call `update_render_state()` once per frame (or after batching writes).
    pub fn write_vt(&mut self, bytes: &[u8]) {
        unsafe {
            ffi::ghostty_terminal_vt_write(self.terminal, bytes.as_ptr(), bytes.len());
        }
    }

    /// Scroll the viewport by delta rows (negative = up).
    ///
    /// Performance note: does **not** update render state snapshot.
    pub fn scroll_viewport_delta_rows(&mut self, delta_rows: isize) {
        unsafe {
            let behavior = ffi::GhosttyTerminalScrollViewport {
                tag: ffi::GhosttyTerminalScrollViewportTag_GHOSTTY_SCROLL_VIEWPORT_DELTA,
                value: ffi::GhosttyTerminalScrollViewportValue { delta: delta_rows },
            };
            ffi::ghostty_terminal_scroll_viewport(self.terminal, behavior);
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> anyhow::Result<()> {
        if cols == 0 || rows == 0 {
            return Ok(());
        }
        if cols == self.cols && rows == self.rows {
            return Ok(());
        }
        unsafe {
            let res = ffi::ghostty_terminal_resize(self.terminal, cols, rows);
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                anyhow::bail!("ghostty_terminal_resize failed: {}", res);
            }
        }
        self.cols = cols;
        self.rows = rows;
        Ok(())
    }

    pub fn update_render_state(&mut self) -> anyhow::Result<()> {
        unsafe {
            let res = ffi::ghostty_render_state_update(self.render_state, self.terminal);
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                anyhow::bail!("ghostty_render_state_update failed: {}", res);
            }
        }
        Ok(())
    }

    pub fn mode_get(&self, mode: ffi::GhosttyMode) -> anyhow::Result<bool> {
        unsafe {
            let mut v: bool = false;
            let res = ffi::ghostty_terminal_mode_get(self.terminal, mode, &mut v as *mut _);
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                anyhow::bail!("ghostty_terminal_mode_get failed: {}", res);
            }
            Ok(v)
        }
    }

    pub fn encode_paste(&mut self, text: &str) -> Vec<u8> {
        let bracketed = self.mode_get(ghostty_dec_mode(2004)).unwrap_or(false);

        // Safety check (best-effort): still allow paste, but prefer bracketed
        // paste when enabled by the remote application.
        let safe = unsafe { ffi::ghostty_paste_is_safe(text.as_ptr() as *const _, text.len()) };

        if bracketed {
            let mut out = Vec::with_capacity(text.len() + 16);
            out.extend_from_slice(b"\x1b[200~");
            out.extend_from_slice(text.as_bytes());
            out.extend_from_slice(b"\x1b[201~");
            out
        } else if safe {
            text.as_bytes().to_vec()
        } else {
            text.as_bytes().to_vec()
        }
    }

    pub fn encode_focus_event(&mut self, focused: bool) -> Vec<u8> {
        let focus_mode = self.mode_get(ghostty_dec_mode(1004)).unwrap_or(false);
        if !focus_mode {
            return Vec::new();
        }

        let ev = if focused {
            ffi::GhosttyFocusEvent_GHOSTTY_FOCUS_GAINED
        } else {
            ffi::GhosttyFocusEvent_GHOSTTY_FOCUS_LOST
        };

        // CSI I / CSI O are tiny, but handle OUT_OF_SPACE anyway.
        let mut buf = [0u8; 8];
        let mut written: usize = 0;
        let res = unsafe {
            ffi::ghostty_focus_encode(ev, buf.as_mut_ptr() as *mut _, buf.len(), &mut written)
        };
        if res == ffi::GhosttyResult_GHOSTTY_SUCCESS {
            buf[..written].to_vec()
        } else if res == ffi::GhosttyResult_GHOSTTY_OUT_OF_SPACE && written > 0 {
            let mut v = vec![0u8; written];
            let mut written2: usize = 0;
            let res2 = unsafe {
                ffi::ghostty_focus_encode(ev, v.as_mut_ptr() as *mut _, v.len(), &mut written2)
            };
            if res2 == ffi::GhosttyResult_GHOSTTY_SUCCESS {
                v.truncate(written2);
                v
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    /// Scrollbar / scrollback geometry for rendering a scrollbar UI.
    ///
    /// Note: upstream docs mention this can be expensive depending on viewport pinning.
    pub fn scrollbar_state(&self) -> anyhow::Result<ScrollbarState> {
        unsafe {
            let mut sb = ffi::GhosttyTerminalScrollbar {
                total: 0,
                offset: 0,
                len: 0,
            };
            let res = ffi::ghostty_terminal_get(
                self.terminal,
                ffi::GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_SCROLLBAR,
                &mut sb as *mut _ as *mut c_void,
            );
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                anyhow::bail!("ghostty_terminal_get(scrollbar) failed: {}", res);
            }
            Ok(ScrollbarState {
                total_rows: sb.total,
                offset_rows: sb.offset,
                viewport_rows: sb.len,
            })
        }
    }

    pub fn cursor_state(&mut self) -> anyhow::Result<CursorState> {
        unsafe {
            let mut visible: bool = false;
            let _ = ffi::ghostty_render_state_get(
                self.render_state,
                ffi::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_CURSOR_VISIBLE,
                &mut visible as *mut _ as *mut c_void,
            );

            let mut blinking: bool = false;
            let _ = ffi::ghostty_render_state_get(
                self.render_state,
                ffi::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_CURSOR_BLINKING,
                &mut blinking as *mut _ as *mut c_void,
            );

            let mut has_pos: bool = false;
            let _ = ffi::ghostty_render_state_get(
                self.render_state,
                ffi::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_HAS_VALUE,
                &mut has_pos as *mut _ as *mut c_void,
            );

            let mut x: u16 = 0;
            let mut y: u16 = 0;
            if has_pos {
                let _ = ffi::ghostty_render_state_get(
                    self.render_state,
                    ffi::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_X,
                    &mut x as *mut _ as *mut c_void,
                );
                let _ = ffi::ghostty_render_state_get(
                    self.render_state,
                    ffi::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_Y,
                    &mut y as *mut _ as *mut c_void,
                );
            }

            let mut visual_style: ffi::GhosttyRenderStateCursorVisualStyle =
                ffi::GhosttyRenderStateCursorVisualStyle_GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK;
            let _ = ffi::ghostty_render_state_get(
                self.render_state,
                ffi::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_CURSOR_VISUAL_STYLE,
                &mut visual_style as *mut _ as *mut c_void,
            );

            // Cursor color: explicit cursor color if provided, else terminal fg.
            let mut colors = ffi::GhosttyRenderStateColors {
                size: std::mem::size_of::<ffi::GhosttyRenderStateColors>(),
                background: ffi::GhosttyColorRgb {
                    r: 10,
                    g: 10,
                    b: 15,
                },
                foreground: ffi::GhosttyColorRgb {
                    r: 230,
                    g: 230,
                    b: 230,
                },
                cursor: ffi::GhosttyColorRgb {
                    r: 230,
                    g: 230,
                    b: 230,
                },
                cursor_has_value: false,
                palette: [ffi::GhosttyColorRgb { r: 0, g: 0, b: 0 }; 256],
            };
            let _ = ffi::ghostty_render_state_colors_get(self.render_state, &mut colors);
            let color = if colors.cursor_has_value {
                colors.cursor
            } else {
                colors.foreground
            };

            Ok(CursorState {
                visible,
                blinking,
                has_pos,
                x,
                y,
                visual_style,
                color,
            })
        }
    }

    pub fn extract_viewport_text(
        &mut self,
        start: (u16, u16),
        end: (u16, u16),
    ) -> anyhow::Result<String> {
        let (mut x0, mut y0) = start;
        let (mut x1, mut y1) = end;
        if (y1, x1) < (y0, x0) {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }

        unsafe {
            let res = ffi::ghostty_render_state_get(
                self.render_state,
                ffi::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR,
                (&mut self.row_iter as *mut ffi::GhosttyRenderStateRowIterator_ptr) as *mut c_void,
            );
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                anyhow::bail!("ghostty_render_state_get(row_iterator) failed: {}", res);
            }

            let mut out = String::new();
            let rows = self.rows;
            let cols = self.cols;

            for y in 0..rows {
                if !ffi::ghostty_render_state_row_iterator_next(self.row_iter) {
                    break;
                }
                if y < y0 || y > y1 {
                    continue;
                }

                let res = ffi::ghostty_render_state_row_get(
                    self.row_iter,
                    ffi::GhosttyRenderStateRowData_GHOSTTY_RENDER_STATE_ROW_DATA_CELLS,
                    (&mut self.row_cells as *mut ffi::GhosttyRenderStateRowCells_ptr)
                        as *mut c_void,
                );
                if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                    anyhow::bail!("ghostty_render_state_row_get(cells) failed: {}", res);
                }

                let (row_x0, row_x1) = if y0 == y1 {
                    (
                        x0.min(cols.saturating_sub(1)),
                        x1.min(cols.saturating_sub(1)),
                    )
                } else if y == y0 {
                    (x0.min(cols.saturating_sub(1)), cols.saturating_sub(1))
                } else if y == y1 {
                    (0, x1.min(cols.saturating_sub(1)))
                } else {
                    (0, cols.saturating_sub(1))
                };

                let mut line = String::new();
                for x in row_x0..=row_x1 {
                    let res = ffi::ghostty_render_state_row_cells_select(self.row_cells, x);
                    if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                        line.push(' ');
                        continue;
                    }

                    let mut len_u32: u32 = 0;
                    let res = ffi::ghostty_render_state_row_cells_get(
                        self.row_cells,
                        ffi::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_LEN,
                        &mut len_u32 as *mut _ as *mut c_void,
                    );
                    if res != ffi::GhosttyResult_GHOSTTY_SUCCESS || len_u32 == 0 {
                        line.push(' ');
                        continue;
                    }

                    self.grapheme_scratch.clear();
                    self.grapheme_scratch.resize(len_u32 as usize, 0);
                    let res = ffi::ghostty_render_state_row_cells_get(
                        self.row_cells,
                        ffi::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_BUF,
                        self.grapheme_scratch.as_mut_ptr() as *mut c_void,
                    );
                    if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                        line.push(' ');
                        continue;
                    }

                    let ch = char::from_u32(self.grapheme_scratch[0]).unwrap_or(' ');
                    line.push(ch);
                }

                // Keep selection output reasonably clean (trim right), but preserve
                // internal spaces.
                out.push_str(line.trim_end_matches(' '));
                if y != y1 {
                    out.push('\n');
                }
            }

            Ok(out)
        }
    }

    fn init_key_encoder(&mut self) -> anyhow::Result<()> {
        unsafe {
            let mut enc: ffi::GhosttyKeyEncoder_ptr = std::ptr::null_mut();
            let res = ffi::ghostty_key_encoder_new(std::ptr::null(), &mut enc);
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                anyhow::bail!("ghostty_key_encoder_new failed: {}", res);
            }

            let mut ev: ffi::GhosttyKeyEvent_ptr = std::ptr::null_mut();
            let res = ffi::ghostty_key_event_new(std::ptr::null(), &mut ev);
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                ffi::ghostty_key_encoder_free(enc);
                anyhow::bail!("ghostty_key_event_new failed: {}", res);
            }

            self.key_encoder = enc;
            self.key_event = ev;
            Ok(())
        }
    }

    pub fn encode_key(
        &mut self,
        action: ffi::GhosttyKeyAction,
        key: ffi::GhosttyKey,
        mods: ffi::GhosttyMods,
    ) -> anyhow::Result<Vec<u8>> {
        self.encode_key_with_utf8(action, key, mods, None)
    }

    /// When `utf8` is set, Ghostty may use it together with `key` + `mods` (e.g. Alt + grapheme).
    pub fn encode_key_with_utf8(
        &mut self,
        action: ffi::GhosttyKeyAction,
        key: ffi::GhosttyKey,
        mods: ffi::GhosttyMods,
        utf8: Option<&str>,
    ) -> anyhow::Result<Vec<u8>> {
        unsafe {
            ffi::ghostty_key_encoder_setopt_from_terminal(self.key_encoder, self.terminal);

            ffi::ghostty_key_event_set_action(self.key_event, action);
            ffi::ghostty_key_event_set_key(self.key_event, key);
            ffi::ghostty_key_event_set_mods(self.key_event, mods);
            ffi::ghostty_key_event_set_consumed_mods(self.key_event, 0);
            ffi::ghostty_key_event_set_composing(self.key_event, false);
            match utf8 {
                Some(s) if !s.is_empty() => {
                    ffi::ghostty_key_event_set_utf8(
                        self.key_event,
                        s.as_ptr() as *const std::os::raw::c_char,
                        s.len(),
                    );
                }
                _ => {
                    ffi::ghostty_key_event_set_utf8(self.key_event, std::ptr::null(), 0);
                }
            }
            ffi::ghostty_key_event_set_unshifted_codepoint(self.key_event, 0);

            let mut required: usize = 0;
            let res = ffi::ghostty_key_encoder_encode(
                self.key_encoder,
                self.key_event,
                std::ptr::null_mut(),
                0,
                &mut required,
            );
            if res == ffi::GhosttyResult_GHOSTTY_SUCCESS {
                return Ok(Vec::new());
            }

            if required == 0 {
                anyhow::bail!("ghostty_key_encoder_encode failed: {}", res);
            }

            let mut buf: Vec<u8> = vec![0u8; required];
            let mut written: usize = 0;
            let res = ffi::ghostty_key_encoder_encode(
                self.key_encoder,
                self.key_event,
                buf.as_mut_ptr() as *mut std::os::raw::c_char,
                buf.len(),
                &mut written,
            );
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                anyhow::bail!("ghostty_key_encoder_encode failed: {}", res);
            }
            buf.truncate(written);
            Ok(buf)
        }
    }

    pub fn dirty(&self) -> anyhow::Result<ffi::GhosttyRenderStateDirty> {
        unsafe {
            let mut dirty: ffi::GhosttyRenderStateDirty =
                ffi::GhosttyRenderStateDirty_GHOSTTY_RENDER_STATE_DIRTY_FALSE;
            let res = ffi::ghostty_render_state_get(
                self.render_state,
                ffi::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_DIRTY,
                &mut dirty as *mut _ as *mut c_void,
            );
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                anyhow::bail!("ghostty_render_state_get(dirty) failed: {}", res);
            }
            Ok(dirty)
        }
    }

    pub fn snapshot_plain_lines_and_clear_dirty(&mut self) -> anyhow::Result<Vec<String>> {
        unsafe {
            // Populate the iterator with the current render state's rows.
            let res = ffi::ghostty_render_state_get(
                self.render_state,
                ffi::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR,
                (&mut self.row_iter as *mut ffi::GhosttyRenderStateRowIterator_ptr) as *mut c_void,
            );
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                anyhow::bail!("ghostty_render_state_get(row_iterator) failed: {}", res);
            }

            let mut lines: Vec<String> = Vec::with_capacity(self.rows as usize);

            while ffi::ghostty_render_state_row_iterator_next(self.row_iter) {
                // Get cells container for this row.
                let res = ffi::ghostty_render_state_row_get(
                    self.row_iter,
                    ffi::GhosttyRenderStateRowData_GHOSTTY_RENDER_STATE_ROW_DATA_CELLS,
                    (&mut self.row_cells as *mut ffi::GhosttyRenderStateRowCells_ptr)
                        as *mut c_void,
                );
                if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                    anyhow::bail!("ghostty_render_state_row_get(cells) failed: {}", res);
                }

                let mut line = String::with_capacity(self.cols as usize);
                while ffi::ghostty_render_state_row_cells_next(self.row_cells) {
                    let mut len_u32: u32 = 0;
                    let res = ffi::ghostty_render_state_row_cells_get(
                        self.row_cells,
                        ffi::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_LEN,
                        &mut len_u32 as *mut _ as *mut c_void,
                    );
                    if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                        // Treat as empty cell.
                        line.push(' ');
                        continue;
                    }
                    if len_u32 == 0 {
                        line.push(' ');
                        continue;
                    }

                    // Read graphemes buffer (base + combining marks).
                    self.grapheme_scratch.clear();
                    self.grapheme_scratch.resize(len_u32 as usize, 0);
                    let res = ffi::ghostty_render_state_row_cells_get(
                        self.row_cells,
                        ffi::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_BUF,
                        self.grapheme_scratch.as_mut_ptr() as *mut c_void,
                    );
                    if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                        line.push(' ');
                        continue;
                    }

                    // Best-effort: take the first codepoint as the displayed character.
                    let ch = char::from_u32(self.grapheme_scratch[0]).unwrap_or(' ');
                    line.push(ch);
                }

                // Trim trailing spaces to keep strings shorter for UI.
                let trimmed = line.trim_end_matches(' ').to_string();
                lines.push(trimmed);

                // Clear row dirty flag.
                let row_dirty_false: bool = false;
                let _ = ffi::ghostty_render_state_row_set(
                    self.row_iter,
                    ffi::GhosttyRenderStateRowOption_GHOSTTY_RENDER_STATE_ROW_OPTION_DIRTY,
                    &row_dirty_false as *const _ as *const c_void,
                );
            }

            // Clear global dirty flag.
            let dirty_false: ffi::GhosttyRenderStateDirty =
                ffi::GhosttyRenderStateDirty_GHOSTTY_RENDER_STATE_DIRTY_FALSE;
            let _ = ffi::ghostty_render_state_set(
                self.render_state,
                ffi::GhosttyRenderStateOption_GHOSTTY_RENDER_STATE_OPTION_DIRTY,
                &dirty_false as *const _ as *const c_void,
            );

            Ok(lines)
        }
    }

    pub fn snapshot_styled_rows_and_clear_dirty(&mut self) -> anyhow::Result<Vec<VtStyledRow>> {
        let mut rows: Vec<VtStyledRow> = vec![VtStyledRow::default(); self.rows as usize];
        self.update_dirty_styled_rows_and_clear_dirty(&mut rows, false)?;
        Ok(rows)
    }

    /// Update only dirty rows into `rows` (resized to `self.rows`).
    /// If `only_dirty` is false, all rows will be regenerated.
    pub fn update_dirty_styled_rows_and_clear_dirty(
        &mut self,
        rows: &mut Vec<VtStyledRow>,
        only_dirty: bool,
    ) -> anyhow::Result<()> {
        self.update_dirty_styled_rows_and_clear_dirty_collect(rows, only_dirty, None)
    }

    /// Like `update_dirty_styled_rows_and_clear_dirty`, but can optionally collect which row indices were updated.
    pub fn update_dirty_styled_rows_and_clear_dirty_collect(
        &mut self,
        rows: &mut Vec<VtStyledRow>,
        only_dirty: bool,
        mut updated_rows_out: Option<&mut Vec<usize>>,
    ) -> anyhow::Result<()> {
        #[cfg(feature = "term-prof")]
        let _span = tracing::info_span!(
            "vt.update_dirty_styled_rows_and_clear_dirty",
            only_dirty = only_dirty,
            cols = self.cols,
            rows = self.rows
        )
        .entered();

        #[cfg(debug_assertions)]
        let prof_started = Instant::now();
        #[cfg(debug_assertions)]
        let mut prof_total_rows: usize = 0;
        #[cfg(debug_assertions)]
        let mut prof_dirty_rows: usize = 0;
        #[cfg(debug_assertions)]
        let mut prof_cells: usize = 0;

        let res = unsafe {
            if rows.len() != self.rows as usize {
                rows.resize_with(self.rows as usize, VtStyledRow::default);
            }

            // Defaults (terminal fg/bg) for cells without explicit colors.
            let mut colors = ffi::GhosttyRenderStateColors {
                size: std::mem::size_of::<ffi::GhosttyRenderStateColors>(),
                background: ffi::GhosttyColorRgb {
                    r: 10,
                    g: 10,
                    b: 15,
                },
                foreground: ffi::GhosttyColorRgb {
                    r: 230,
                    g: 230,
                    b: 230,
                },
                cursor: ffi::GhosttyColorRgb { r: 0, g: 0, b: 0 },
                cursor_has_value: false,
                palette: [ffi::GhosttyColorRgb { r: 0, g: 0, b: 0 }; 256],
            };
            let _ = ffi::ghostty_render_state_colors_get(self.render_state, &mut colors);

            // Optimization 1: Early exit when frame is completely clean.
            if only_dirty {
                let mut global_dirty =
                    ffi::GhosttyRenderStateDirty_GHOSTTY_RENDER_STATE_DIRTY_FALSE;
                let _ = ffi::ghostty_render_state_get(
                    self.render_state,
                    ffi::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_DIRTY,
                    &mut global_dirty as *mut _ as *mut c_void,
                );
                if global_dirty == ffi::GhosttyRenderStateDirty_GHOSTTY_RENDER_STATE_DIRTY_FALSE {
                    return Ok(());
                }
            }

            let res = ffi::ghostty_render_state_get(
                self.render_state,
                ffi::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR,
                (&mut self.row_iter as *mut ffi::GhosttyRenderStateRowIterator_ptr) as *mut c_void,
            );
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                anyhow::bail!("ghostty_render_state_get(row_iterator) failed: {}", res);
            }

            let cols = self.cols as usize;
            let rows_len = self.rows as usize;
            let mut y: usize = 0;
            while y < rows_len && ffi::ghostty_render_state_row_iterator_next(self.row_iter) {
                #[cfg(debug_assertions)]
                {
                    prof_total_rows += 1;
                }
                let mut row_dirty: bool = false;
                let res = ffi::ghostty_render_state_row_get(
                    self.row_iter,
                    ffi::GhosttyRenderStateRowData_GHOSTTY_RENDER_STATE_ROW_DATA_DIRTY,
                    &mut row_dirty as *mut _ as *mut c_void,
                );
                if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                    anyhow::bail!("ghostty_render_state_row_get(dirty) failed: {}", res);
                }
                #[cfg(debug_assertions)]
                {
                    if row_dirty {
                        prof_dirty_rows += 1;
                    }
                }

                if only_dirty && !row_dirty {
                    y += 1;
                    continue;
                }

                if let Some(out) = updated_rows_out.as_deref_mut() {
                    out.push(y);
                }

                // Get cells for this row.
                let res = ffi::ghostty_render_state_row_get(
                    self.row_iter,
                    ffi::GhosttyRenderStateRowData_GHOSTTY_RENDER_STATE_ROW_DATA_CELLS,
                    (&mut self.row_cells as *mut ffi::GhosttyRenderStateRowCells_ptr)
                        as *mut c_void,
                );
                if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                    anyhow::bail!("ghostty_render_state_row_get(cells) failed: {}", res);
                }

                // Reuse per-row run buffer to avoid reallocating/dropping a Vec
                // for every dirty row on every frame.
                let out_runs = &mut rows[y].runs;
                out_runs.clear();
                let mut x: usize = 0;
                while x < cols && ffi::ghostty_render_state_row_cells_next(self.row_cells) {
                    // Raw cell metadata (width / spacer markers).
                    // SAFETY: GhosttyCell is a plain FFI struct (no interior mutability, no Drop).
                    // We initialize it with zeroed bytes because the C library will overwrite
                    // all fields via ghostty_render_state_row_cells_get before we read them.
                    let mut raw_cell: ffi::GhosttyCell = std::mem::zeroed();
                    let _ = ffi::ghostty_render_state_row_cells_get(
                        self.row_cells,
                        ffi::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_RAW,
                        &mut raw_cell as *mut _ as *mut c_void,
                    );
                    let mut wide: ffi::GhosttyCellWide =
                        ffi::GhosttyCellWide_GHOSTTY_CELL_WIDE_NARROW;
                    let _ = ffi::ghostty_cell_get(
                        raw_cell,
                        ffi::GhosttyCellData_GHOSTTY_CELL_DATA_WIDE,
                        &mut wide as *mut _ as *mut c_void,
                    );

                    // Grapheme len
                    let mut len_u32: u32 = 0;
                    let res = ffi::ghostty_render_state_row_cells_get(
                        self.row_cells,
                        ffi::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_LEN,
                        &mut len_u32 as *mut _ as *mut c_void,
                    );

                    // Style
                    let mut style = ffi::GhosttyStyle {
                        size: std::mem::size_of::<ffi::GhosttyStyle>(),
                        fg_color: ffi::GhosttyStyleColor {
                            tag: 0,
                            value: ffi::GhosttyStyleColorValue { _padding: 0 },
                        },
                        bg_color: ffi::GhosttyStyleColor {
                            tag: 0,
                            value: ffi::GhosttyStyleColorValue { _padding: 0 },
                        },
                        underline_color: ffi::GhosttyStyleColor {
                            tag: 0,
                            value: ffi::GhosttyStyleColorValue { _padding: 0 },
                        },
                        bold: false,
                        italic: false,
                        faint: false,
                        blink: false,
                        inverse: false,
                        invisible: false,
                        strikethrough: false,
                        overline: false,
                        underline: 0,
                    };
                    let _ = ffi::ghostty_render_state_row_cells_get(
                        self.row_cells,
                        ffi::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_STYLE,
                        &mut style as *mut _ as *mut c_void,
                    );

                    // Optimization 2: Run-level color caching.
                    // We fetch colors from FFI only when style changes, then reuse within the same run.
                    let bold = style.bold;
                    let underline = style.underline != 0;
                    let dim = style.faint;
                    let strikethrough = style.strikethrough;

                    // Determine if we need FFI call based on style changes vs previous run.
                    let needs_style_check = match out_runs.last() {
                        None => true, // First cell: must FFI
                        Some(last) => {
                            last.bold != bold
                                || last.underline != underline
                                || last.dim != dim
                                || last.strikethrough != strikethrough
                        }
                    };

                    let mut fg: ffi::GhosttyColorRgb;
                    let mut bg: ffi::GhosttyColorRgb;
                    let mut has_bg: bool;

                    if needs_style_check {
                        // Fetch colors from FFI (expensive).
                        fg = colors.foreground;
                        bg = colors.background;
                        let res_fg = ffi::ghostty_render_state_row_cells_get(
                            self.row_cells,
                            ffi::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_FG_COLOR,
                            &mut fg as *mut _ as *mut c_void,
                        );
                        if res_fg != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                            fg = colors.foreground;
                        }
                        let res_bg = ffi::ghostty_render_state_row_cells_get(
                            self.row_cells,
                            ffi::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_BG_COLOR,
                            &mut bg as *mut _ as *mut c_void,
                        );
                        if res_bg == ffi::GhosttyResult_GHOSTTY_SUCCESS {
                            has_bg = true;
                        } else {
                            bg = colors.background;
                            has_bg = false;
                        }
                        if style.inverse {
                            std::mem::swap(&mut fg, &mut bg);
                            has_bg = true;
                        }
                    } else {
                        // Style unchanged: reuse the last run's colors without FFI call.
                        let last_run = out_runs.last().unwrap();
                        fg = last_run.fg;
                        bg = last_run.bg;
                        has_bg = last_run.has_bg;
                    }

                    // Grapheme (may be multiple codepoints; keep full sequence for correctness).
                    //
                    // Styled rendering is modeled as an explicit **cell sequence**:
                    // - base cell: actual grapheme
                    // - continuation cell(s): `" "` + `continuation=true` (must occupy a column but be invisible)
                    let mut cell_text = String::new();
                    let continuation = matches!(
                        wide,
                        ffi::GhosttyCellWide_GHOSTTY_CELL_WIDE_SPACER_TAIL
                            | ffi::GhosttyCellWide_GHOSTTY_CELL_WIDE_SPACER_HEAD
                    );
                    if res != ffi::GhosttyResult_GHOSTTY_SUCCESS || len_u32 == 0 {
                        cell_text.push(' ');
                    } else {
                        // Optimization 4: Use stack buffer for small graphemes (≤16 codepoints),
                        // fallback to heap for rare large grapheme clusters.
                        let use_stack = len_u32 as usize <= 16;
                        let buf_slice: &mut [u32] = if use_stack {
                            self.grapheme_len = len_u32 as usize;
                            &mut self.grapheme_buf[..len_u32 as usize]
                        } else {
                            self.grapheme_scratch.clear();
                            self.grapheme_scratch.resize(len_u32 as usize, 0);
                            self.grapheme_scratch.as_mut_slice()
                        };
                        let res2 = ffi::ghostty_render_state_row_cells_get(
                            self.row_cells,
                            ffi::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_BUF,
                            buf_slice.as_mut_ptr() as *mut c_void,
                        );
                        if res2 == ffi::GhosttyResult_GHOSTTY_SUCCESS {
                            // Optimization 3: Pre-allocate String to avoid reallocation.
                            cell_text = String::with_capacity(len_u32 as usize);
                            for &cp in buf_slice.iter() {
                                if let Some(ch) = char::from_u32(cp) {
                                    cell_text.push(ch);
                                }
                            }
                            if cell_text.is_empty() {
                                cell_text.push(' ');
                            }
                        } else {
                            cell_text.push(' ');
                        }
                    }

                    // Group into runs.
                    let needs_new = match out_runs.last() {
                        None => true,
                        Some(last) => {
                            last.fg.r != fg.r
                                || last.fg.g != fg.g
                                || last.fg.b != fg.b
                                || last.bg.r != bg.r
                                || last.bg.g != bg.g
                                || last.bg.b != bg.b
                                || last.has_bg != has_bg
                                || last.bold != bold
                                || last.underline != underline
                                || last.dim != dim
                                || last.strikethrough != strikethrough
                        }
                    };

                    if needs_new {
                        out_runs.push(VtStyledRun {
                            cells: Vec::new(),
                            fg,
                            bg,
                            has_bg,
                            bold,
                            underline,
                            dim,
                            strikethrough,
                        });
                    }
                    let last = out_runs.last_mut().unwrap();
                    last.cells.push(VtStyledCell {
                        text: cell_text,
                        continuation,
                    });
                    x += 1;
                }
                #[cfg(debug_assertions)]
                {
                    prof_cells += x;
                }

                // Keep trailing spaces: full-screen TUIs (e.g. `top`) use them
                // to paint full-width bars/status rows.

                // Clear row dirty flag after "rendering" it.
                let row_dirty_false: bool = false;
                let _ = ffi::ghostty_render_state_row_set(
                    self.row_iter,
                    ffi::GhosttyRenderStateRowOption_GHOSTTY_RENDER_STATE_ROW_OPTION_DIRTY,
                    &row_dirty_false as *const _ as *const c_void,
                );

                y += 1;
            }

            // Clear global dirty flag.
            let dirty_false: ffi::GhosttyRenderStateDirty =
                ffi::GhosttyRenderStateDirty_GHOSTTY_RENDER_STATE_DIRTY_FALSE;
            let _ = ffi::ghostty_render_state_set(
                self.render_state,
                ffi::GhosttyRenderStateOption_GHOSTTY_RENDER_STATE_OPTION_DIRTY,
                &dirty_false as *const _ as *const c_void,
            );

            Ok(())
        };
        #[cfg(debug_assertions)]
        {
            // Throttle logs to avoid distorting measurements.
            static CALLS: AtomicUsize = AtomicUsize::new(0);
            let calls = CALLS.fetch_add(1, Ordering::Relaxed) + 1;
            let cost_ms = prof_started.elapsed().as_millis();
            // Log periodically, and always log slow frames.
            if (calls % 120 == 0) || cost_ms >= 8 {
                log::debug!(
                    target: "term-prof",
                    "[term-prof] vt.update_dirty_styled_rows only_dirty={} rows_seen={} dirty_rows_seen={} cells={} cost={}ms",
                    only_dirty,
                    prof_total_rows,
                    prof_dirty_rows,
                    prof_cells,
                    cost_ms
                );
            }
        }
        res
    }

    /// Incrementally update only dirty rows into an existing `lines` buffer.
    /// The buffer is resized to `self.rows` if needed. Rows that are not dirty are left unchanged.
    pub fn update_dirty_plain_lines_and_clear_dirty(
        &mut self,
        lines: &mut Vec<String>,
    ) -> anyhow::Result<()> {
        self.update_dirty_plain_lines_inner(lines, None)
    }

    /// Same as [`Self::update_dirty_plain_lines_and_clear_dirty`], and records viewport row indices
    /// that were refreshed (for UI-side incremental `join` patching).
    pub fn update_dirty_plain_lines_collect_dirty_rows(
        &mut self,
        lines: &mut Vec<String>,
        dirty_rows_out: &mut Vec<usize>,
    ) -> anyhow::Result<()> {
        dirty_rows_out.clear();
        self.update_dirty_plain_lines_inner(lines, Some(dirty_rows_out))
    }

    fn update_dirty_plain_lines_inner(
        &mut self,
        lines: &mut Vec<String>,
        mut dirty_rows_out: Option<&mut Vec<usize>>,
    ) -> anyhow::Result<()> {
        unsafe {
            if lines.len() != self.rows as usize {
                lines.resize_with(self.rows as usize, String::new);
            }

            // Populate iterator.
            let res = ffi::ghostty_render_state_get(
                self.render_state,
                ffi::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR,
                (&mut self.row_iter as *mut ffi::GhosttyRenderStateRowIterator_ptr) as *mut c_void,
            );
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                anyhow::bail!("ghostty_render_state_get(row_iterator) failed: {}", res);
            }

            let cols = self.cols as usize;
            let rows = self.rows as usize;
            let mut y: usize = 0;
            while y < rows && ffi::ghostty_render_state_row_iterator_next(self.row_iter) {
                let mut row_dirty: bool = false;
                let res = ffi::ghostty_render_state_row_get(
                    self.row_iter,
                    ffi::GhosttyRenderStateRowData_GHOSTTY_RENDER_STATE_ROW_DATA_DIRTY,
                    &mut row_dirty as *mut _ as *mut c_void,
                );
                if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                    anyhow::bail!("ghostty_render_state_row_get(dirty) failed: {}", res);
                }

                if row_dirty {
                    if let Some(out) = dirty_rows_out.as_mut() {
                        out.push(y);
                    }
                    // Get cells for this row.
                    let res = ffi::ghostty_render_state_row_get(
                        self.row_iter,
                        ffi::GhosttyRenderStateRowData_GHOSTTY_RENDER_STATE_ROW_DATA_CELLS,
                        (&mut self.row_cells as *mut ffi::GhosttyRenderStateRowCells_ptr)
                            as *mut c_void,
                    );
                    if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                        anyhow::bail!("ghostty_render_state_row_get(cells) failed: {}", res);
                    }

                    let mut line = String::with_capacity(cols);
                    let mut x: usize = 0;
                    while x < cols && ffi::ghostty_render_state_row_cells_next(self.row_cells) {
                        let mut len_u32: u32 = 0;
                        let res = ffi::ghostty_render_state_row_cells_get(
                            self.row_cells,
                            ffi::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_LEN,
                            &mut len_u32 as *mut _ as *mut c_void,
                        );
                        if res != ffi::GhosttyResult_GHOSTTY_SUCCESS || len_u32 == 0 {
                            line.push(' ');
                            x += 1;
                            continue;
                        }

                        // Read graphemes buffer.
                        self.grapheme_scratch.clear();
                        self.grapheme_scratch.resize(len_u32 as usize, 0);
                        let res = ffi::ghostty_render_state_row_cells_get(
                            self.row_cells,
                            ffi::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_BUF,
                            self.grapheme_scratch.as_mut_ptr() as *mut c_void,
                        );
                        if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                            line.push(' ');
                            x += 1;
                            continue;
                        }

                        let ch = char::from_u32(self.grapheme_scratch[0]).unwrap_or(' ');
                        line.push(ch);
                        x += 1;
                    }

                    lines[y] = line.trim_end_matches(' ').to_string();

                    // Clear row dirty flag after updating.
                    let row_dirty_false: bool = false;
                    let _ = ffi::ghostty_render_state_row_set(
                        self.row_iter,
                        ffi::GhosttyRenderStateRowOption_GHOSTTY_RENDER_STATE_ROW_OPTION_DIRTY,
                        &row_dirty_false as *const _ as *const c_void,
                    );
                }

                y += 1;
            }

            // Clear global dirty flag.
            let dirty_false: ffi::GhosttyRenderStateDirty =
                ffi::GhosttyRenderStateDirty_GHOSTTY_RENDER_STATE_DIRTY_FALSE;
            let _ = ffi::ghostty_render_state_set(
                self.render_state,
                ffi::GhosttyRenderStateOption_GHOSTTY_RENDER_STATE_OPTION_DIRTY,
                &dirty_false as *const _ as *const c_void,
            );

            Ok(())
        }
    }

    pub fn snapshot_codepoints_grid_and_clear_dirty(&mut self) -> anyhow::Result<Vec<u32>> {
        unsafe {
            // Populate the iterator with the current render state's rows.
            let res = ffi::ghostty_render_state_get(
                self.render_state,
                ffi::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR,
                (&mut self.row_iter as *mut ffi::GhosttyRenderStateRowIterator_ptr) as *mut c_void,
            );
            if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                anyhow::bail!("ghostty_render_state_get(row_iterator) failed: {}", res);
            }

            let cols = self.cols as usize;
            let rows = self.rows as usize;
            let mut grid: Vec<u32> = vec![0; cols.saturating_mul(rows)];

            let mut y: usize = 0;
            while y < rows && ffi::ghostty_render_state_row_iterator_next(self.row_iter) {
                // Get cells container for this row.
                let res = ffi::ghostty_render_state_row_get(
                    self.row_iter,
                    ffi::GhosttyRenderStateRowData_GHOSTTY_RENDER_STATE_ROW_DATA_CELLS,
                    (&mut self.row_cells as *mut ffi::GhosttyRenderStateRowCells_ptr)
                        as *mut c_void,
                );
                if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                    anyhow::bail!("ghostty_render_state_row_get(cells) failed: {}", res);
                }

                let mut x: usize = 0;
                while x < cols && ffi::ghostty_render_state_row_cells_next(self.row_cells) {
                    let mut len_u32: u32 = 0;
                    let res = ffi::ghostty_render_state_row_cells_get(
                        self.row_cells,
                        ffi::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_LEN,
                        &mut len_u32 as *mut _ as *mut c_void,
                    );

                    let codepoint = if res != ffi::GhosttyResult_GHOSTTY_SUCCESS || len_u32 == 0 {
                        0
                    } else {
                        self.grapheme_scratch.clear();
                        self.grapheme_scratch.resize(len_u32 as usize, 0);
                        let res = ffi::ghostty_render_state_row_cells_get(
                            self.row_cells,
                            ffi::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_BUF,
                            self.grapheme_scratch.as_mut_ptr() as *mut c_void,
                        );
                        if res != ffi::GhosttyResult_GHOSTTY_SUCCESS {
                            0
                        } else {
                            self.grapheme_scratch[0]
                        }
                    };

                    grid[y * cols + x] = codepoint;
                    x += 1;
                }

                // Clear row dirty flag.
                let row_dirty_false: bool = false;
                let _ = ffi::ghostty_render_state_row_set(
                    self.row_iter,
                    ffi::GhosttyRenderStateRowOption_GHOSTTY_RENDER_STATE_ROW_OPTION_DIRTY,
                    &row_dirty_false as *const _ as *const c_void,
                );

                y += 1;
            }

            // Clear global dirty flag.
            let dirty_false: ffi::GhosttyRenderStateDirty =
                ffi::GhosttyRenderStateDirty_GHOSTTY_RENDER_STATE_DIRTY_FALSE;
            let _ = ffi::ghostty_render_state_set(
                self.render_state,
                ffi::GhosttyRenderStateOption_GHOSTTY_RENDER_STATE_OPTION_DIRTY,
                &dirty_false as *const _ as *const c_void,
            );

            Ok(grid)
        }
    }
}

impl Drop for GhosttyVtTerminal {
    fn drop(&mut self) {
        unsafe {
            ffi::ghostty_key_event_free(self.key_event);
            ffi::ghostty_key_encoder_free(self.key_encoder);
            ffi::ghostty_render_state_row_cells_free(self.row_cells);
            ffi::ghostty_render_state_row_iterator_free(self.row_iter);
            ffi::ghostty_render_state_free(self.render_state);
            ffi::ghostty_terminal_free(self.terminal);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vt_new_write_and_snapshot_lines() {
        let mut vt = GhosttyVtTerminal::new(20, 4, 1000).expect("create vt");
        vt.write_vt(b"hello\r\nworld");
        vt.update_render_state().expect("update render state");
        let lines = vt
            .snapshot_plain_lines_and_clear_dirty()
            .expect("snapshot lines");
        let joined = lines.join("\n");
        assert!(joined.contains("hello"), "expected 'hello' in snapshot");
        assert!(joined.contains("world"), "expected 'world' in snapshot");
    }

    #[test]
    fn vt_snapshot_codepoints_grid_non_empty_after_write() {
        let mut vt = GhosttyVtTerminal::new(10, 2, 1000).expect("create vt");
        vt.write_vt(b"abc");
        vt.update_render_state().expect("update render state");
        let grid = vt
            .snapshot_codepoints_grid_and_clear_dirty()
            .expect("snapshot grid");
        assert_eq!(grid.len(), 10 * 2);
        assert!(
            grid.iter().any(|&cp| cp != 0),
            "expected some non-zero codepoints"
        );
    }

    #[test]
    fn vt_resize_updates_dimensions_and_snapshot_still_works() {
        let mut vt = GhosttyVtTerminal::new(5, 2, 1000).expect("create vt");
        vt.write_vt(b"x");
        vt.resize(8, 3).expect("resize");
        vt.update_render_state().expect("update render state");
        assert_eq!(vt.cols(), 8);
        assert_eq!(vt.rows(), 3);
        let grid = vt
            .snapshot_codepoints_grid_and_clear_dirty()
            .expect("snapshot grid after resize");
        assert_eq!(grid.len(), 8 * 3);
    }

    #[test]
    fn vt_encode_key_produces_bytes_for_enter() {
        let mut vt = GhosttyVtTerminal::new(10, 2, 1000).expect("create vt");
        let seq = vt
            .encode_key(
                ffi::GhosttyKeyAction_GHOSTTY_KEY_ACTION_PRESS,
                ffi::GhosttyKey_GHOSTTY_KEY_ENTER,
                0,
            )
            .expect("encode key");
        assert!(!seq.is_empty(), "expected some encoded bytes");
    }

    #[test]
    fn vt_update_dirty_lines_updates_buffer() {
        let mut vt = GhosttyVtTerminal::new(10, 2, 1000).expect("create vt");
        let mut lines: Vec<String> = Vec::new();

        vt.write_vt(b"abc");
        vt.update_render_state().expect("update render state");
        vt.update_dirty_plain_lines_and_clear_dirty(&mut lines)
            .expect("update dirty");
        assert_eq!(lines.len(), 2);
        assert!(lines.join("\n").contains("abc"));

        // Update with more content; should still work and modify lines.
        vt.write_vt(b"\r\nz");
        vt.update_render_state().expect("update render state 2");
        vt.update_dirty_plain_lines_and_clear_dirty(&mut lines)
            .expect("update dirty 2");
        assert!(lines.join("\n").contains('z'));
    }

    #[test]
    fn vt_snapshot_styled_rows_produces_runs() {
        let mut vt = GhosttyVtTerminal::new(10, 2, 1000).expect("create vt");
        vt.write_vt(b"\x1b[1;31mR\x1b[0m");
        vt.update_render_state().expect("update render state");
        let rows = vt.snapshot_styled_rows_and_clear_dirty().expect("styled");
        assert_eq!(rows.len(), 2);
        assert!(
            rows[0]
                .runs
                .iter()
                .flat_map(|r| r.cells.iter())
                .any(|c| c.text.contains('R') && !c.continuation)
        );
    }
}
