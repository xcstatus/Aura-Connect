use std::path::{Path, PathBuf};
use std::collections::{BTreeMap, BTreeSet};

use ab_glyph::{point, Font, FontArc, Glyph, PxScale, ScaleFont};
use anyhow::Result;
use image::{Rgba, RgbaImage};

use crate::backend::ghostty_vt::VtStyledRow;
use crate::terminal::glyph_atlas::AtlasPixelSnapshot;
use crate::terminal::gpu_renderer::{GpuFrameDumpSnapshot, GpuGlyphDumpInstance};

#[derive(Clone, Debug)]
pub(crate) struct CompareDumpResult {
    pub dir: PathBuf,
    pub cpu_path: PathBuf,
    pub gpu_path: PathBuf,
    pub diff_path: PathBuf,
    pub compare_path: PathBuf,
    pub used_slots_path: PathBuf,
    pub used_slots_manifest_path: PathBuf,
    pub mean_abs_diff: f32,
    pub diff_ratio: f32,
    pub gpu_cov_mean: f32,
    pub gpu_cov_nonzero_ratio: f32,
}

pub(crate) fn write_compare_dump(
    rows: &[VtStyledRow],
    gpu: &GpuFrameDumpSnapshot,
    atlas: &AtlasPixelSnapshot,
) -> Result<CompareDumpResult> {
    let dir = dump_dir();
    std::fs::create_dir_all(&dir)?;

    let mut cpu = RgbaImage::from_pixel(
        gpu.viewport_px.0.max(1),
        gpu.viewport_px.1.max(1),
        Rgba([10, 10, 15, 255]),
    );
    let mut gpu_img = RgbaImage::from_pixel(
        gpu.viewport_px.0.max(1),
        gpu.viewport_px.1.max(1),
        Rgba([10, 10, 15, 255]),
    );

    render_cpu_reference(&mut cpu, rows, gpu)?;
    let (gpu_cov_mean, gpu_cov_nonzero_ratio) = render_gpu_simulation(&mut gpu_img, gpu, atlas);

    let (diff, mean_abs_diff, diff_ratio) = make_diff_image(&cpu, &gpu_img);
    let compare = stitch_compare(&cpu, &gpu_img, &diff);

    let ts = chrono_like_timestamp();
    let cpu_path = dir.join(format!("{ts}_cpu.png"));
    let gpu_path = dir.join(format!("{ts}_gpu_sim.png"));
    let diff_path = dir.join(format!("{ts}_diff.png"));
    let compare_path = dir.join(format!("{ts}_compare.png"));
    let used_slots_path = dir.join(format!("{ts}_used_slots.png"));
    let used_slots_manifest_path = dir.join(format!("{ts}_used_slots.tsv"));

    cpu.save(&cpu_path)?;
    gpu_img.save(&gpu_path)?;
    diff.save(&diff_path)?;
    compare.save(&compare_path)?;
    export_used_slots_sheet(
        atlas,
        &gpu.glyph_instances,
        &used_slots_path,
        &used_slots_manifest_path,
    )?;

    Ok(CompareDumpResult {
        dir,
        cpu_path,
        gpu_path,
        diff_path,
        compare_path,
        used_slots_path,
        used_slots_manifest_path,
        mean_abs_diff,
        diff_ratio,
        gpu_cov_mean,
        gpu_cov_nonzero_ratio,
    })
}

