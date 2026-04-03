//! Single source of truth for terminal viewport geometry ↔ PTY grid (`TerminalViewportSpec`).
//! `view.rs` must use the same spec for padding/spacing so cols/rows match the scrollable area.

use std::sync::{Mutex, OnceLock};

use iced::Point;
use iced::{Padding, Size};

use crate::settings::TerminalSettings;
use crate::theme::layout::{
    BOTTOM_BAR_HEIGHT, TOP_BAR_HEIGHT, terminal_scroll_hit_exclude_right_px,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct FontMetricKey {
    path: String,
    face_index: u32,
    // Quantize to avoid cache thrash on minor slider jitter.
    q_font_px_x2: i32,
    q_line_h_x100: i32,
}

#[derive(Clone, Copy, Debug, Default)]
struct MeasuredCellMetrics {
    mono_advance_px: f32,
    font_height_px: f32,
}

static FONT_METRICS_CACHE: OnceLock<
    Mutex<std::collections::HashMap<FontMetricKey, MeasuredCellMetrics>>,
> = OnceLock::new();

fn metrics_cache() -> &'static Mutex<std::collections::HashMap<FontMetricKey, MeasuredCellMetrics>>
{
    FONT_METRICS_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

#[inline]
fn qx2(px: f32) -> i32 {
    (px * 2.0).round() as i32
}

#[inline]
fn qx100(x: f32) -> i32 {
    (x * 100.0).round() as i32
}

fn pick_font_file_for_terminal(t: &TerminalSettings) -> (String, u32) {
    // Prefer explicit override used by the GPU glyph atlas (file path + face index).
    if let Some(p) = t
        .gpu_font_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return (p.to_string(), t.gpu_font_face_index.unwrap_or(0));
    }

    // Best-effort mapping for common terminal fonts.
    // When we cannot resolve the exact font family, fall back to a stable system monospace.
    #[cfg(target_os = "macos")]
    {
        let fam = t.font_family.trim();
        let path = match fam {
            "SF Mono" => "/System/Library/Fonts/SFNSMono.ttf",
            "Menlo" => "/System/Library/Fonts/Menlo.ttc",
            "Monaco" => "/System/Library/Fonts/Monaco.ttf",
            _ => "/System/Library/Fonts/Menlo.ttc",
        };
        return (path.to_string(), 0);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let path = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf";
        (path.to_string(), 0)
    }
}

fn measure_cell_metrics(t: &TerminalSettings) -> Option<MeasuredCellMetrics> {
    use ab_glyph::{Font, FontArc, FontVec, PxScale, ScaleFont};

    let font_px = t.font_size.clamp(6.0, 96.0);
    let line_h = t.line_height.clamp(1.0, 3.0);
    let (path, face_index) = pick_font_file_for_terminal(t);
    let key = FontMetricKey {
        path: path.clone(),
        face_index,
        q_font_px_x2: qx2(font_px),
        q_line_h_x100: qx100(line_h),
    };

    if let Ok(g) = metrics_cache().lock() {
        if let Some(v) = g.get(&key) {
            return Some(*v);
        }
    }

    let data = std::fs::read(&path).ok()?;
    let face_count = ttf_parser::fonts_in_collection(&data)
        .map(|n| n.max(1))
        .unwrap_or(1);
    let idx = face_index.min(face_count.saturating_sub(1));
    let fv = FontVec::try_from_vec_and_index(data, idx).ok()?;
    let fa = FontArc::new(fv);

    let scale_px = (key.q_font_px_x2 as f32) / 2.0;
    let sf = fa.as_scaled(PxScale::from(scale_px));
    let adv_m = sf.h_advance(fa.glyph_id('M'));
    let adv_0 = sf.h_advance(fa.glyph_id('0'));
    let mono_advance_px = adv_m.max(adv_0).max(1.0);
    let font_height_px = sf.height().max(1.0);
    let v = MeasuredCellMetrics {
        mono_advance_px,
        font_height_px,
    };

    if let Ok(mut g) = metrics_cache().lock() {
        g.insert(key, v);
    }

    Some(v)
}

/// Layout tokens shared by `view` (visual) and `terminal_scroll_area_px` (PTY sizing).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerminalViewportSpec {
    /// First row: tab strip (`chrome::TOP_BAR_H` / `TOP_BAR_HEIGHT`).
    pub top_bar_h: f32,
    /// Extra height subtracted below window top (e.g. macOS safe area / future titlebar insets). Usually `0`.
    pub macos_window_top_inset: f32,
    /// Breadcrumb container `padding([6, 12])` → vertical `p[0]`.
    pub breadcrumb_padding_top: f32,
    pub breadcrumb_padding_bottom: f32,
    /// Breadcrumb horizontal padding (`p[1]`).
    pub breadcrumb_padding_h: f32,
    /// Minimum content height inside breadcrumb (text + compact buttons row).
    pub breadcrumb_row_min_h: f32,
    /// `column![breadcrumb, main, bottom].spacing(gap)` — **two** gaps eat vertical space.
    pub main_column_gap: f32,
    pub bottom_bar_h: f32,
    /// Terminal panel `padding([8, 12])` → top/bottom `8`, sides `12`.
    pub terminal_panel_pad_top: f32,
    pub terminal_panel_pad_bottom: f32,
    pub terminal_panel_pad_left: f32,
    pub terminal_panel_pad_right: f32,
    /// Border width of the inner terminal frame (`container::bordered_box`).
    pub terminal_inner_border_px: f32,
    /// Inner `column!` spacing inside terminal panel (toolbar row + scroll). When only the scroll
    /// area uses `Fill`, this does **not** reduce scroll height — keep for view parity only.
    pub terminal_panel_inner_column_spacing: f32,
    pub term_font_px: f32,
    pub term_cell_w_per_font: f32,
    pub term_cell_h_per_font: f32,
}

