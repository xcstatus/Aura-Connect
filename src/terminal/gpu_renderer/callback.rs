use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

static OFFSCREEN_DUMP_DONE: AtomicBool = AtomicBool::new(false);
static ATLAS_DUMP_DONE: AtomicBool = AtomicBool::new(false);

fn glyph_uv_crop_ratio() -> f32 {
    std::env::var("RUST_SSH_GPU_GLYPH_UV_CROP")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.0)
        .clamp(0.0, 0.30)
}

fn glyph_alpha_boost() -> f32 {
    std::env::var("RUST_SSH_GPU_GLYPH_ALPHA_BOOST")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(1.25)
        .clamp(0.5, 3.0)
}

impl TerminalOffscreenClearCallback {
    pub(crate) fn new(
        texture: std::sync::Arc<wgpu::Texture>,
        view: std::sync::Arc<wgpu::TextureView>,
        shared: Arc<Mutex<SharedFrameData>>,
        atlas: GlyphAtlasShared,
    ) -> Self {
        Self {
            texture,
            view,
            shared,
            atlas,
        }
    }

    fn should_dump_offscreen() -> bool {
        std::env::var("RUST_SSH_TERM_DUMP_OFFSCREEN")
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "True"))
            .unwrap_or(false)
    }

    fn dump_dir() -> std::path::PathBuf {
        if let Ok(raw) = std::env::var("RUST_SSH_TERM_DUMP_DIR") {
            let p = std::path::PathBuf::from(raw);
            if !p.as_os_str().is_empty() {
                return p;
            }
        }
        std::path::PathBuf::from("term-dumps")
    }

    fn unix_ts() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn maybe_schedule_offscreen_dump(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        egui_encoder: &mut wgpu::CommandEncoder,
    ) {
        if !Self::should_dump_offscreen() {
            return;
        }
        let (bg_n, glyph_n) = {
            let shared = self.shared.lock().unwrap();
            (shared.bg_instances.len(), shared.glyph_instances.len())
        };
        // Avoid dumping the startup clear frame before terminal data is populated.
        if bg_n == 0 && glyph_n == 0 {
            return;
        }
        if OFFSCREEN_DUMP_DONE
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }

        let size = self.texture.size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        let width = size.width;
        let height = size.height;
        let unpadded_bytes_per_row = width.saturating_mul(4);
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(256) * 256;
        let copy_size = padded_bytes_per_row as u64 * height as u64;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terminal_offscreen_readback"),
            size: copy_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        egui_encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let dump_dir = Self::dump_dir();
        let out_path = dump_dir.join(format!("{}_offscreen_real.png", Self::unix_ts()));
        let out_path_for_log = out_path.clone();
        let readback_for_submit = readback.clone();
        queue.on_submitted_work_done(move || {
            let readback_for_map = readback_for_submit.clone();
            let dump_dir_for_map = dump_dir.clone();
            let out_path_for_map = out_path.clone();
            readback_for_submit.slice(..).map_async(wgpu::MapMode::Read, move |res| {
                if let Err(e) = res {
                    log::warn!(target: "term-diag", "GPU offscreen dump map failed: {e:?}");
                    return;
                }
                if let Err(e) = std::fs::create_dir_all(&dump_dir_for_map) {
                    log::warn!(target: "term-diag", "GPU offscreen dump create dir failed: {e}");
                    readback_for_map.unmap();
                    return;
                }
                let mapped = readback_for_map.slice(..).get_mapped_range();
                let mut tight = vec![0u8; (width as usize) * (height as usize) * 4];
                let src = mapped.as_ref();
                for y in 0..height as usize {
                    let src_row_start = y * padded_bytes_per_row as usize;
                    let src_row_end = src_row_start + unpadded_bytes_per_row as usize;
                    let dst_row_start = y * unpadded_bytes_per_row as usize;
                    let dst_row_end = dst_row_start + unpadded_bytes_per_row as usize;
                    tight[dst_row_start..dst_row_end]
                        .copy_from_slice(&src[src_row_start..src_row_end]);
                }
                drop(mapped);
                readback_for_map.unmap();

                match image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(width, height, tight) {
                    Some(img) => {
                        if let Err(e) = img.save(&out_path_for_map) {
                            log::warn!(target: "term-diag", "GPU offscreen dump save failed: {e}");
                        } else {
                            log::info!(
                                target: "term-diag",
                                "GPU offscreen real dump saved: {}",
                                out_path_for_map.display()
                            );
                        }
                    }
                    None => {
                        log::warn!(target: "term-diag", "GPU offscreen dump invalid image buffer");
                    }
                }
            });
        });
        log::info!(
            target: "term-diag",
            "GPU offscreen real dump scheduled: {} ({}x{}, bg_instances={}, glyph_instances={})",
            out_path_for_log.display(),
            width,
            height,
            bg_n,
            glyph_n
        );
    }

    fn should_dump_atlas() -> bool {
        std::env::var("RUST_SSH_TERM_DUMP_ATLAS")
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "True"))
            .unwrap_or(false)
    }

    fn maybe_dump_atlas_preview(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        egui_encoder: &mut wgpu::CommandEncoder,
        atlas_tex: &wgpu::Texture,
    ) {
        if !Self::should_dump_atlas() {
            return;
        }
        if ATLAS_DUMP_DONE
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }
        let dump_dir = Self::dump_dir();
        let out_path = dump_dir.join(format!("{}_atlas_preview.png", Self::unix_ts()));
        let out_path_for_log = out_path.clone();

        let width: u32 = 512;
        let height: u32 = 512;
        let bytes_per_row_unpadded = width; // R8
        let bytes_per_row_padded = bytes_per_row_unpadded.div_ceil(256) * 256;
        let copy_size = (bytes_per_row_padded as u64) * (height as u64);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terminal_atlas_preview_readback"),
            size: copy_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        egui_encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: atlas_tex,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row_padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let readback_for_submit = readback.clone();
        queue.on_submitted_work_done(move || {
            let readback_for_map = readback_for_submit.clone();
            let dump_dir_for_map = dump_dir.clone();
            let out_path_for_map = out_path.clone();
            readback_for_submit.slice(..).map_async(wgpu::MapMode::Read, move |res| {
                if let Err(e) = res {
                    log::warn!(target: "term-diag", "atlas dump map failed: {e:?}");
                    return;
                }
                if let Err(e) = std::fs::create_dir_all(&dump_dir_for_map) {
                    log::warn!(target: "term-diag", "atlas dump create dir failed: {e}");
                    readback_for_map.unmap();
                    return;
                }
                let mapped = readback_for_map.slice(..).get_mapped_range();
                let src = mapped.as_ref();
                let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];
                for y in 0..height as usize {
                    let src_row = y * (bytes_per_row_padded as usize);
                    let dst_row = y * (width as usize) * 4;
                    for x in 0..width as usize {
                        let a = src[src_row + x];
                        let di = dst_row + x * 4;
                        rgba[di] = a;
                        rgba[di + 1] = a;
                        rgba[di + 2] = a;
                        rgba[di + 3] = 255;
                    }
                }
                drop(mapped);
                readback_for_map.unmap();

                if let Some(img) =
                    image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(width, height, rgba)
                {
                    if let Err(e) = img.save(&out_path_for_map) {
                        log::warn!(target: "term-diag", "atlas dump save failed: {e}");
                    } else {
                        log::info!(
                            target: "term-diag",
                            "atlas preview saved: {}",
                            out_path_for_map.display()
                        );
                    }
                } else {
                    log::warn!(target: "term-diag", "atlas dump invalid image buffer");
                }
            });
        });

        log::info!(
            target: "term-diag",
            "atlas preview scheduled: {} ({}x{})",
            out_path_for_log.display(),
            width,
            height
        );
    }
}