fn export_used_slots_sheet(
    atlas: &AtlasPixelSnapshot,
    glyph_instances: &[GpuGlyphDumpInstance],
    slots_path: &Path,
    manifest_path: &Path,
) -> Result<()> {
    let mut used_counts: BTreeMap<u16, u32> = BTreeMap::new();
    for g in glyph_instances {
        if g.glyph_slot != 0 {
            *used_counts.entry(g.glyph_slot).or_insert(0) += 1;
        }
    }
    let used_slots: BTreeSet<u16> = used_counts.keys().copied().collect();
    if used_slots.is_empty() {
        let empty = RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 255]));
        empty.save(slots_path)?;
        std::fs::write(
            manifest_path,
            b"slot\tpage\tslot_in_page\tcount\tch\tfont\tbounds_w\tbounds_h\toff_x\toff_y\tpixels_written\tfallback_used\n",
        )?;
        return Ok(());
    }

    let tile = atlas.slot_px.max(1);
    let pad = 2u32;
    let cols = 16u32;
    let rows = ((used_slots.len() as u32) + cols - 1) / cols;
    let w = cols * tile + (cols + 1) * pad;
    let h = rows * tile + (rows + 1) * pad;
    let mut out = RgbaImage::from_pixel(w, h, Rgba([12, 12, 16, 255]));
    let atlas_cols = (atlas.atlas_w / atlas.slot_px).max(1);
    let slots_per_page = atlas_cols * atlas_cols;
    let page_px = (atlas.atlas_w * atlas.atlas_h) as usize;

    for (i, slot) in used_slots.iter().enumerate() {
        let i = i as u32;
        let gx = i % cols;
        let gy = i / cols;
        let dx0 = pad + gx * (tile + pad);
        let dy0 = pad + gy * (tile + pad);

        let global = (*slot as u32).saturating_sub(1);
        let page = if slots_per_page > 0 { global / slots_per_page } else { 0 };
        let in_page = if slots_per_page > 0 { global % slots_per_page } else { 0 };
        if page >= atlas.pages {
            continue;
        }
        let sx = in_page % atlas_cols;
        let sy = in_page / atlas_cols;
        let ox = sx * atlas.slot_px;
        let oy = sy * atlas.slot_px;
        let base = (page as usize) * page_px;

        for y in 0..tile {
            for x in 0..tile {
                let ax = ox + x;
                let ay = oy + y;
                if ax >= atlas.atlas_w || ay >= atlas.atlas_h {
                    continue;
                }
                let idx = base + (ay * atlas.atlas_w + ax) as usize;
                if idx >= atlas.pixels.len() {
                    continue;
                }
                let a = atlas.pixels[idx];
                out.put_pixel(dx0 + x, dy0 + y, Rgba([a, a, a, 255]));
            }
        }
    }

    out.save(slots_path)?;

    let mut tsv = String::from(
        "slot\tpage\tslot_in_page\tcount\tch\tfont\tbounds_w\tbounds_h\toff_x\toff_y\tpixels_written\tfallback_used\n",
    );
    let debug_by_slot: BTreeMap<u16, _> = atlas
        .slot_debug
        .iter()
        .map(|d| (d.slot, d))
        .collect();
    for slot in used_slots {
        let count = used_counts.get(&slot).copied().unwrap_or(0);
        let global = (slot as u32).saturating_sub(1);
        let page = if slots_per_page > 0 { global / slots_per_page } else { 0 };
        let in_page = if slots_per_page > 0 { global % slots_per_page } else { 0 };
        if let Some(d) = debug_by_slot.get(&slot) {
            let ch = char::from_u32(d.ch).unwrap_or('?');
            tsv.push_str(&format!(
                "{}\t{}\t{}\t{}\tU+{:04X}({})\t{}\t{:.2}\t{:.2}\t{}\t{}\t{}\t{}\n",
                slot,
                page,
                in_page,
                count,
                d.ch,
                ch,
                d.font_label.replace('\t', " "),
                d.bounds_w,
                d.bounds_h,
                d.off_x,
                d.off_y,
                d.pixels_written,
                d.fallback_used
            ));
        } else {
            tsv.push_str(&format!(
                "{}\t{}\t{}\t{}\t<missing>\t<missing>\t0\t0\t0\t0\t0\tfalse\n",
                slot, page, in_page, count
            ));
        }
    }
    std::fs::write(manifest_path, tsv)?;
    Ok(())
}

