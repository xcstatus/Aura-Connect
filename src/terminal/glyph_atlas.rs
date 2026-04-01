//! Dynamic R8 atlas for GPU terminal glyphs (Songti.ttc / TTC via `ab_glyph`).
use ab_glyph::{point, Font, FontArc, Glyph, PxScale, ScaleFont};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

mod font_loading;
mod helpers;
use font_loading::{font_has_outline, load_terminal_fonts, LoadedFont};
use helpers::{clamp_slot_offset, fit_scale_for_slot};
#[cfg(test)]
mod tests;

pub(crate) const ATLAS_PX: u32 = 2048;
pub(crate) const GLYPH_SLOT_PX: u32 = 64;
pub(crate) const MAX_ATLAS_PAGES: u32 = 8;
// Tuned to visually match CPU fallback (`egui::FontId::monospace(14.0)`)
// under TERMINAL_ROW_HEIGHT=18 / TERMINAL_CELL_WIDTH=8.5.
const GLYPH_SCALE_IN_SLOT: f32 = 0.98;
const GLYPH_TARGET_FILL_MAX: f32 = 0.97;
// Slot size coefficient relative to font px. Chosen to cover most ascender/descender cases.
const SLOT_PX_COEFF: f32 = 1.45;

fn env_bool(name: &str, default: bool) -> bool {
    match std::env::var(name).ok().as_deref().map(str::trim) {
        None => default,
        Some("") => default,
        Some("1" | "true" | "TRUE" | "True" | "yes" | "YES" | "Yes") => true,
        Some("0" | "false" | "FALSE" | "False" | "no" | "NO" | "No") => false,
        _ => default,
    }
}

fn glyph_tight_rect_enabled() -> bool {
    // A/B toggle for diagnosing geometry mapping instability.
    // 1 = tight (current), 0 = full-slot (stable baseline).
    //
    // Default to full-slot sampling because tight-rect + dst-rect mapping can
    // amplify small metric mismatches into visible "garble/overlap" artifacts.
    // Keep the toggle for targeted diagnosis.
    env_bool("RUST_SSH_GLYPH_TIGHT_RECT", false)
}

fn glyph_fit_scale_enabled() -> bool {
    // 1 = enable per-glyph fit (may reduce clipping but can introduce subtle size drift),
    // 0 = fixed scale (recommended baseline for stability).
    env_bool("RUST_SSH_GLYPH_FIT_SCALE", false)
}