impl TerminalViewportSpec {
    /// Keep in lockstep with `view.rs` terminal tab layout.
    pub const DEFAULT: Self = Self {
        top_bar_h: TOP_BAR_HEIGHT,
        macos_window_top_inset: 0.0,
        breadcrumb_padding_top: 6.0,
        breadcrumb_padding_bottom: 6.0,
        breadcrumb_padding_h: 12.0,
        breadcrumb_row_min_h: 28.0,
        main_column_gap: 4.0,
        bottom_bar_h: BOTTOM_BAR_HEIGHT,
        terminal_panel_pad_top: 8.0,
        terminal_panel_pad_bottom: 8.0,
        terminal_panel_pad_left: 12.0,
        terminal_panel_pad_right: 12.0,
        terminal_inner_border_px: 1.0,
        terminal_panel_inner_column_spacing: 10.0,
        term_font_px: 14.0,
        term_cell_w_per_font: 0.62,
        term_cell_h_per_font: 1.28,
    };

    /// Simulated macOS client-area overlap (unit tests; wire from real insets when needed).
    #[allow(dead_code)] // only referenced from `#[cfg(test)]` in this module; keep for future hook
    pub const fn with_macos_top_inset(self, inset: f32) -> Self {
        Self {
            macos_window_top_inset: inset,
            ..self
        }
    }

    #[inline]
    pub const fn breadcrumb_block_h(self) -> f32 {
        self.breadcrumb_padding_top + self.breadcrumb_padding_bottom + self.breadcrumb_row_min_h
    }

    #[inline]
    pub const fn breadcrumb_padding(self) -> Padding {
        Padding {
            top: self.breadcrumb_padding_top,
            right: self.breadcrumb_padding_h,
            bottom: self.breadcrumb_padding_bottom,
            left: self.breadcrumb_padding_h,
        }
    }

    #[inline]
    pub const fn terminal_panel_padding(self) -> Padding {
        Padding {
            top: self.terminal_panel_pad_top,
            right: self.terminal_panel_pad_right,
            bottom: self.terminal_panel_pad_bottom,
            left: self.terminal_panel_pad_left,
        }
    }

    #[inline]
    pub const fn main_column_spacing(self) -> f32 {
        self.main_column_gap
    }

    #[inline]
    pub const fn terminal_panel_inner_spacing(self) -> f32 {
        self.terminal_panel_inner_column_spacing
    }

    pub fn term_cell_w(self) -> f32 {
        self.term_font_px * self.term_cell_w_per_font
    }

    pub fn term_cell_h(self) -> f32 {
        self.term_font_px * self.term_cell_h_per_font
    }
}