fn render_cpu_reference(img: &mut RgbaImage, rows: &[VtStyledRow], gpu: &GpuFrameDumpSnapshot) -> Result<()> {
    let cell_w = gpu.cell_size_px.0.max(1.0);
    let cell_h = gpu.cell_size_px.1.max(1.0);
    let origin_x = gpu.origin_px.0;
    let origin_y = gpu.origin_px.1;

    let font = load_font()?;
    let scale_px = (cell_h * 0.82).max(1.0);
    let scaled = font.as_scaled(PxScale::from(scale_px));
    let ascent = scaled.ascent();
    let line_h = scaled.height().max(1.0);
    let top_pad = ((cell_h - line_h) * 0.5).max(0.0);

    for (y, row) in rows.iter().enumerate() {
        let mut x = 0usize;
        for run in &row.runs {
            if run.cols == 0 {
                continue;
            }
            let fg0 = [run.fg.r, run.fg.g, run.fg.b, 255];
            let fg = if run.dim {
                [
                    (fg0[0] as u16 * 160 / 255) as u8,
                    (fg0[1] as u16 * 160 / 255) as u8,
                    (fg0[2] as u16 * 160 / 255) as u8,
                    255,
                ]
            } else {
                fg0
            };
            let bg = [run.bg.r, run.bg.g, run.bg.b, 255];
            if run.has_bg {
                fill_rect(
                    img,
                    origin_x + (x as f32) * cell_w,
                    origin_y + (y as f32) * cell_h,
                    (run.cols as f32) * cell_w,
                    cell_h,
                    bg,
                );
            }

            // Decorations (underline / strikethrough) are drawn per-run using cell geometry,
            // matching the GPU path semantics.
            if run.underline {
                fill_rect(
                    img,
                    origin_x + (x as f32) * cell_w,
                    origin_y + (y as f32) * cell_h + cell_h * 0.90,
                    (run.cols as f32) * cell_w,
                    (cell_h * 0.08).max(1.0),
                    fg,
                );
            }
            if run.strikethrough {
                fill_rect(
                    img,
                    origin_x + (x as f32) * cell_w,
                    origin_y + (y as f32) * cell_h + cell_h * 0.55,
                    (run.cols as f32) * cell_w,
                    (cell_h * 0.06).max(1.0),
                    fg,
                );
            }

            for ch in run.text.chars() {
                if ch != ' ' {
                    let px = origin_x + (x as f32) * cell_w + 1.0;
                    let py = origin_y + (y as f32) * cell_h + top_pad + ascent;
                    draw_glyph(img, &font, ch, scale_px, px, py, fg);
                    if run.bold {
                        // Cheap synthetic bold similar to CPU fallback.
                        draw_glyph(img, &font, ch, scale_px, px + 0.7, py, fg);
                    }
                }
                x += 1;
            }
        }
    }
    Ok(())
}

fn glyph_alpha_boost() -> f32 {
    std::env::var("RUST_SSH_GPU_GLYPH_ALPHA_BOOST")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(1.0)
        .clamp(0.5, 3.0)
}

fn render_gpu_simulation(
    img: &mut RgbaImage,
    gpu: &GpuFrameDumpSnapshot,
    atlas: &AtlasPixelSnapshot,
) -> (f32, f32) {
    let cell_w = gpu.cell_size_px.0.max(1.0);
    let cell_h = gpu.cell_size_px.1.max(1.0);
    let origin_x = gpu.origin_px.0;
    let origin_y = gpu.origin_px.1;
    let atlas_cols = (atlas.atlas_w / atlas.slot_px).max(1);
    let mut cov_sum = 0.0f64;
    let mut cov_count = 0u64;
    let mut cov_nonzero = 0u64;
    // Force use of `pages` so multi-page dumps do not warn as unused.
    let _pages = atlas.pages;

    for b in &gpu.bg_instances {
        fill_rect(
            img,
            origin_x + (b.cell_xy[0] as f32) * cell_w,
            origin_y + (b.cell_xy[1] as f32) * cell_h,
            cell_w,
            cell_h,
            b.bg_rgba,
        );
    }

    for g in &gpu.glyph_instances {
        render_gpu_glyph_cell(
            img,
            g,
            atlas,
            atlas_cols,
            origin_x + (g.cell_xy[0] as f32) * cell_w,
            origin_y + (g.cell_xy[1] as f32) * cell_h,
            cell_w,
            cell_h,
            &mut cov_sum,
            &mut cov_count,
            &mut cov_nonzero,
        );
    }
    let mean = if cov_count > 0 {
        (cov_sum / cov_count as f64) as f32
    } else {
        0.0
    };
    let nonzero_ratio = if cov_count > 0 {
        cov_nonzero as f32 / cov_count as f32
    } else {
        0.0
    };
    (mean, nonzero_ratio)
}