pub(crate) type GlyphAtlasShared = Arc<Mutex<GlyphAtlasState>>;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct AtlasDiagStats {
    pub chars_mapped: usize,
    pub failed_chars_cached: usize,
    pub next_slot: u32,
    pub failed_cache_hits: u64,
    pub zero_pixel_failures: u64,
    pub fallback_successes: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BaselineJitterStats {
    pub off_y_stddev: f32,
    pub bbox_top_drift: f32,
    pub bbox_bottom_drift: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BBoxWidthStats {
    pub w_over_slot_min: f32,
    pub w_over_slot_mean: f32,
    pub w_over_slot_max: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct AtlasPixelSnapshot {
    pub atlas_w: u32,
    pub atlas_h: u32,
    pub slot_px: u32,
    pub pages: u32,
    pub pixels: Vec<u8>,
    pub slot_debug: Vec<AtlasSlotDebug>,
}

#[derive(Clone, Debug)]
pub(crate) struct AtlasSlotDebug {
    pub slot: u16,
    pub ch: u32,
    pub font_label: String,
    pub bounds_w: f32,
    pub bounds_h: f32,
    pub off_x: i32,
    pub off_y: i32,
    pub pixels_written: u32,
    pub fallback_used: bool,
}

pub(crate) struct GlyphAtlasState {
    fonts: Vec<LoadedFont>,
    font_load_attempted: bool,
    /// After load: verified that at least one ASCII glyph outline rasterizes (None = not checked yet).
    probe_ascii_raster_ok: Option<bool>,
    pub(crate) atlas_w: u32,
    pub(crate) atlas_h: u32,
    pub(crate) slot_px: u32,
    pub(crate) atlas_cols: u32,
    pages_used: u32,
    pixels: Vec<u8>,
    char_to_slot: HashMap<u32, u16>,
    failed_chars: HashSet<u32>,
    slot_debug: HashMap<u16, AtlasSlotDebug>,
    slot_to_char: HashMap<u16, u32>,
    slot_last_used_tick: HashMap<u16, u64>,
    /// Per-page tile cursor (0-based within a page).
    next_slot: u32,
    use_tick: u64,
    dirty_slots: Vec<u32>,
    upload_cell_scratch: Vec<u8>,
    cache_hits: u64,
    cache_misses: u64,
    evictions: u64,
    allocations: u64,
    failed_cache_hits: u64,
    zero_pixel_failures: u64,
    fallback_successes: u64,
    font_override_path: Option<String>,
    font_override_face_index: Option<u32>,
    reset_on_pressure: bool,
    /// Target font size in *pixels* (post `pixels_per_point`), quantized for stability.
    target_font_px: Option<f32>,
    /// Target cell width in *pixels* from terminal column policy.
    target_cell_width_px: Option<f32>,
}

impl GlyphAtlasState {
    fn raster_scale_px(&self) -> f32 {
        let slot_scale = (self.slot_px as f32) * 0.88;
        self.target_font_px
            .unwrap_or(slot_scale)
            .max(slot_scale)
            .max(1.0)
    }

    pub(crate) fn new() -> Self {
        let atlas_w = ATLAS_PX;
        let atlas_h = ATLAS_PX;
        let slot_px = GLYPH_SLOT_PX;
        let atlas_cols = atlas_w / slot_px;
        let pages_used = 1;
        let page_px = (atlas_w * atlas_h) as usize;
        Self {
            fonts: Vec::new(),
            font_load_attempted: false,
            probe_ascii_raster_ok: None,
            atlas_w,
            atlas_h,
            slot_px,
            atlas_cols,
            pages_used,
            pixels: vec![0u8; page_px * (MAX_ATLAS_PAGES as usize)],
            char_to_slot: HashMap::new(),
            failed_chars: HashSet::new(),
            slot_debug: HashMap::new(),
            slot_to_char: HashMap::new(),
            slot_last_used_tick: HashMap::new(),
            next_slot: 0,
            use_tick: 0,
            dirty_slots: Vec::new(),
            upload_cell_scratch: Vec::new(),
            cache_hits: 0,
            cache_misses: 0,
            evictions: 0,
            allocations: 0,
            failed_cache_hits: 0,
            zero_pixel_failures: 0,
            fallback_successes: 0,
            font_override_path: None,
            font_override_face_index: None,
            reset_on_pressure: false,
            target_font_px: None,
            target_cell_width_px: None,
        }
    }

    pub(crate) fn set_target_cell_metrics(
        &mut self,
        cell_width_px: f32,
        cell_height_px: f32,
        line_height_ratio: f32,
    ) {
        if !cell_height_px.is_finite() || cell_height_px <= 0.0 {
            return;
        }
        let cell_w = if cell_width_px.is_finite() && cell_width_px > 0.0 {
            Some((cell_width_px * 2.0).round() / 2.0)
        } else {
            None
        };
        let ratio = if line_height_ratio.is_finite() && line_height_ratio > 0.0 {
            line_height_ratio
        } else {
            1.4
        };
        let font_px_raw = (cell_height_px / ratio).max(1.0);
        // Quantize to 0.5px buckets to avoid thrashing on minor DPI jitter.
        let font_px = (font_px_raw * 2.0).round() / 2.0;

        // Slot px: choose a power-of-two divisor of ATLAS_PX for stable packing.
        // Keep a lower bound to preserve stroke detail density after filtering.
        let want_slot = (font_px * SLOT_PX_COEFF).ceil().max(16.0) as u32;
        let mut slot_px = want_slot.next_power_of_two().clamp(64, 128);
        if ATLAS_PX % slot_px != 0 {
            slot_px = GLYPH_SLOT_PX;
        }

        let font_changed = self
            .target_font_px
            .map(|old| (old - font_px).abs() > f32::EPSILON)
            .unwrap_or(true);
        let cell_w_changed = self.target_cell_width_px != cell_w;
        let slot_changed = self.slot_px != slot_px;
        if !font_changed && !slot_changed && !cell_w_changed {
            return;
        }

        self.target_font_px = Some(font_px);
        self.target_cell_width_px = cell_w;
        if slot_changed {
            self.slot_px = slot_px;
            self.atlas_cols = (self.atlas_w / self.slot_px).max(1);
        }

        // New metrics/layout => clear runtime caches + reload scaled fonts.
        self.reset_runtime_state_keep_fonts();
        self.font_load_attempted = false;
        self.probe_ascii_raster_ok = None;
        self.fonts.clear();
    }

    pub(crate) fn set_policy(
        &mut self,
        reset_on_pressure: bool,
        font_path: Option<String>,
        font_face_index: Option<u32>,
    ) {
        self.reset_on_pressure = reset_on_pressure;
        self.set_font_overrides(font_path, font_face_index);
    }

    fn reset_runtime_state_keep_fonts(&mut self) {
        self.char_to_slot.clear();
        self.failed_chars.clear();
        self.slot_debug.clear();
        self.slot_to_char.clear();
        self.slot_last_used_tick.clear();
        self.pages_used = 1;
        self.next_slot = 0;
        self.use_tick = 0;
        self.dirty_slots.clear();
        self.pixels.fill(0);
        self.upload_cell_scratch.clear();
        self.cache_hits = 0;
        self.cache_misses = 0;
        self.evictions = 0;
        self.allocations = 0;
        self.failed_cache_hits = 0;
        self.zero_pixel_failures = 0;
        self.fallback_successes = 0;
    }

    fn page_px_len(&self) -> usize {
        (self.atlas_w * self.atlas_h) as usize
    }

    fn slots_per_page(&self) -> u32 {
        (self.atlas_w / self.slot_px) * (self.atlas_h / self.slot_px)
    }

    fn decode_slot(&self, slot: u16) -> Option<(u32, u32)> {
        let s = slot as u32;
        if s == 0 {
            return None;
        }
        let spp = self.slots_per_page();
        let tile = s.saturating_sub(1);
        let page = tile / spp;
        let in_page = tile % spp;
        Some((page, in_page))
    }

    fn encode_slot(&self, page: u32, in_page: u32) -> Option<u16> {
        let spp = self.slots_per_page();
        let tile = page.saturating_mul(spp).saturating_add(in_page);
        let s = tile.saturating_add(1);
        u16::try_from(s).ok()
    }

    pub(crate) fn set_font_overrides(
        &mut self,
        path: Option<String>,
        face_index: Option<u32>,
    ) {
        let path = path.and_then(|s| {
            let t = s.trim().to_string();
            if t.is_empty() { None } else { Some(t) }
        });
        if self.font_override_path == path && self.font_override_face_index == face_index {
            return;
        }
        self.font_override_path = path;
        self.font_override_face_index = face_index;

        // Reset font load state so next raster attempt reloads with the new overrides.
        self.font_load_attempted = false;
        self.probe_ascii_raster_ok = None;
        self.fonts.clear();

        // Clear caches that depend on glyph availability.
        self.char_to_slot.clear();
        self.failed_chars.clear();
        self.slot_debug.clear();
        self.slot_to_char.clear();
        self.slot_last_used_tick.clear();
        self.pages_used = 1;
        self.next_slot = 1;
        self.use_tick = 0;
        self.dirty_slots.clear();
        self.pixels.fill(0);
        self.upload_cell_scratch.clear();
        self.cache_hits = 0;
        self.cache_misses = 0;
        self.evictions = 0;
        self.allocations = 0;
    }

    /// `true` if GPU terminal text can render (font loaded and probes OK). Used to fall back to CPU galley when atlas is unusable.
    pub(crate) fn gpu_raster_viable(&mut self) -> bool {
        self.ensure_font_loaded();
        if self.fonts.is_empty() {
            return false;
        }
        if let Some(ok) = self.probe_ascii_raster_ok {
            return ok;
        }
        let scale_px = (self.slot_px as f32) * GLYPH_SCALE_IN_SLOT;
        let ok = self
            .fonts
            .iter()
            .any(|f| font_has_outline(&f.font, 'a', scale_px));
        if !ok {
            log::warn!(
                "GPU glyph atlas: font loaded but ASCII probe failed; using CPU terminal text until a working font is available"
            );
        }
        self.probe_ascii_raster_ok = Some(ok);
        ok
    }

    #[cfg(test)]
    #[allow(dead_code)]
    fn max_slots(&self) -> u32 {
        // Total address space of global slots across all pages (including reserved in-page 0).
        self.slots_per_page() * MAX_ATLAS_PAGES
    }

    pub(crate) fn diag_stats(&self) -> AtlasDiagStats {
        AtlasDiagStats {
            chars_mapped: self.char_to_slot.len(),
            failed_chars_cached: self.failed_chars.len(),
            next_slot: self.next_slot,
            failed_cache_hits: self.failed_cache_hits,
            zero_pixel_failures: self.zero_pixel_failures,
            fallback_successes: self.fallback_successes,
        }
    }

    pub(crate) fn baseline_jitter_for_slots(&self, slots: &[u16]) -> Option<BaselineJitterStats> {
        let mut n = 0f32;
        let mut sum = 0f32;
        let mut sum_sq = 0f32;
        let mut top_min = f32::INFINITY;
        let mut top_max = f32::NEG_INFINITY;
        let mut bottom_min = f32::INFINITY;
        let mut bottom_max = f32::NEG_INFINITY;

        for &slot in slots {
            if slot == 0 {
                continue;
            }
            let Some(d) = self.slot_debug.get(&slot) else {
                continue;
            };
            let top = d.off_y as f32;
            let bottom = (d.off_y as f32) + d.bounds_h;
            n += 1.0;
            sum += top;
            sum_sq += top * top;
            top_min = top_min.min(top);
            top_max = top_max.max(top);
            bottom_min = bottom_min.min(bottom);
            bottom_max = bottom_max.max(bottom);
        }

        if n <= 0.0 {
            return None;
        }
        let mean = sum / n;
        let var = (sum_sq / n) - (mean * mean);
        Some(BaselineJitterStats {
            off_y_stddev: var.max(0.0).sqrt(),
            bbox_top_drift: (top_max - top_min).max(0.0),
            bbox_bottom_drift: (bottom_max - bottom_min).max(0.0),
        })
    }

    pub(crate) fn bbox_width_stats_for_slots(&self, slots: &[u16]) -> Option<BBoxWidthStats> {
        let sp = self.slot_px.max(1) as f32;
        let mut n = 0f32;
        let mut sum = 0f32;
        let mut min_v = f32::INFINITY;
        let mut max_v = f32::NEG_INFINITY;
        for &slot in slots {
            if slot == 0 {
                continue;
            }
            let Some(d) = self.slot_debug.get(&slot) else { continue; };
            let v = (d.bounds_w / sp).clamp(0.0, 10.0);
            n += 1.0;
            sum += v;
            min_v = min_v.min(v);
            max_v = max_v.max(v);
        }
        if n <= 0.0 {
            return None;
        }
        Some(BBoxWidthStats {
            w_over_slot_min: min_v,
            w_over_slot_mean: sum / n,
            w_over_slot_max: max_v,
        })
    }

    pub(crate) fn layout_diag(&self) -> (u32, Option<f32>) {
        // (slot_px, cell_width_px)
        (self.slot_px, self.target_cell_width_px)
    }

    pub(crate) fn font_diag_mono_advance(&self) -> Option<f32> {
        self.fonts.first().map(|f| f.mono_advance)
    }

    pub(crate) fn pixel_snapshot(&self) -> AtlasPixelSnapshot {
        let page_px = self.page_px_len();
        let used_len = page_px * (self.pages_used as usize);
        AtlasPixelSnapshot {
            atlas_w: self.atlas_w,
            atlas_h: self.atlas_h,
            slot_px: self.slot_px,
            pages: self.pages_used,
            pixels: self.pixels[..used_len].to_vec(),
            slot_debug: {
                let mut v: Vec<AtlasSlotDebug> = self.slot_debug.values().cloned().collect();
                v.sort_by_key(|e| e.slot);
                v
            },
        }
    }

    pub(crate) fn slot_uv_rect_u16(&self, slot: u16) -> Option<[u16; 4]> {
        let d = self.slot_debug.get(&slot)?;
        if !glyph_tight_rect_enabled() {
            return Some([0, 0, u16::MAX, u16::MAX]);
        }
        let sp = self.slot_px.max(1) as f32;
        let u0 = (d.off_x.max(0) as f32 / sp).clamp(0.0, 1.0);
        let v0 = (d.off_y.max(0) as f32 / sp).clamp(0.0, 1.0);
        let u1 = ((d.off_x as f32 + d.bounds_w) / sp).clamp(0.0, 1.0);
        let v1 = ((d.off_y as f32 + d.bounds_h) / sp).clamp(0.0, 1.0);
        if u1 <= u0 || v1 <= v0 {
            return None;
        }
        let q = |x: f32| -> u16 { (x * 65535.0).round().clamp(0.0, 65535.0) as u16 };
        // Keep monospace horizontal advance semantics by default:
        // X uses full-cell/full-slot mapping; only Y uses glyph-tight mapping.
        Some([q(0.0), q(v0), q(1.0), q(v1)])
    }

    pub(crate) fn slot_dst_rect_u16(&self, slot: u16) -> Option<[u16; 4]> {
        // Destination always maps to the full cell.
        // Keep tight/full policy only on UV sampling to avoid shrinking rendered glyphs
        // into tiny "dot-like" text when slot glyph bounds are much smaller than the slot.
        let _ = self.slot_debug.get(&slot)?;
        Some([0, 0, u16::MAX, u16::MAX])
    }

    pub(crate) fn ensure_font_loaded(&mut self) {
        if self.font_load_attempted {
            return;
        }
        self.font_load_attempted = true;
        let loaded = load_terminal_fonts(
            self.font_override_path.clone(),
            self.font_override_face_index,
        );
        if loaded.is_empty() {
            log::warn!("GPU terminal glyph atlas: font load failed (no usable terminal fonts); CJK GPU text disabled");
            return;
        }
        let scale_px = self.raster_scale_px();
        let loaded_len = loaded.len();
        let mut out = loaded;
        for f in &mut out {
            let mut font_scale = scale_px;
            let mut sf = f.font.as_scaled(PxScale::from(font_scale));
            f.ascent = sf.ascent();
            f.descent = sf.descent();
            f.line_gap = sf.line_gap();
            f.height = sf.height();
            let adv_m = sf.h_advance(f.font.glyph_id('M'));
            let adv_0 = sf.h_advance(f.font.glyph_id('0'));
            f.mono_advance = adv_m.max(adv_0).max(1.0);
            if let Some(cell_w) = self.target_cell_width_px {
                // Keep cell width and glyph advance derived from a single source of truth:
                // terminal column policy cell width. Bound adjustment to stay stable.
                let adj = (cell_w / f.mono_advance).clamp(0.85, 1.15);
                if (adj - 1.0).abs() > 0.01 {
                    font_scale *= adj;
                    sf = f.font.as_scaled(PxScale::from(font_scale));
                    f.ascent = sf.ascent();
                    f.descent = sf.descent();
                    f.line_gap = sf.line_gap();
                    f.height = sf.height();
                    let adv_m2 = sf.h_advance(f.font.glyph_id('M'));
                    let adv_02 = sf.h_advance(f.font.glyph_id('0'));
                    f.mono_advance = adv_m2.max(adv_02).max(1.0);
                }
            }
            f.raster_scale_px = font_scale;
            log::debug!(
                target: "term-diag",
                "GPU font metrics: font={} scale={:.2} ascent={:.2} descent={:.2} line_gap={:.2} line_h={:.2} mono_adv={:.2}",
                f.label,
                f.raster_scale_px,
                f.ascent,
                f.descent,
                f.line_gap,
                f.height,
                f.mono_advance
            );
            if f.height <= 0.0 {
                log::warn!(
                    target: "term-diag",
                    "GPU font metrics abnormal: {} has line_h <= 0",
                    f.label
                );
            }
        }
        log::debug!(
            "GPU terminal glyph atlas: loaded {} font(s) for rasterization",
            loaded_len
        );
        self.fonts = out;
    }

    /// Returns atlas slot index (0 = empty / missing).
    pub(crate) fn slot_for_char(&mut self, ch: u32) -> u16 {
        self.ensure_font_loaded();
        if ch == 0 || ch == u32::from(' ') {
            return 0;
        }
        if let Some(&s) = self.char_to_slot.get(&ch) {
            self.cache_hits = self.cache_hits.saturating_add(1);
            self.touch_slot(s);
            return s;
        }
        self.cache_misses = self.cache_misses.saturating_add(1);
        if self.failed_chars.contains(&ch) {
            self.failed_cache_hits = self.failed_cache_hits.saturating_add(1);
            return 0;
        }
        let Some(font_idx) = self.select_font_for_char(ch) else {
            self.failed_chars.insert(ch);
            return 0;
        };

        let Some(slot) = self.allocate_slot_for_char(ch) else {
            return 0;
        };
        self.allocations = self.allocations.saturating_add(1);
        if self.raster_into_slot(font_idx, ch, slot) {
            self.bind_slot_to_char(slot, ch);
            self.failed_chars.remove(&ch);
            self.dirty_slots.push(slot as u32);
        } else {
            self.failed_chars.insert(ch);
            self.slot_to_char.remove(&slot);
            self.slot_last_used_tick.remove(&slot);
            self.slot_debug.remove(&slot);
            return 0;
        }
        slot
    }

    fn raster_into_slot(&mut self, font_idx: usize, ch: u32, slot: u16) -> bool {
        let Some(f) = self.fonts.get(font_idx) else {
            return false;
        };
        // Copy what we need up-front to avoid holding an immutable borrow of `self.fonts`
        // across mutable operations on `self` (slot clears/draws).
        let font: FontArc = f.font.clone();
        let font_label: String = f.label.clone();
        let font_ascent = f.ascent;
        let font_height = f.height;
        let font_mono_advance = f.mono_advance;
        let font_scale_px = f.raster_scale_px.max(1.0);
        let ch = char::from_u32(ch).unwrap_or('?');

        let outline_for = |scale_px: f32| {
            let scale = PxScale::from(scale_px);
            let g: Glyph = font
                .glyph_id(ch)
                .with_scale_and_position(scale, point(0.0, 0.0));
            let outlined = font.outline_glyph(g)?;
            let bounds = outlined.px_bounds();
            Some((outlined, bounds))
        };

        let mut scale_px = font_scale_px;
        let Some((mut outlined, mut bounds)) = outline_for(scale_px) else {
            return false;
        };

        if glyph_fit_scale_enabled() {
            let fitted_scale = fit_scale_for_slot(
                scale_px,
                bounds.width(),
                bounds.height(),
                self.slot_px as f32,
                GLYPH_TARGET_FILL_MAX,
            );
            if (fitted_scale - scale_px).abs() > f32::EPSILON {
                if let Some((o2, b2)) = outline_for(fitted_scale) {
                    outlined = o2;
                    bounds = b2;
                    scale_px = fitted_scale;
                }
            }
        }

        let w = bounds.width();
        let h = bounds.height();
        if w <= 0.0 || h <= 0.0 {
            return false;
        }

        let (page, in_page) = match self.decode_slot(slot) {
            Some(v) => v,
            None => return false,
        };
        if page >= MAX_ATLAS_PAGES {
            return false;
        }
        let sx = in_page % self.atlas_cols;
        let sy = in_page / self.atlas_cols;
        let ox = (sx * self.slot_px) as i32;
        let oy = (sy * self.slot_px) as i32;

        // Always start from a clean slot to avoid stale glyph pixels accumulating
        // when a slot gets retried/reused.
        self.clear_slot_pixels(page, in_page);

        // `outlined.draw` emits `dx/dy` in the glyph-local bbox [0..w) x [0..h).
        // Baseline alignment: compute a stable baseline position from font metrics
        // (ascender/descender) instead of hard-coded top/center alignment.
        //
        // We map the font's baseline proportion (ascent / height) into the slot height.
        let line_h = font_height.max(1.0);
        let baseline_frac = (font_ascent / line_h).clamp(0.0, 1.0);
        let baseline_y = (self.slot_px as f32) * baseline_frac;
        // Horizontal anchor: fixed monospace advance per font, not per-glyph bbox centering.
        // This keeps all cells on a strict terminal grid and avoids "proportional font" feel.
        let adv = font_mono_advance.max(w).min(self.slot_px as f32);
        let left_pad = ((self.slot_px as f32 - adv) * 0.5).max(0.0);
        let off_x = clamp_slot_offset((left_pad - bounds.min.x).round() as i32, w, self.slot_px);
        // Fixed baseline main path (no per-glyph baseline/center switching).
        let off_y = clamp_slot_offset((baseline_y - bounds.max.y).round() as i32, h, self.slot_px);

        let h_ratio = h / (self.slot_px as f32);
        if h_ratio < 0.30 {
            log::debug!(
                target: "term-diag",
                "GPU glyph too short: ch='{}' U+{:04X} slot={} h={:.2} slot_h={} ratio={:.3} scale_px={:.2}",
                ch,
                ch as u32,
                slot,
                h,
                self.slot_px,
                h_ratio,
                scale_px
            );
        }

        let mut chosen_off_x = off_x;
        let mut chosen_off_y = off_y;
        let mut fallback_used = false;
        let mut pixels_written =
            self.draw_outlined_with_offset(page, &outlined, ox, oy, off_x, off_y);
        if pixels_written == 0 {
            // Fallback for problematic glyph metrics: place bbox at geometric center.
            self.clear_slot_pixels(page, in_page);
            let center_off_x =
                clamp_slot_offset(((self.slot_px as f32 - w) * 0.5).round() as i32, w, self.slot_px);
            let center_off_y =
                clamp_slot_offset(((self.slot_px as f32 - h) * 0.5).round() as i32, h, self.slot_px);
            pixels_written =
                self.draw_outlined_with_offset(page, &outlined, ox, oy, center_off_x, center_off_y);
            if pixels_written > 0 {
                chosen_off_x = center_off_x;
                chosen_off_y = center_off_y;
                fallback_used = true;
                self.fallback_successes = self.fallback_successes.saturating_add(1);
            }
        }

        if pixels_written == 0 {
            self.zero_pixel_failures = self.zero_pixel_failures.saturating_add(1);
            log::debug!("GPU atlas: zero pixels for U+{:04X} (slot {}); clip/layout may be off", ch as u32, slot);
            return false;
        }
        self.slot_debug.insert(
            slot,
            AtlasSlotDebug {
                slot,
                ch: ch as u32,
                font_label,
                bounds_w: w,
                bounds_h: h,
                off_x: chosen_off_x,
                off_y: chosen_off_y,
                pixels_written,
                fallback_used,
            },
        );
        true
    }

    fn select_font_for_char(&self, ch: u32) -> Option<usize> {
        if self.fonts.is_empty() {
            return None;
        }
        let c = char::from_u32(ch).unwrap_or('?');
        let scale_px = (self.slot_px as f32) * GLYPH_SCALE_IN_SLOT;
        if c.is_ascii() {
            if let Some((i, _)) = self.fonts.iter().enumerate().find(|(_, f)| {
                f.label.contains("Menlo") && font_has_outline(&f.font, c, scale_px)
            }) {
                return Some(i);
            }
        }
        self.fonts
            .iter()
            .enumerate()
            .find(|(_, f)| font_has_outline(&f.font, c, scale_px))
            .map(|(i, _)| i)
    }

    fn touch_slot(&mut self, slot: u16) {
        self.use_tick = self.use_tick.wrapping_add(1);
        self.slot_last_used_tick.insert(slot, self.use_tick);
    }

    fn bind_slot_to_char(&mut self, slot: u16, ch: u32) {
        if let Some(old_ch) = self.slot_to_char.insert(slot, ch) {
            self.char_to_slot.remove(&old_ch);
        }
        self.char_to_slot.insert(ch, slot);
        self.touch_slot(slot);
    }

    fn allocate_slot_for_char(&mut self, ch: u32) -> Option<u16> {
        let spp = self.slots_per_page();
        // Allocate within the current page first.
        if self.next_slot < spp {
            let in_page = self.next_slot;
            self.next_slot = self.next_slot.saturating_add(1);
            return self.encode_slot(self.pages_used.saturating_sub(1), in_page);
        }
        // Grow to a new page when possible (reduce LRU churn on long sessions).
        if self.pages_used < MAX_ATLAS_PAGES {
            self.pages_used += 1;
            self.next_slot = 0;
            let in_page = self.next_slot;
            self.next_slot = self.next_slot.saturating_add(1);
            return self.encode_slot(self.pages_used.saturating_sub(1), in_page);
        }
        let victim = self
            .slot_last_used_tick
            .iter()
            .filter(|(slot, _)| **slot != 0)
            .min_by_key(|(_, tick)| *tick)
            .map(|(slot, _)| *slot);
        if let Some(slot) = victim {
            self.evictions = self.evictions.saturating_add(1);
            if let Some(old_ch) = self.slot_to_char.get(&slot).copied() {
                self.char_to_slot.remove(&old_ch);
                self.slot_debug.remove(&slot);
                log::debug!(
                    target: "term-diag",
                    "GPU atlas LRU evict: slot={} old=U+{:04X} new=U+{:04X}",
                    slot,
                    old_ch,
                    ch
                );
            }
            return Some(slot);
        }
        log::warn!("GPU glyph atlas full; dropping char U+{:04X}", ch);
        None
    }

    fn clear_slot_pixels(&mut self, page: u32, in_page: u32) {
        let sx = in_page % self.atlas_cols;
        let sy = in_page / self.atlas_cols;
        let ox = sx * self.slot_px;
        let oy = sy * self.slot_px;
        let page_base = (page as usize) * self.page_px_len();
        for row in 0..self.slot_px {
            let src = page_base + ((oy + row) * self.atlas_w + ox) as usize;
            self.pixels[src..src + self.slot_px as usize].fill(0);
        }
    }

    fn draw_outlined_with_offset(
        &mut self,
        page: u32,
        outlined: &ab_glyph::OutlinedGlyph,
        ox: i32,
        oy: i32,
        off_x: i32,
        off_y: i32,
    ) -> u32 {
        let mut pixels_written = 0u32;
        let page_base = (page as usize) * self.page_px_len();
        outlined.draw(|dx, dy, cov| {
            // `dx/dy` are glyph-local. Convert once to atlas-space pixel coordinates,
            // then clip against the current slot bounds.
            let atlas_x_i = ox + dx as i32 + off_x;
            let atlas_y_i = oy + dy as i32 + off_y;
            if atlas_x_i < ox
                || atlas_y_i < oy
                || atlas_x_i >= ox + self.slot_px as i32
                || atlas_y_i >= oy + self.slot_px as i32
            {
                return;
            }
            if atlas_x_i < 0 || atlas_y_i < 0 {
                return;
            }
            let atlas_x = atlas_x_i as u32;
            let atlas_y = atlas_y_i as u32;
            if atlas_x >= self.atlas_w || atlas_y >= self.atlas_h {
                return;
            }
            let idx = (atlas_y * self.atlas_w + atlas_x) as usize;
            let a = (cov.clamp(0.0, 1.0) * 255.0).round() as u8;
            if a > 0 {
                pixels_written += 1;
            }
            self.pixels[page_base + idx] = a;
        });
        pixels_written
    }

    pub(crate) fn upload_dirty(
        &mut self,
        queue: &egui_wgpu::wgpu::Queue,
        texture: &egui_wgpu::wgpu::Texture,
    ) {
        if self.dirty_slots.is_empty() {
            return;
        }

        let slot_px = self.slot_px as usize;
        // wgpu requires bytes_per_row to be COPY_BYTES_PER_ROW_ALIGNMENT (256) aligned for texture uploads.
        // Our atlas is R8 (1 byte per pixel), so row stride is in bytes.
        let row_stride = ((slot_px + 255) / 256) * 256;
        let needed = row_stride * slot_px;
        if self.upload_cell_scratch.len() != needed {
            self.upload_cell_scratch.resize(needed, 0);
        }

        let spp = self.slots_per_page();
        let page_px_len = self.page_px_len();
        for slot in self.dirty_slots.drain(..) {
            if slot == 0 {
                continue;
            }
            let tile = slot.saturating_sub(1);
            let page = tile / spp;
            let in_page = tile % spp;
            if page >= self.pages_used {
                continue;
            }
            let sx = in_page % self.atlas_cols;
            let sy = in_page / self.atlas_cols;
            let ox = sx * self.slot_px;
            let oy = sy * self.slot_px;
            let page_base = (page as usize) * page_px_len;

            for row in 0..self.slot_px {
                let src = ((oy + row) * self.atlas_w + ox) as usize;
                let dst = (row as usize) * row_stride;
                self.upload_cell_scratch[dst..dst + slot_px]
                    .copy_from_slice(&self.pixels[page_base + src..page_base + src + slot_px]);
                // Clear padding bytes to keep deterministic uploads.
                self.upload_cell_scratch[dst + slot_px..dst + row_stride].fill(0);
            }

            queue.write_texture(
                egui_wgpu::wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: egui_wgpu::wgpu::Origin3d { x: ox, y: oy, z: page },
                    aspect: egui_wgpu::wgpu::TextureAspect::All,
                },
                &self.upload_cell_scratch,
                egui_wgpu::wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row_stride as u32),
                    rows_per_image: Some(self.slot_px),
                },
                egui_wgpu::wgpu::Extent3d {
                    width: self.slot_px,
                    height: self.slot_px,
                    depth_or_array_layers: 1,
                },
            );
        }

        let stats = self.diag_stats();
        if (stats.zero_pixel_failures > 0 || stats.failed_cache_hits > 0)
            && (stats.zero_pixel_failures + stats.failed_cache_hits) % 64 == 0
        {
            log::info!(
                target: "term-diag",
                "GPU atlas stats: mapped={} failed_cached={} next_slot={} failed_cache_hits={} zero_pixel_failures={} fallback_successes={}",
                stats.chars_mapped,
                stats.failed_chars_cached,
                stats.next_slot,
                stats.failed_cache_hits,
                stats.zero_pixel_failures,
                stats.fallback_successes
            );
        }

        // Pressure warning: if we are evicting aggressively, hint that the session is stressing the atlas.
        // Keep this as a low-frequency warning so normal runs stay quiet.
        if self.evictions > 0 && self.allocations > 0 && (self.evictions % 2048 == 0) {
            let rate = (self.evictions as f64) / (self.allocations as f64);
            if rate > 0.35 && self.pages_used >= MAX_ATLAS_PAGES {
                log::warn!(
                    target: "term-diag",
                    "GPU atlas pressure: evictions={} allocations={} eviction_rate={:.2} pages_used={}/{} (consider reset/compact strategy)",
                    self.evictions,
                    self.allocations,
                    rate,
                    self.pages_used,
                    MAX_ATLAS_PAGES
                );
                if self.reset_on_pressure {
                    log::warn!(
                        target: "term-diag",
                        "GPU atlas pressure reset triggered (eviction_rate={:.2})",
                        rate
                    );
                    self.reset_runtime_state_keep_fonts();
                }
            }
        }
    }
}