/// PTY grid + Iced terminal metrics from persisted [`TerminalSettings`].
///
/// Chrome insets match [`TerminalViewportSpec::DEFAULT`]; font size / line height come from settings
/// when [`TerminalSettings::apply_terminal_metrics`] is true.
pub fn terminal_viewport_spec_for_settings(t: &TerminalSettings) -> TerminalViewportSpec {
    let base = TerminalViewportSpec::DEFAULT;
    if !t.apply_terminal_metrics {
        return base;
    }
    let font_px = t.font_size.clamp(6.0, 96.0);
    let line_h = t.line_height.clamp(1.0, 3.0);
    let measured = measure_cell_metrics(t);
    let cell_w_per_font = measured
        .map(|m| (m.mono_advance_px / font_px).clamp(0.30, 1.50))
        .unwrap_or(base.term_cell_w_per_font);
    // Font height is for line_height=1.0; apply user line height as a multiplier.
    let cell_h_per_font = measured
        .map(|m| ((m.font_height_px * line_h) / font_px).clamp(0.80, 3.50))
        .unwrap_or(base.term_cell_h_per_font);

    TerminalViewportSpec {
        term_font_px: font_px,
        term_cell_w_per_font: cell_w_per_font,
        term_cell_h_per_font: cell_h_per_font,
        ..base
    }
}

/// Scrollable terminal area in logical pixels (width × height) available for monospace grid.
pub fn terminal_scroll_area_px(window: Size, spec: &TerminalViewportSpec) -> (f32, f32) {
    let (_, _, w, h) = terminal_scroll_area_rect(window, spec);
    (w, h)
}

/// Scrollable terminal area rectangle in window coordinates: `(x, y, w, h)`.
pub fn terminal_scroll_area_rect(
    window: Size,
    spec: &TerminalViewportSpec,
) -> (f32, f32, f32, f32) {
    let win_w = window.width.max(1.0);
    let win_h = window.height.max(1.0);

    // X: inside terminal panel horizontal padding.
    let x = spec.terminal_panel_pad_left + spec.terminal_inner_border_px;
    let w = (win_w
        - spec.terminal_panel_pad_left
        - spec.terminal_panel_pad_right
        - spec.terminal_inner_border_px * 2.0)
        .max(1.0);

    // Y: top chrome + breadcrumb + gap + inside terminal panel vertical padding.
    let y = (spec.top_bar_h
        + spec.macos_window_top_inset
        + spec.breadcrumb_block_h()
        + spec.main_column_gap
        + spec.terminal_panel_pad_top
        + spec.terminal_inner_border_px)
        .max(0.0);

    // Height: derived from the same SSOT chain as `terminal_scroll_area_px`.
    let below_top = (win_h - spec.top_bar_h - spec.macos_window_top_inset).max(1.0);
    let body_h =
        (below_top - spec.breadcrumb_block_h() - spec.bottom_bar_h - spec.main_column_gap * 2.0)
            .max(1.0);
    let h = (body_h
        - spec.terminal_panel_pad_top
        - spec.terminal_panel_pad_bottom
        - spec.terminal_inner_border_px * 2.0)
        .max(1.0);
    (x, y, w, h)
}

/// Mappable scroll width (excluding right hit-test gutter) and **uniform** column band width.
/// Must match [`window_point_to_grid_with_dims`] and terminal row layout in `terminal_rich`.
pub fn terminal_scroll_cell_geometry(
    window: Size,
    spec: &TerminalViewportSpec,
    cols: u16,
) -> (f32, f32) {
    let (_, _, w, _) = terminal_scroll_area_rect(window, spec);
    let cols = cols.max(1);
    let metric_cw = spec.term_cell_w().max(1.0);
    let gutter = terminal_scroll_hit_exclude_right_px();
    let exclude_r = if w > gutter + 8.0 && metric_cw > 2.0 {
        gutter.min((metric_cw - 1.0).max(0.0))
    } else {
        0.0
    };
    let w_map = (w - exclude_r).max(1.0);
    let cell_w = w_map / cols as f32;
    (w_map, cell_w)
}