fn render_gpu_glyph_cell(
    img: &mut RgbaImage,
    g: &GpuGlyphDumpInstance,
    atlas: &AtlasPixelSnapshot,
    atlas_cols: u32,
    cell_x: f32,
    cell_y: f32,
    cell_w: f32,
    cell_h: f32,
    cov_sum: &mut f64,
    cov_count: &mut u64,
    cov_nonzero: &mut u64,
) {
    let alpha_boost = glyph_alpha_boost();
    let dim_alpha = if (g.flags & 4) != 0 { 0.65 } else { 1.0 };
    let dx0 = g.dst_rect[0] as f32 / 65535.0;
    let dy0 = g.dst_rect[1] as f32 / 65535.0;
    let dx1 = g.dst_rect[2] as f32 / 65535.0;
    let dy1 = g.dst_rect[3] as f32 / 65535.0;
    let dmin_x = dx0.min(dx1);
    let dmin_y = dy0.min(dy1);
    let dmax_x = dx0.max(dx1);
    let dmax_y = dy0.max(dy1);
    let dst_w = ((dmax_x - dmin_x).max(1.0 / atlas.slot_px.max(1) as f32)) * cell_w;
    let dst_h = ((dmax_y - dmin_y).max(1.0 / atlas.slot_px.max(1) as f32)) * cell_h;
    let draw_x = cell_x + dmin_x * cell_w;
    let draw_y = cell_y + dmin_y * cell_h;
    if g.glyph_slot != 0 {
        let slots_per_page = atlas_cols * atlas_cols;
        let global = (g.glyph_slot as u32).saturating_sub(1);
        let page = if g.atlas_page != 0 {
            g.atlas_page as u32
        } else if slots_per_page > 0 {
            global / slots_per_page
        } else {
            0
        };
        let in_page = if slots_per_page > 0 { global % slots_per_page } else { 0 };
        if page >= atlas.pages {
            return;
        }
        let sx = in_page % atlas_cols;
        let sy = in_page / atlas_cols;
        let ox = sx * atlas.slot_px;
        let oy = sy * atlas.slot_px;
        let page_px = (atlas.atlas_w * atlas.atlas_h) as usize;
        let base = (page as usize) * page_px;

        let u0 = g.uv_rect[0] as f32 / 65535.0;
        let v0 = g.uv_rect[1] as f32 / 65535.0;
        let u1 = g.uv_rect[2] as f32 / 65535.0;
        let v1 = g.uv_rect[3] as f32 / 65535.0;
        let du = (u1 - u0).max(1.0 / atlas.slot_px.max(1) as f32);
        let dv = (v1 - v0).max(1.0 / atlas.slot_px.max(1) as f32);

        let x0 = draw_x.floor().max(0.0) as i32;
        let y0 = draw_y.floor().max(0.0) as i32;
        let x1 = (draw_x + dst_w).ceil().max(0.0) as i32;
        let y1 = (draw_y + dst_h).ceil().max(0.0) as i32;

        for y in y0..y1 {
            for x in x0..x1 {
                if x < 0 || y < 0 || x as u32 >= img.width() || y as u32 >= img.height() {
                    continue;
                }
                let u = ((x as f32 + 0.5) - draw_x) / dst_w;
                let v = ((y as f32 + 0.5) - draw_y) / dst_h;
                if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                    continue;
                }
                let su = (u0 + u * du).clamp(0.0, 1.0);
                let sv = (v0 + v * dv).clamp(0.0, 1.0);
                let ax = ox + (su * atlas.slot_px as f32).floor() as u32;
                let ay = oy + (sv * atlas.slot_px as f32).floor() as u32;
                if ax >= atlas.atlas_w || ay >= atlas.atlas_h {
                    continue;
                }
                let idx = base + (ay * atlas.atlas_w + ax) as usize;
                if idx >= atlas.pixels.len() {
                    continue;
                }
                let a = atlas.pixels[idx];
                let boosted = ((a as f32 / 255.0) * alpha_boost).clamp(0.0, 1.0);
                *cov_sum += boosted as f64;
                *cov_count += 1;
                if boosted > 0.0 {
                    *cov_nonzero += 1;
                }
                if a == 0 {
                    continue;
                }
                blend_over(
                    img.get_pixel_mut(x as u32, y as u32),
                    g.fg_rgba,
                    ((boosted * dim_alpha) * 255.0).round().clamp(0.0, 255.0) as u8,
                );
            }
        }
    }

    // Underline flag bit1 in GPU data (value 2).
    if (g.flags & 2) != 0 {
        fill_rect(
            img,
            cell_x,
            cell_y + cell_h * 0.90,
            cell_w,
            (cell_h * 0.08).max(1.0),
            g.fg_rgba,
        );
    }
    // Strikethrough flag bit3 in GPU data (value 8).
    if (g.flags & 8) != 0 {
        fill_rect(
            img,
            cell_x,
            cell_y + cell_h * 0.55,
            cell_w,
            (cell_h * 0.06).max(1.0),
            g.fg_rgba,
        );
    }
}