impl egui_wgpu::CallbackTrait for TerminalOffscreenClearCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let need_glyphs = {
            let g = self.shared.lock().unwrap();
            !g.glyph_instances.is_empty()
        };
        if need_glyphs {
            if callback_resources.get::<GlyphResources>().is_none() {
                callback_resources.insert(GlyphResources::new(device));
            }
            let gr = callback_resources
                .get_mut::<GlyphResources>()
                .expect("GlyphResources inserted");
            let mut atlas = self.atlas.lock().unwrap();
            atlas.upload_dirty(queue, &gr.atlas_tex);
            self.maybe_dump_atlas_preview(device, queue, egui_encoder, &gr.atlas_tex);
        }

        // All `queue.write_buffer` / atlas uploads MUST finish before `begin_render_pass`.
        // Writing vertex/uniform data while a render pass using those buffers is open is undefined
        // on some backends (symptom: blank terminal on GPU path).
        let guard = self.shared.lock().unwrap();
        let shared = &*guard;

        let bg_inst_n = if shared.cols > 0 && shared.rows > 0 && !shared.bg_instances.is_empty() {
            if callback_resources.get::<BgResources>().is_none() {
                callback_resources.insert(BgResources::new(device));
            }
            let bg = callback_resources
                .get_mut::<BgResources>()
                .expect("BgResources inserted");
            let u = BgUniform {
                viewport_px: [shared.viewport_px.0 as f32, shared.viewport_px.1 as f32],
                cell_size_px: [shared.cell_size_px.0, shared.cell_size_px.1],
                origin_px: [shared.origin_px.0, shared.origin_px.1],
                _pad: [0.0, 0.0],
            };
            queue.write_buffer(&bg.uniform_buf, 0, bytemuck::bytes_of(&u));
            bg.ensure_instance_capacity(device, shared.bg_instances.len());
            queue.write_buffer(
                &bg.instance_buf,
                0,
                bytemuck::cast_slice(&shared.bg_instances),
            );
            Some(shared.bg_instances.len() as u32)
        } else {
            None
        };

        let glyph_inst_n = if !shared.glyph_instances.is_empty() {
            let gr = callback_resources
                .get_mut::<GlyphResources>()
                .expect("GlyphResources inserted with glyph_instances");
            let (atlas_cols, slot_px, slots_per_page) = {
                let atlas = self.atlas.lock().unwrap();
                let cols = (atlas.atlas_w / atlas.slot_px).max(1) as f32;
                let sp = atlas.slot_px.max(1) as f32;
                let spp = ((atlas.atlas_w / atlas.slot_px).max(1) * (atlas.atlas_h / atlas.slot_px).max(1)) as f32;
                (cols, sp, spp)
            };
            let u = GlyphUniform {
                viewport_px: [shared.viewport_px.0 as f32, shared.viewport_px.1 as f32],
                cell_size_px: [shared.cell_size_px.0, shared.cell_size_px.1],
                origin_px: [shared.origin_px.0, shared.origin_px.1],
                atlas_w: ATLAS_PX as f32,
                atlas_h: ATLAS_PX as f32,
                slot_px,
                atlas_cols,
                slots_per_page,
                glyph_uv_crop: glyph_uv_crop_ratio(),
                glyph_alpha_boost: glyph_alpha_boost(),
                _pad0: 0.0,
            };
            queue.write_buffer(&gr.uniform_buf, 0, bytemuck::bytes_of(&u));
            gr.ensure_instance_capacity(device, shared.glyph_instances.len());
            queue.write_buffer(
                &gr.instance_buf,
                0,
                bytemuck::cast_slice(&shared.glyph_instances),
            );
            Some(shared.glyph_instances.len() as u32)
        } else {
            None
        };

        drop(guard);

        {
            let mut rp = egui_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terminal_offscreen_clear_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 10.0 / 255.0,
                            g: 10.0 / 255.0,
                            b: 15.0 / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let Some(n) = bg_inst_n {
                let bg = callback_resources
                    .get::<BgResources>()
                    .expect("BgResources prepared");
                rp.set_pipeline(&bg.pipeline);
                rp.set_bind_group(0, &bg.bind_group, &[]);
                rp.set_vertex_buffer(0, bg.vertex_buf.slice(..));
                rp.set_vertex_buffer(1, bg.instance_buf.slice(..));
                rp.set_index_buffer(bg.index_buf.slice(..), wgpu::IndexFormat::Uint16);
                rp.draw_indexed(0..bg.index_count, 0, 0..n);
            }

            if let Some(n) = glyph_inst_n {
                let gr = callback_resources
                    .get::<GlyphResources>()
                    .expect("GlyphResources prepared");
                rp.set_pipeline(&gr.pipeline);
                rp.set_bind_group(0, &gr.bind_group0, &[]);
                rp.set_bind_group(1, &gr.bind_group1, &[]);
                rp.set_vertex_buffer(0, gr.vertex_buf.slice(..));
                rp.set_vertex_buffer(1, gr.instance_buf.slice(..));
                rp.set_index_buffer(gr.index_buf.slice(..), wgpu::IndexFormat::Uint16);
                rp.draw_indexed(0..gr.index_count, 0, 0..n);
            }
        }

        self.maybe_schedule_offscreen_dump(device, queue, egui_encoder);

        Vec::new()
    }

    fn paint(
        &self,
        _info: eframe::egui::PaintCallbackInfo,
        _render_pass: &mut wgpu::RenderPass<'static>,
        _callback_resources: &egui_wgpu::CallbackResources,
    ) {
        // No-op: we already rendered to offscreen in prepare().
    }
}