/// Map a window cursor position into terminal grid coordinates `(col, row)` if inside the
/// scrollable terminal area.
/// Like [`window_point_to_grid`], but maps using the **current terminal grid dims** and the
/// scroll area's pixel width/height. This is more stable than estimating cell width from font.
pub fn window_point_to_grid_with_dims(
    window: Size,
    spec: &TerminalViewportSpec,
    p: Point,
    cols: u16,
    rows: u16,
) -> Option<(u16, u16)> {
    let (x0, y0, w, h) = terminal_scroll_area_rect(window, spec);
    let x1 = x0 + w;
    let y1 = y0 + h;
    if p.x < x0 || p.x >= x1 || p.y < y0 || p.y >= y1 {
        return None;
    }

    let cols = cols.max(1);
    let rows = rows.max(1);

    let local_x = p.x - x0;
    let local_y = p.y - y0;
    if local_x < 0.0 || local_y < 0.0 || w <= 0.0 || h <= 0.0 {
        return None;
    }

    // Row: same `ch` as PTY + fixed terminal row height in `terminal_rich`.
    let ch = spec.term_cell_h().max(1.0);
    let content_h = rows as f32 * ch;
    if local_y >= content_h {
        return None;
    }

    let (w_map, cell_w) = terminal_scroll_cell_geometry(window, spec, cols);
    if local_x >= w_map {
        return None;
    }

    // Column: partition the *mappable* width into exactly `cols` bands. `spec.term_cell_w()` comes
    // from font metrics and can disagree with Iced/cryoglyph glyph advances, so `local_x/metric_cw`
    // drifts (e.g. clicking “dream” letter-by-letter skips columns). `w_map/cols` matches the PTY
    // resize rule `cols = floor(tw / cw_metric)` while aligning bands to the live viewport width.
    let col =
        ((local_x / cell_w).floor() as i32).clamp(0, i32::from(cols.saturating_sub(1))) as u16;
    let row = ((local_y / ch).floor() as i32).clamp(0, i32::from(rows.saturating_sub(1))) as u16;
    Some((col, row))
}

/// `(cols, rows)` for PTY / local VT using default cell metrics (no settings).
#[allow(dead_code)] // convenience for tools/tests; Iced app uses `grid_from_window_size_with_spec` + settings.
pub fn grid_from_window_size(window: Size) -> (u16, u16) {
    grid_from_window_size_with_spec(window, &TerminalViewportSpec::DEFAULT)
}

pub fn grid_from_window_size_with_spec(window: Size, spec: &TerminalViewportSpec) -> (u16, u16) {
    let (w, h) = terminal_scroll_area_px(window, spec);
    let cw = spec.term_cell_w();
    let ch = spec.term_cell_h();
    let cols = (w / cw).floor().max(1.0) as u16;
    let rows = (h / ch).floor().max(1.0) as u16;
    (cols, rows)
}

// --- diagnostic logging (on change only, low frequency) ---

#[derive(Clone, Copy, PartialEq, Eq)]
struct ViewportDiagKey {
    q_ww: i32,
    q_wh: i32,
    q_tw: i32,
    q_th: i32,
    cols: u16,
    rows: u16,
    pty_sent: bool,
}

#[inline]
fn qpx(x: f32) -> i32 {
    (x * 100.0).round() as i32
}

static LAST_VIEWPORT_DIAG: OnceLock<Mutex<Option<ViewportDiagKey>>> = OnceLock::new();

fn last_diag_mutex() -> &'static Mutex<Option<ViewportDiagKey>> {
    LAST_VIEWPORT_DIAG.get_or_init(|| Mutex::new(None))
}

