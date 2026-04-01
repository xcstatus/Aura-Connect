pub(super) fn clamp_slot_offset(offset: i32, glyph_extent: f32, slot_px: u32) -> i32 {
    let glyph_px = glyph_extent.ceil().max(1.0) as i32;
    let slot_px = slot_px as i32;
    // Keep glyph fully inside the slot to avoid partial draws that look like
    // "garbled / overlapped / abstract" glyphs when only a slice of the bitmap lands in-bounds.
    let min_off = 0;
    let max_off = (slot_px - glyph_px).max(0);
    offset.clamp(min_off, max_off)
}

pub(super) fn fit_scale_for_slot(
    base_scale_px: f32,
    bounds_w: f32,
    bounds_h: f32,
    slot_px: f32,
    max_fill_ratio: f32,
) -> f32 {
    if base_scale_px <= 0.0 || bounds_w <= 0.0 || bounds_h <= 0.0 || slot_px <= 0.0 {
        return base_scale_px.max(1.0);
    }
    let target_px = (slot_px * max_fill_ratio).max(1.0);
    let max_dim = bounds_w.max(bounds_h);
    if max_dim <= target_px {
        return base_scale_px;
    }
    (base_scale_px * (target_px / max_dim)).max(1.0)
}
