use ab_glyph::{point, Font, FontArc, FontVec, Glyph, PxScale};
use std::path::PathBuf;

pub(super) struct LoadedFont {
    pub(super) label: String,
    pub(super) font: FontArc,
    pub(super) ascent: f32,
    pub(super) descent: f32,
    pub(super) line_gap: f32,
    pub(super) height: f32,
    pub(super) mono_advance: f32,
    pub(super) raster_scale_px: f32,
}

fn terminal_font_search_paths(override_path: Option<&str>) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(p) = override_path {
        v.push(PathBuf::from(p));
        return v;
    }
    // Developer override (kept for diagnostics / CLI use).
    if let Ok(p) = std::env::var("RUST_SSH_FONT_TTC") {
        if !p.trim().is_empty() {
            v.push(PathBuf::from(p));
            return v;
        }
    }
    #[cfg(target_os = "macos")]
    {
        v.extend(
            [
                // Prefer monospace for ASCII readability in terminal.
                "/System/Library/Fonts/Menlo.ttc",
                "/System/Library/Fonts/Monaco.ttf",
                "/System/Library/Fonts/Supplemental/Songti.ttc",
                "/System/Library/Fonts/Supplemental/Songti SC.ttc",
                "/System/Library/Fonts/Supplemental/Songti TC.ttc",
                "/Library/Fonts/Songti.ttc",
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            ]
            .map(PathBuf::from),
        );
    }
    #[cfg(not(target_os = "macos"))]
    {
        v.extend(
            [
                "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
                "/usr/share/fonts/google-noto-cjk/NotoSerifCJK-Regular.ttc",
                "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
                "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
            ]
            .map(PathBuf::from),
        );
    }
    v
}

fn font_outlines_basic_latin(font: &FontArc) -> bool {
    let scale = PxScale::from(18.0);
    let g: Glyph = font
        .glyph_id('a')
        .with_scale_and_position(scale, point(0.0, 0.0));
    font.outline_glyph(g).is_some()
}

fn font_outlines_cjk_probe(font: &FontArc) -> bool {
    // A representative CJK glyph to ensure we keep a usable fallback.
    let scale = PxScale::from(18.0);
    let g: Glyph = font
        .glyph_id('你')
        .with_scale_and_position(scale, point(0.0, 0.0));
    font.outline_glyph(g).is_some()
}

fn face_indices_for_data(face_count: u32) -> Vec<u32> {
    // Developer override (kept for diagnostics / CLI use).
    if let Ok(s) = std::env::var("RUST_SSH_FONT_FACE_INDEX") {
        if let Ok(idx) = s.trim().parse::<u32>() {
            if idx < face_count {
                return vec![idx];
            }
        }
    }
    // Stable default: lock to face 0 unless explicitly overridden.
    // This avoids face-to-face metric/hinting drift causing visual inconsistency.
    if face_count > 0 {
        vec![0]
    } else {
        Vec::new()
    }
}

pub(super) fn font_has_outline(font: &FontArc, ch: char, scale_px: f32) -> bool {
    let g: Glyph = font
        .glyph_id(ch)
        .with_scale_and_position(PxScale::from(scale_px), point(0.0, 0.0));
    font.outline_glyph(g).is_some()
}

pub(super) fn load_terminal_fonts(
    override_path: Option<String>,
    override_face_index: Option<u32>,
) -> Vec<LoadedFont> {
    let override_path = override_path.as_deref();

    // If user explicitly overrides the font path, respect it and keep only a single resident font
    // to avoid unexpected RSS spikes from large TTC collections.
    if override_path.is_some() {
        let mut out = Vec::new();
        for path in terminal_font_search_paths(override_path) {
            let data = match std::fs::read(&path) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let face_count = ttf_parser::fonts_in_collection(&data)
                .map(|n| n.max(1))
                .unwrap_or(1);
            let indices = if let Some(idx) = override_face_index {
                if idx < face_count {
                    vec![idx]
                } else {
                    face_indices_for_data(face_count)
                }
            } else {
                face_indices_for_data(face_count)
            };
            for &idx in &indices {
                if let Ok(f) = FontVec::try_from_vec_and_index(data.clone(), idx) {
                    let fa = FontArc::new(f);
                    let label = format!("{}#{}", path.display(), idx);
                    log::debug!(target: "term-diag", "GPU terminal font (override): {:?}", label);
                    // Metrics are filled by the caller (glyph_atlas) which knows the runtime scale.
                    out.push(LoadedFont { label, font: fa, ascent: 0.0, descent: 0.0, line_gap: 0.0, height: 0.0, mono_advance: 0.0, raster_scale_px: 0.0 });
                    return out;
                }
            }
        }
        return out;
    }

    // Default resident set: keep at most 2 fonts:
    // - one ASCII-friendly monospace
    // - one CJK-capable fallback
    let mut ascii: Option<LoadedFont> = None;
    let mut cjk: Option<LoadedFont> = None;

    for path in terminal_font_search_paths(override_path) {
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let face_count = ttf_parser::fonts_in_collection(&data)
            .map(|n| n.max(1))
            .unwrap_or(1);
        let indices = if let Some(idx) = override_face_index {
            if idx < face_count {
                vec![idx]
            } else {
                // Fall back to the developer override / all faces.
                face_indices_for_data(face_count)
            }
        } else {
            face_indices_for_data(face_count)
        };
        // Keep memory bounded: one selected face per font file.
        // Loading every face from large TTC files (e.g. Songti.ttc) duplicates
        // font bytes and can push app RSS to hundreds of MB at startup.
        for &idx in &indices {
            if let Ok(f) = FontVec::try_from_vec_and_index(data.clone(), idx) {
                let fa = FontArc::new(f);
                let label = format!("{}#{}", path.display(), idx);
                if ascii.is_none() && font_outlines_basic_latin(&fa) {
                    log::debug!(target: "term-diag", "GPU terminal font (ASCII resident): {:?}", label);
                    ascii = Some(LoadedFont { label: label.clone(), font: fa.clone(), ascent: 0.0, descent: 0.0, line_gap: 0.0, height: 0.0, mono_advance: 0.0, raster_scale_px: 0.0 });
                }
                if cjk.is_none() && font_outlines_cjk_probe(&fa) {
                    log::debug!(target: "term-diag", "GPU terminal font (CJK resident): {:?}", label);
                    cjk = Some(LoadedFont { label, font: fa, ascent: 0.0, descent: 0.0, line_gap: 0.0, height: 0.0, mono_advance: 0.0, raster_scale_px: 0.0 });
                }
                if ascii.is_some() && cjk.is_some() {
                    break;
                }
            }
        }
        if ascii.is_some() && cjk.is_some() {
            break;
        }
    }
    let mut out = Vec::new();
    if let Some(a) = ascii {
        out.push(a);
    }
    if let Some(c) = cjk {
        // Avoid duplicates if one font covers both probes.
        if out.first().is_none_or(|a| a.label != c.label) {
            out.push(c);
        }
    }
    out
}