/// Log when window / terminal px / grid / PTY push changes (for field debugging).
pub fn log_viewport_geometry_if_changed(
    window: Size,
    spec: &TerminalViewportSpec,
    cols: u16,
    rows: u16,
    pty_resize_sent: bool,
) {
    if !crate::term_diag::enabled("VIEWPORT") {
        return;
    }
    let (tw, th) = terminal_scroll_area_px(window, spec);
    let key = ViewportDiagKey {
        q_ww: qpx(window.width),
        q_wh: qpx(window.height),
        q_tw: qpx(tw),
        q_th: qpx(th),
        cols,
        rows,
        pty_sent: pty_resize_sent,
    };
    let mut g = match last_diag_mutex().lock() {
        Ok(m) => m,
        Err(_) => return,
    };
    if g.as_ref() == Some(&key) {
        return;
    }
    *g = Some(key);
    log::info!(
        target: "term_viewport",
        "terminal viewport geometry updated window_px=({}, {}) terminal_scroll_px=({}, {}) cols_rows=({}, {}) pty_resize_sent={}",
        window.width,
        window.height,
        tw,
        th,
        cols,
        rows,
        pty_resize_sent,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_grid_positive(window: Size, spec: &TerminalViewportSpec) {
        let (c, r) = grid_from_window_size_with_spec(window, spec);
        assert!(c >= 1 && r >= 1, "cols={c} rows={r}");
    }

    #[test]
    fn point_to_grid_with_dims_maps_corners() {
        let window = Size::new(1280.0, 800.0);
        let spec = TerminalViewportSpec::DEFAULT;
        let (x0, y0, w, h) = terminal_scroll_area_rect(window, &spec);
        let cols: u16 = 120;
        let rows: u16 = 36;

        let tl = Point::new(x0 + 1.0, y0 + 1.0);
        assert_eq!(
            window_point_to_grid_with_dims(window, &spec, tl, cols, rows),
            Some((0, 0))
        );

        let ch = spec.term_cell_h();
        let (w_map, cell_w) = terminal_scroll_cell_geometry(window, &spec, cols);
        let exclude_r = w - w_map;
        let br_x = (x0 + (f32::from(cols) - 1.0) * cell_w + 0.5 * cell_w).min(x0 + w_map - 0.01);
        let br_y = (y0 + (f32::from(rows) - 1.0) * ch + 0.5 * ch).min(y0 + h - 0.01);
        let br = Point::new(br_x, br_y);
        assert_eq!(
            window_point_to_grid_with_dims(window, &spec, br, cols, rows),
            Some((cols - 1, rows - 1))
        );

        if exclude_r > 1.0 {
            let in_scrollbar_strip = Point::new(x0 + w - exclude_r * 0.25, y0 + h * 0.5);
            assert!(
                window_point_to_grid_with_dims(window, &spec, in_scrollbar_strip, cols, rows)
                    .is_none(),
                "right overlay strip should not map to grid"
            );
        }
    }

    #[test]
    fn point_to_grid_cell_centers_roundtrip() {
        let window = Size::new(1280.0, 800.0);
        let spec = TerminalViewportSpec::DEFAULT;
        let (x0, y0, _, _) = terminal_scroll_area_rect(window, &spec);
        let cols: u16 = 120;
        let rows: u16 = 36;
        let ch = spec.term_cell_h();
        let (_, cell_w) = terminal_scroll_cell_geometry(window, &spec, cols);

        for &(c, r) in &[(0u16, 0u16), (1, 1), (30, 10), (79, 20), (119, 35)] {
            let p = Point::new(
                x0 + (c as f32) * cell_w + cell_w * 0.5,
                y0 + (r as f32) * ch + ch * 0.5,
            );
            assert_eq!(
                window_point_to_grid_with_dims(window, &spec, p, cols, rows),
                Some((c, r)),
                "center should map back to same cell: ({c},{r})"
            );
        }
    }

    #[test]
    fn point_to_grid_bottom_slack_is_not_mapped() {
        let window = Size::new(1279.0, 803.0);
        let spec = TerminalViewportSpec::DEFAULT;
        let (x0, y0, w, h) = terminal_scroll_area_rect(window, &spec);
        let cols: u16 = 120;
        let rows: u16 = 36;
        let ch = spec.term_cell_h();
        let content_h = rows as f32 * ch;
        if h <= content_h + 0.5 {
            return;
        }
        let p = Point::new(x0 + w * 0.5, y0 + content_h + 1.0);
        assert!(
            window_point_to_grid_with_dims(window, &spec, p, cols, rows).is_none(),
            "bottom slack area should not hit any row"
        );
    }

    #[test]
    fn default_window_reasonable_grid() {
        let w = Size::new(1280.0, 800.0);
        let spec = TerminalViewportSpec::DEFAULT;
        let (cols, rows) = grid_from_window_size_with_spec(w, &spec);
        let (tw, th) = terminal_scroll_area_px(w, &spec);
        assert!(cols >= 80, "cols={cols} tw={tw}");
        assert!(rows >= 30, "rows={rows} th={th}");
    }

    #[test]
    fn tiny_window_still_one_cell() {
        let w = Size::new(50.0, 50.0);
        assert_grid_positive(w, &TerminalViewportSpec::DEFAULT);
    }

    #[test]
    fn fractional_scale_boundary() {
        let w = Size::new(0.1, 0.1);
        assert_grid_positive(w, &TerminalViewportSpec::DEFAULT);
    }

    #[test]
    fn macos_top_inset_reduces_rows() {
        let window = Size::new(900.0, 600.0);
        let base = TerminalViewportSpec::DEFAULT;
        let inset = base.with_macos_top_inset(28.0);
        let (_, r0) = grid_from_window_size_with_spec(window, &base);
        let (_, r1) = grid_from_window_size_with_spec(window, &inset);
        assert!(
            r1 < r0,
            "expected inset to shrink rows: base={r0} inset={r1}"
        );
    }

    #[test]
    fn width_change_affects_cols_not_rows() {
        let spec = TerminalViewportSpec::DEFAULT;
        let short = Size::new(640.0, 720.0);
        let wide = Size::new(1280.0, 720.0);
        let (c0, r0) = grid_from_window_size_with_spec(short, &spec);
        let (c1, r1) = grid_from_window_size_with_spec(wide, &spec);
        assert_eq!(r0, r1, "same height should preserve rows");
        assert!(c1 > c0, "wider window should increase cols: {c0} vs {c1}");
    }

    #[test]
    fn height_change_affects_rows_not_cols() {
        let spec = TerminalViewportSpec::DEFAULT;
        let squat = Size::new(900.0, 500.0);
        let tall = Size::new(900.0, 900.0);
        let (c0, r0) = grid_from_window_size_with_spec(squat, &spec);
        let (c1, r1) = grid_from_window_size_with_spec(tall, &spec);
        assert_eq!(c0, c1, "same width should preserve cols");
        assert!(r1 > r0, "taller window should increase rows: {r0} vs {r1}");
    }

    #[test]
    fn padding_change_affects_grid() {
        let w = Size::new(800.0, 600.0);
        let base = TerminalViewportSpec::DEFAULT;
        let loose = TerminalViewportSpec {
            terminal_panel_pad_top: base.terminal_panel_pad_top + 40.0,
            terminal_panel_pad_bottom: base.terminal_panel_pad_bottom + 40.0,
            ..base
        };
        let (c0, r0) = grid_from_window_size_with_spec(w, &base);
        let (c1, r1) = grid_from_window_size_with_spec(w, &loose);
        assert_eq!(c0, c1, "vertical-only padding should not change cols");
        assert!(
            r1 < r0,
            "extra vertical padding must reduce rows: {r0} vs {r1}"
        );
    }

    #[test]
    fn larger_terminal_font_reduces_grid_dims() {
        use crate::settings::TerminalSettings;
        let w = Size::new(1000.0, 700.0);
        let mut t = TerminalSettings::default();
        t.apply_terminal_metrics = true;
        t.font_size = 12.0;
        let spec_s = super::terminal_viewport_spec_for_settings(&t);
        t.font_size = 28.0;
        let spec_l = super::terminal_viewport_spec_for_settings(&t);
        let (c_s, r_s) = grid_from_window_size_with_spec(w, &spec_s);
        let (c_l, r_l) = grid_from_window_size_with_spec(w, &spec_l);
        assert!(c_l < c_s, "larger font should reduce cols: {c_s} vs {c_l}");
        assert!(r_l < r_s, "larger font should reduce rows: {r_s} vs {r_l}");
    }

    #[test]
    fn apply_terminal_metrics_off_uses_default_cell_metrics() {
        use crate::settings::TerminalSettings;
        let mut t = TerminalSettings::default();
        t.apply_terminal_metrics = false;
        t.font_size = 32.0;
        t.line_height = 2.5;
        let spec = super::terminal_viewport_spec_for_settings(&t);
        assert_eq!(
            spec.term_font_px,
            TerminalViewportSpec::DEFAULT.term_font_px
        );
        assert_eq!(
            spec.term_cell_h_per_font,
            TerminalViewportSpec::DEFAULT.term_cell_h_per_font
        );
    }
}
