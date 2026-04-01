use anyhow::Result;
use egui_wgpu::wgpu;

use crate::backend::ghostty_vt::VtStyledRow;
use crate::terminal::glyph_atlas::{
    AtlasDiagStats, AtlasPixelSnapshot, BaselineJitterStats, BBoxWidthStats, GlyphAtlasShared,
    GlyphAtlasState, ATLAS_PX, GLYPH_SLOT_PX,
};

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod callback;
mod bg_pipeline;
mod grid;
mod glyph_pipeline;
#[cfg(test)]
mod tests;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BgVertex {
    // Unit quad positions (0..1)
    pos: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BgInstance {
    cell_xy: [u16; 2],
    _pad0: [u16; 2],
    bg_rgba: [u8; 4],
    _pad1: [u8; 12],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BgUniform {
    viewport_px: [f32; 2],
    cell_size_px: [f32; 2],
    origin_px: [f32; 2],
    _pad: [f32; 2],
}

struct BgResources {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    index_count: u32,
    instance_buf: wgpu::Buffer,
    instance_cap: usize,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GlyphInstance {
    cell_xy: [u16; 2],
    glyph_slot: u16, // atlas slot; 0 = skip / empty
    atlas_page: u16,
    /// GPU: bit0 bold (synthetic stroke), bit1 underline bar in fragment.
    flags: u16,
    _pad0: u16,
    fg_rgba: [u8; 4],
    /// UV crop rect in slot-local normalized coordinates, quantized to [0..65535].
    /// Layout: [u0, v0, u1, v1].
    uv_rect: [u16; 4],
    /// Destination rect in cell-local normalized coordinates.
    /// Layout: [x0, y0, x1, y1].
    dst_rect: [u16; 4],
    // Keep stride aligned for wgpu vertex buffer requirements.
    _pad: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GlyphUniform {
    viewport_px: [f32; 2],
    cell_size_px: [f32; 2],
    origin_px: [f32; 2],
    atlas_w: f32,
    atlas_h: f32,
    slot_px: f32,
    atlas_cols: f32,
    slots_per_page: f32,
    glyph_uv_crop: f32,
    glyph_alpha_boost: f32,
    _pad0: f32,
}

struct GlyphResources {
    pipeline: wgpu::RenderPipeline,
    bind_group0: wgpu::BindGroup, // uniforms
    uniform_buf: wgpu::Buffer,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    index_count: u32,
    instance_buf: wgpu::Buffer,
    instance_cap: usize,
    atlas_tex: wgpu::Texture,
    bind_group1: wgpu::BindGroup, // atlas
}

pub(crate) struct SharedFrameData {
    cols: u16,
    rows: u16,
    viewport_px: (u32, u32),
    cell_size_px: (f32, f32),
    origin_px: (f32, f32),
    bg_instances: Vec<BgInstance>,
    glyph_instances: Vec<GlyphInstance>,
    glyph_build_diag: GlyphBuildDiag,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct GlyphBuildDiag {
    pub total_cells: usize,
    pub non_space_cells: usize,
    pub zero_slot_cells: usize,
    pub underline_space_quads: usize,
    pub glyph_instances: usize,
    pub off_y_stddev: f32,
    pub bbox_top_drift: f32,
    pub bbox_bottom_drift: f32,
    pub font_mono_advance: f32,
    pub cell_width_px: f32,
    pub bbox_w_over_slot_min: f32,
    pub bbox_w_over_slot_mean: f32,
    pub bbox_w_over_slot_max: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct GpuGlyphDumpInstance {
    pub cell_xy: [u16; 2],
    pub glyph_slot: u16,
    pub atlas_page: u16,
    pub flags: u16,
    pub fg_rgba: [u8; 4],
    pub uv_rect: [u16; 4],
    pub dst_rect: [u16; 4],
}

#[derive(Clone, Debug)]
pub(crate) struct GpuBgDumpInstance {
    pub cell_xy: [u16; 2],
    pub bg_rgba: [u8; 4],
}

#[derive(Clone, Debug)]
pub(crate) struct GpuFrameDumpSnapshot {
    pub viewport_px: (u32, u32),
    pub cell_size_px: (f32, f32),
    pub origin_px: (f32, f32),
    pub bg_instances: Vec<GpuBgDumpInstance>,
    pub glyph_instances: Vec<GpuGlyphDumpInstance>,
}

pub struct TerminalOffscreenClearCallback {
    texture: std::sync::Arc<wgpu::Texture>,
    view: std::sync::Arc<wgpu::TextureView>,
    shared: Arc<Mutex<SharedFrameData>>,
    atlas: GlyphAtlasShared,
}

/// GPU renderer skeleton for render-to-texture terminal.
///
/// Step 1 (this file): own the offscreen texture + egui TextureId registration.
/// Next steps will add glyph atlas, pipelines, and instance buffers.
pub struct TerminalGpuRenderer {
    pub texture_id: Option<eframe::egui::TextureId>,
    size_px: (u32, u32),
    pending_size_px: Option<(u32, u32)>,
    last_resize_request: Option<Instant>,
    grid: CellGrid,
    offscreen: Option<OffscreenTarget>,
    shared: Arc<Mutex<SharedFrameData>>,
    glyph_atlas: GlyphAtlasShared,
    bg_scratch: Vec<BgInstance>,
    glyph_scratch: Vec<GlyphInstance>,
}

impl Default for TerminalGpuRenderer {
    fn default() -> Self {
        Self {
            texture_id: None,
            size_px: (0, 0),
            pending_size_px: None,
            last_resize_request: None,
            grid: CellGrid::default(),
            offscreen: None,
            shared: Arc::new(Mutex::new(SharedFrameData {
                cols: 0,
                rows: 0,
                viewport_px: (0, 0),
                cell_size_px: (0.0, 0.0),
                origin_px: (0.0, 0.0),
                bg_instances: Vec::new(),
                glyph_instances: Vec::new(),
                glyph_build_diag: GlyphBuildDiag::default(),
            })),
            glyph_atlas: Arc::new(Mutex::new(GlyphAtlasState::new())),
            bg_scratch: Vec::new(),
            glyph_scratch: Vec::new(),
        }
    }
}

impl TerminalGpuRenderer {
    const DEFAULT_BG: [u8; 4] = [10, 10, 15, 255];

    fn full_uv_rect() -> [u16; 4] {
        [0, 0, u16::MAX, u16::MAX]
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_settings(
        &mut self,
        gpu_font_path: Option<String>,
        gpu_font_face_index: Option<u32>,
        atlas_reset_on_pressure: bool,
    ) {
        self.glyph_atlas
            .lock()
            .unwrap()
            .set_policy(atlas_reset_on_pressure, gpu_font_path, gpu_font_face_index);
    }

    pub(crate) fn glyph_atlas_shared(&self) -> GlyphAtlasShared {
        self.glyph_atlas.clone()
    }

    /// When false, `VtTerminalWidget` should use CPU galley text so SSH/login lines remain visible.
    pub(crate) fn gpu_glyph_raster_viable(&self) -> bool {
        self.glyph_atlas.lock().unwrap().gpu_raster_viable()
    }

    /// When false, `VtTerminalWidget` should disable GPU terminal text and fall back to CPU,
    /// because the current grid contains grapheme clusters that cannot be rasterized as a single glyph.
    pub(crate) fn gpu_text_safe(&self) -> bool {
        !self.grid.seen_complex_grapheme
    }

    pub fn ensure_texture(
        &mut self,
        frame: &mut eframe::Frame,
        width_px: u32,
        height_px: u32,
    ) -> Result<()> {
        let Some(rs) = frame.wgpu_render_state() else {
            return Ok(());
        };
        let width_px = width_px.max(1);
        let height_px = height_px.max(1);
        if self.texture_id.is_some() && self.size_px == (width_px, height_px) {
            return Ok(());
        }

        // Debounce rapid resize bursts (window dragging). We keep the last requested size and only
        // rebuild the offscreen texture after it stabilizes briefly.
        const RESIZE_DEBOUNCE: Duration = Duration::from_millis(80);
        let now = Instant::now();
        self.pending_size_px = Some((width_px, height_px));
        match self.last_resize_request {
            None => {
                self.last_resize_request = Some(now);
                // Fall through to create immediately on first resize.
            }
            Some(last) => {
                self.last_resize_request = Some(now);
                if now.duration_since(last) < RESIZE_DEBOUNCE {
                    return Ok(());
                }
            }
        }
        let (width_px, height_px) = self.pending_size_px.unwrap_or((width_px, height_px));
        if self.texture_id.is_some() && self.size_px == (width_px, height_px) {
            return Ok(());
        }

        // Create an offscreen texture view (rgba8) and register it with egui.
        // NOTE: we intentionally keep this minimal for now; later we will keep the
        // actual wgpu::Texture around and render into it via an egui_wgpu callback.
        let device = &rs.device;
        let texture = std::sync::Arc::new(device.create_texture(&wgpu::TextureDescriptor {
            label: Some("terminal_offscreen_rgba8"),
            size: wgpu::Extent3d {
                width: width_px,
                height: height_px,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        }));
        let view = std::sync::Arc::new(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        self.offscreen = Some(OffscreenTarget {
            texture: texture.clone(),
            view: view.clone(),
        });

        fn offscreen_filter_mode() -> wgpu::FilterMode {
            match std::env::var("RUST_SSH_OFFSCREEN_FILTER")
                .ok()
                .as_deref()
                .map(str::trim)
            {
                Some("nearest" | "NEAREST" | "Nearest") => wgpu::FilterMode::Nearest,
                Some("linear" | "LINEAR" | "Linear") => wgpu::FilterMode::Linear,
                _ => wgpu::FilterMode::Linear,
            }
        }

        let filter = offscreen_filter_mode();
        let mut renderer = rs.renderer.write();
        if let Some(id) = self.texture_id {
            renderer.update_egui_texture_from_wgpu_texture(device, &view, filter, id);
        } else {
            let tid = renderer.register_native_texture(device, &view, filter);
            self.texture_id = Some(tid);
        }
        self.size_px = (width_px, height_px);
        Ok(())
    }

    pub(crate) fn offscreen_size_px(&self) -> (u32, u32) {
        self.size_px
    }

    /// Update CPU-side cell grid from styled rows.
    ///
    /// This is the bridge between libghostty's row/runs representation and our future GPU instance buffers.
    /// `updated_rows` should contain row indices that changed; if empty, we treat it as "update all rows".
    pub fn update_cells_from_rows(
        &mut self,
        cols: u16,
        rows: u16,
        styled_rows: &[VtStyledRow],
        updated_rows: &[usize],
    ) {
        let cols_usize = cols.max(1) as usize;
        let rows_usize = rows.max(1) as usize;
        self.grid.ensure_size(cols_usize, rows_usize);

        let update_all = updated_rows.is_empty();
        if update_all {
            for y in 0..rows_usize.min(styled_rows.len()) {
                self.grid.update_row(cols_usize, y, &styled_rows[y]);
            }
        } else {
            for &y in updated_rows {
                if y < rows_usize && y < styled_rows.len() {
                    self.grid.update_row(cols_usize, y, &styled_rows[y]);
                }
            }
        }
    }

    pub fn offscreen_view(&self) -> Option<std::sync::Arc<wgpu::TextureView>> {
        self.offscreen.as_ref().map(|o| o.view.clone())
    }

    pub fn offscreen_texture(&self) -> Option<std::sync::Arc<wgpu::Texture>> {
        self.offscreen.as_ref().map(|o| o.texture.clone())
    }

    pub(crate) fn shared_frame_data(&self) -> Arc<Mutex<SharedFrameData>> {
        self.shared.clone()
    }

    pub(crate) fn instance_counts(&self) -> (usize, usize) {
        let shared = self.shared.lock().unwrap();
        (shared.bg_instances.len(), shared.glyph_instances.len())
    }

    pub(crate) fn glyph_build_diag(&self) -> GlyphBuildDiag {
        let shared = self.shared.lock().unwrap();
        shared.glyph_build_diag
    }

    pub(crate) fn atlas_diag_stats(&self) -> AtlasDiagStats {
        self.glyph_atlas.lock().unwrap().diag_stats()
    }

    pub(crate) fn dump_snapshot(&self) -> (GpuFrameDumpSnapshot, AtlasPixelSnapshot) {
        let shared = self.shared.lock().unwrap();
        let frame = GpuFrameDumpSnapshot {
            viewport_px: shared.viewport_px,
            cell_size_px: shared.cell_size_px,
            origin_px: shared.origin_px,
            bg_instances: shared
                .bg_instances
                .iter()
                .map(|b| GpuBgDumpInstance {
                    cell_xy: b.cell_xy,
                    bg_rgba: b.bg_rgba,
                })
                .collect(),
            glyph_instances: shared
                .glyph_instances
                .iter()
                .map(|g| GpuGlyphDumpInstance {
                    cell_xy: g.cell_xy,
                    glyph_slot: g.glyph_slot,
                    atlas_page: g.atlas_page,
                    flags: g.flags,
                    fg_rgba: g.fg_rgba,
                    uv_rect: g.uv_rect,
                    dst_rect: g.dst_rect,
                })
                .collect(),
        };
        drop(shared);
        let atlas = self.glyph_atlas.lock().unwrap().pixel_snapshot();
        (frame, atlas)
    }

    pub fn build_bg_instances(&mut self, cols: u16, rows: u16) {
        let cols = cols.max(1) as usize;
        let rows = rows.max(1) as usize;
        // Sparse background instances: only emit quads for cells that explicitly
        // set a non-default background. The default background is covered by
        // the offscreen clear pass.
        self.bg_scratch.clear();
        self.bg_scratch.reserve(cols * rows / 8);
        for y in 0..rows {
            for x in 0..cols {
                let idx = y * cols + x;
                let Some(c) = self.grid.cells.get(idx) else { continue; };
                let has_bg = (c.flags & (1 << 0)) != 0;
                if !has_bg {
                    continue;
                }
                let bg = c.bg;
                if bg == Self::DEFAULT_BG {
                    continue;
                }
                self.bg_scratch.push(BgInstance {
                    cell_xy: [x as u16, y as u16],
                    _pad0: [0, 0],
                    bg_rgba: bg,
                    _pad1: [0; 12],
                });
            }
        }
        let mut shared = self.shared.lock().unwrap();
        shared.cols = cols as u16;
        shared.rows = rows as u16;
        std::mem::swap(&mut shared.bg_instances, &mut self.bg_scratch);
    }

    pub fn build_glyph_instances(&mut self, cols: u16, rows: u16) {
        let cols = cols.max(1) as usize;
        let rows = rows.max(1) as usize;
        self.glyph_scratch.clear();
        self.glyph_scratch.reserve(cols * rows / 2);
        let mut diag = GlyphBuildDiag::default();
        diag.total_cells = cols * rows;
        let mut used_slots: Vec<u16> = Vec::new();

        let mut atlas = self.glyph_atlas.lock().unwrap();
        for y in 0..rows {
            for x in 0..cols {
                let idx = y * cols + x;
                let Some(c) = self.grid.cells.get(idx) else { continue; };
                let ch = c.ch;
                // Cell flags: bit0 has_bg, bit1 bold, bit2 underline (see `CellGrid::update_row`).
                let mut gpu_flags = 0u16;
                if c.flags & (1 << 1) != 0 {
                    gpu_flags |= 1;
                }
                if c.flags & (1 << 2) != 0 {
                    gpu_flags |= 2;
                }
                if c.flags & (1 << 3) != 0 {
                    gpu_flags |= 4;
                }
                if c.flags & (1 << 4) != 0 {
                    gpu_flags |= 8;
                }
                let is_space = ch == u32::from(' ');
                if !is_space {
                    diag.non_space_cells = diag.non_space_cells.saturating_add(1);
                }
                let glyph_slot = if is_space {
                    0u16
                } else {
                    atlas.slot_for_char(ch)
                };
                let atlas_page = if glyph_slot == 0 {
                    0u16
                } else {
                    let spp = (ATLAS_PX / GLYPH_SLOT_PX) * (ATLAS_PX / GLYPH_SLOT_PX);
                    let tile = (glyph_slot as u32).saturating_sub(1);
                    let page = tile / spp.max(1);
                    page.min(0xffff) as u16
                };
                let uv_rect = if glyph_slot != 0 {
                    atlas
                        .slot_uv_rect_u16(glyph_slot)
                        .unwrap_or_else(Self::full_uv_rect)
                } else {
                    Self::full_uv_rect()
                };
                let dst_rect = if glyph_slot != 0 {
                    atlas
                        .slot_dst_rect_u16(glyph_slot)
                        .unwrap_or_else(Self::full_uv_rect)
                } else {
                    Self::full_uv_rect()
                };
                if !is_space && glyph_slot == 0 {
                    diag.zero_slot_cells = diag.zero_slot_cells.saturating_add(1);
                }
                let need_quad = glyph_slot != 0 || (gpu_flags & (2 | 8)) != 0;
                if !need_quad {
                    continue;
                }
                if glyph_slot != 0 {
                    used_slots.push(glyph_slot);
                }
                if is_space && (gpu_flags & 2) != 0 {
                    diag.underline_space_quads = diag.underline_space_quads.saturating_add(1);
                }
                self.glyph_scratch.push(GlyphInstance {
                    cell_xy: [x as u16, y as u16],
                    glyph_slot,
                    atlas_page,
                    flags: gpu_flags,
                    _pad0: 0,
                    fg_rgba: c.fg,
                    uv_rect,
                    dst_rect: if (gpu_flags & (2 | 8)) != 0 {
                        // Keep underline/strikethrough geometry anchored to full cell.
                        Self::full_uv_rect()
                    } else {
                        dst_rect
                    },
                    _pad: [0; 4],
                });
            }
        }
        if !used_slots.is_empty() {
            used_slots.sort_unstable();
            used_slots.dedup();
            if let Some(BaselineJitterStats {
                off_y_stddev,
                bbox_top_drift,
                bbox_bottom_drift,
            }) = atlas.baseline_jitter_for_slots(&used_slots)
            {
                diag.off_y_stddev = off_y_stddev;
                diag.bbox_top_drift = bbox_top_drift;
                diag.bbox_bottom_drift = bbox_bottom_drift;
            }
            if let Some(BBoxWidthStats {
                w_over_slot_min,
                w_over_slot_mean,
                w_over_slot_max,
            }) = atlas.bbox_width_stats_for_slots(&used_slots)
            {
                diag.bbox_w_over_slot_min = w_over_slot_min;
                diag.bbox_w_over_slot_mean = w_over_slot_mean;
                diag.bbox_w_over_slot_max = w_over_slot_max;
            }
        }
        // Log-friendly layout diagnostics (mono advance is from the currently loaded font).
        if let Some(ma) = atlas.font_diag_mono_advance() {
            diag.font_mono_advance = ma;
        }
        let (_slot_px, cell_w) = atlas.layout_diag();
        if let Some(w) = cell_w {
            diag.cell_width_px = w;
        }
        drop(atlas);
        diag.glyph_instances = self.glyph_scratch.len();

        let mut shared = self.shared.lock().unwrap();
        std::mem::swap(&mut shared.glyph_instances, &mut self.glyph_scratch);
        shared.glyph_build_diag = diag;
    }

    pub fn update_viewport_params(
        &mut self,
        viewport_px: (u32, u32),
        cell_size_px: (f32, f32),
        origin_px: (f32, f32),
        line_height_ratio: f32,
    ) {
        // Feed the atlas layout the final on-screen cell height (px) + line-height ratio.
        // Atlas rasterization uses "font px" (cell_height / ratio), while layout uses a slot
        // size derived from font px to cover ascender/descender.
        if let Ok(mut atlas) = self.glyph_atlas.lock() {
            atlas.set_target_cell_metrics(cell_size_px.0, cell_size_px.1, line_height_ratio);
        }
        let mut shared = self.shared.lock().unwrap();
        shared.viewport_px = viewport_px;
        shared.cell_size_px = cell_size_px;
        shared.origin_px = origin_px;
    }
}

struct OffscreenTarget {
    #[allow(dead_code)]
    texture: std::sync::Arc<wgpu::Texture>,
    view: std::sync::Arc<wgpu::TextureView>,
}

#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
struct Cell {
    // Unicode scalar value for the cell (space if empty).
    ch: u32,
    fg: [u8; 4],
    bg: [u8; 4],
    flags: u8, // bit0 has_bg, bit1 bold, bit2 underline
}

#[derive(Default)]
struct CellGrid {
    cols: usize,
    rows: usize,
    cells: Vec<Cell>,
    /// Once true, the current visible content includes multi-codepoint graphemes.
    /// Our GPU atlas currently rasterizes single `char` only, so we must fall back to CPU text.
    seen_complex_grapheme: bool,
}