fn load_font() -> Result<FontArc> {
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("RUST_SSH_FONT_TTC") {
        paths.push(PathBuf::from(p));
    }
    paths.push(PathBuf::from("/System/Library/Fonts/Menlo.ttc"));
    paths.push(PathBuf::from("/System/Library/Fonts/Monaco.ttf"));
    paths.push(PathBuf::from("/System/Library/Fonts/Supplemental/Arial Unicode.ttf"));
    for p in paths {
        if let Ok(data) = std::fs::read(&p) {
            let face_count = ttf_parser::fonts_in_collection(&data).unwrap_or(1).max(1);
            for idx in 0..face_count {
                if let Ok(fv) = ab_glyph::FontVec::try_from_vec_and_index(data.clone(), idx) {
                    return Ok(FontArc::new(fv));
                }
            }
        }
    }
    anyhow::bail!("no usable font for compare dump")
}

fn draw_glyph(img: &mut RgbaImage, font: &FontArc, ch: char, scale_px: f32, x: f32, y: f32, fg: [u8; 4]) {
    let g: Glyph = font
        .glyph_id(ch)
        .with_scale_and_position(PxScale::from(scale_px), point(x, y));
    let Some(outlined) = font.outline_glyph(g) else { return; };
    outlined.draw(|dx, dy, cov| {
        let px = x.floor() as i32 + dx as i32;
        let py = y.floor() as i32 + dy as i32;
        if px < 0 || py < 0 || px as u32 >= img.width() || py as u32 >= img.height() {
            return;
        }
        let a = (cov.clamp(0.0, 1.0) * 255.0) as u8;
        if a > 0 {
            blend_over(img.get_pixel_mut(px as u32, py as u32), fg, a);
        }
    });
}

fn blend_over(dst: &mut Rgba<u8>, src_rgb: [u8; 4], src_a: u8) {
    let sa = src_a as f32 / 255.0;
    let da = dst[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0 {
        return;
    }
    for i in 0..3 {
        let s = src_rgb[i] as f32 / 255.0;
        let d = dst[i] as f32 / 255.0;
        let out = (s * sa + d * da * (1.0 - sa)) / out_a;
        dst[i] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    dst[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}

fn fill_rect(img: &mut RgbaImage, x: f32, y: f32, w: f32, h: f32, color: [u8; 4]) {
    let x0 = x.floor().max(0.0) as i32;
    let y0 = y.floor().max(0.0) as i32;
    let x1 = (x + w).ceil().max(0.0) as i32;
    let y1 = (y + h).ceil().max(0.0) as i32;
    for yy in y0..y1 {
        for xx in x0..x1 {
            if xx < 0 || yy < 0 || xx as u32 >= img.width() || yy as u32 >= img.height() {
                continue;
            }
            img.put_pixel(xx as u32, yy as u32, Rgba(color));
        }
    }
}

fn make_diff_image(a: &RgbaImage, b: &RgbaImage) -> (RgbaImage, f32, f32) {
    let w = a.width().min(b.width());
    let h = a.height().min(b.height());
    let mut diff = RgbaImage::new(w, h);
    let mut sum = 0f64;
    let mut changed = 0u64;
    let total = (w as u64) * (h as u64);
    for y in 0..h {
        for x in 0..w {
            let pa = a.get_pixel(x, y);
            let pb = b.get_pixel(x, y);
            let dr = pa[0].abs_diff(pb[0]);
            let dg = pa[1].abs_diff(pb[1]);
            let db = pa[2].abs_diff(pb[2]);
            let d = ((dr as u16 + dg as u16 + db as u16) / 3) as u8;
            sum += d as f64;
            if d > 24 {
                changed += 1;
            }
            diff.put_pixel(x, y, Rgba([d, 0, 255u8.saturating_sub(d), 255]));
        }
    }
    let mean = if total > 0 { (sum / total as f64) as f32 } else { 0.0 };
    let ratio = if total > 0 {
        changed as f32 / total as f32
    } else {
        0.0
    };
    (diff, mean, ratio)
}

fn stitch_compare(cpu: &RgbaImage, gpu: &RgbaImage, diff: &RgbaImage) -> RgbaImage {
    let gap = 8u32;
    let h = cpu.height().max(gpu.height()).max(diff.height());
    let w = cpu.width() + gpu.width() + diff.width() + gap * 2;
    let mut out = RgbaImage::from_pixel(w, h, Rgba([20, 20, 24, 255]));
    blit(&mut out, cpu, 0, 0);
    blit(&mut out, gpu, cpu.width() + gap, 0);
    blit(&mut out, diff, cpu.width() + gap + gpu.width() + gap, 0);
    out
}

fn blit(dst: &mut RgbaImage, src: &RgbaImage, x_off: u32, y_off: u32) {
    for y in 0..src.height() {
        for x in 0..src.width() {
            let dx = x + x_off;
            let dy = y + y_off;
            if dx < dst.width() && dy < dst.height() {
                dst.put_pixel(dx, dy, *src.get_pixel(x, y));
            }
        }
    }
}

#[cfg(test)]
mod golden_tests {
    use super::*;
    use crate::terminal::gpu_renderer::TerminalGpuRenderer;
    use crate::backend::ghostty_vt::{VtStyledRow, VtStyledRun, ffi};

    fn mk_run(text: &str, cols: u16, bold: bool, underline: bool, dim: bool, strike: bool) -> VtStyledRun {
        VtStyledRun {
            text: text.to_string(),
            cols,
            fg: ffi::GhosttyColorRgb { r: 230, g: 230, b: 230 },
            bg: ffi::GhosttyColorRgb { r: 10, g: 10, b: 15 },
            has_bg: false,
            bold,
            underline,
            dim,
            strikethrough: strike,
        }
    }

    fn key_charset_rows(cols: u16, rows: u16) -> Vec<VtStyledRow> {
        let mut out = Vec::new();
        let mut push = |s: &str, bold: bool, underline: bool, dim: bool, strike: bool| {
            let w = unicode_width::UnicodeWidthStr::width(s).max(1);
            let cols = (w as u16).min(cols.max(1));
            out.push(VtStyledRow {
                runs: vec![mk_run(s, cols, bold, underline, dim, strike)],
            });
        };

        push("ASCII: The quick brown fox 0123456789", false, false, false, false);
        push("CJK: 你好 世界 中英混排", false, false, false, false);
        push("Box: ┌─┬─┐ │终│端│ └─┴─┘", false, false, false, false);
        push("Marks: e\u{301} a\u{308} n\u{303}", false, false, false, false);
        push("Style: Bold Underline Strike Dim", true, true, true, true);

        out.resize_with(rows as usize, VtStyledRow::default);
        out
    }

    #[test]
    #[ignore]
    fn golden_compare_dump_key_charset() {
        let cols: u16 = 60;
        let rows: u16 = 10;
        let styled_rows = key_charset_rows(cols, rows);

        let mut r = TerminalGpuRenderer::new();
        r.update_cells_from_rows(cols, rows, &styled_rows, &[]);
        r.update_viewport_params((900, 220), (9.0, 18.0), (6.0, 6.0));
        r.build_bg_instances(cols, rows);
        r.build_glyph_instances(cols, rows);

        let (gpu_snap, atlas_snap) = r.dump_snapshot();
        let res = write_compare_dump(&styled_rows, &gpu_snap, &atlas_snap).expect("dump ok");
        eprintln!(
            "golden compare dump: cpu={} gpu={} diff={}",
            res.cpu_path.display(),
            res.gpu_path.display(),
            res.diff_path.display()
        );
    }
}

fn dump_dir() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SSH_TERM_DUMP_DIR") {
        return PathBuf::from(p);
    }
    Path::new(".").join("term-dumps")
}

fn chrono_like_timestamp() -> String {
    // Keep this lightweight without adding chrono dependency.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", now.as_secs())
}
