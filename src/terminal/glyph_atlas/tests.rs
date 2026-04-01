use super::GlyphAtlasState;
use crate::terminal::glyph_atlas::helpers::{clamp_slot_offset, fit_scale_for_slot};

#[test]
fn fit_scale_reduces_when_bounds_too_large() {
    let scaled = fit_scale_for_slot(28.0, 18.0, 44.0, 32.0, 0.92);
    assert!(scaled < 28.0);
    assert!(scaled > 0.0);
}

#[test]
fn fit_scale_keeps_when_already_within_slot() {
    let scaled = fit_scale_for_slot(22.0, 13.0, 20.0, 32.0, 0.92);
    assert!((scaled - 22.0).abs() < f32::EPSILON);
}

#[test]
fn clamp_slot_offset_keeps_in_bounds() {
    assert_eq!(clamp_slot_offset(100, 10.0, 32), 31);
    assert_eq!(clamp_slot_offset(-100, 10.0, 32), -9);
}

#[test]
fn slot_for_char_caches_failed_char_when_no_font() {
    let mut atlas = GlyphAtlasState::new();
    atlas.font_load_attempted = true;
    atlas.fonts.clear();

    let slot = atlas.slot_for_char('A' as u32);
    assert_eq!(slot, 0);
    assert!(atlas.failed_chars.contains(&(u32::from('A'))));
}

#[test]
fn failed_char_cache_hit_does_not_advance_slots() {
    let mut atlas = GlyphAtlasState::new();
    atlas.font_load_attempted = true;
    atlas.failed_chars.insert(u32::from('Z'));
    let before = atlas.next_slot;
    let _ = atlas.slot_for_char(u32::from('Z'));
    assert_eq!(atlas.next_slot, before);
    assert_eq!(atlas.failed_cache_hits, 1);
}

#[test]
fn lru_allocate_reuses_oldest_slot_when_full() {
    let mut atlas = GlyphAtlasState::new();
    // Force "fully allocated across all pages" so allocator must pick an LRU victim.
    atlas.pages_used = super::MAX_ATLAS_PAGES;
    atlas.next_slot = atlas.slots_per_page();
    atlas.slot_to_char.insert(1, u32::from('A'));
    atlas.slot_to_char.insert(2, u32::from('B'));
    atlas.char_to_slot.insert(u32::from('A'), 1);
    atlas.char_to_slot.insert(u32::from('B'), 2);
    atlas.slot_last_used_tick.insert(1, 10);
    atlas.slot_last_used_tick.insert(2, 20);

    let slot = atlas.allocate_slot_for_char(u32::from('C'));
    assert_eq!(slot, Some(1));
    assert!(!atlas.char_to_slot.contains_key(&u32::from('A')));
}

#[test]
fn bind_slot_to_char_replaces_old_mapping() {
    let mut atlas = GlyphAtlasState::new();
    atlas.bind_slot_to_char(5, u32::from('X'));
    atlas.bind_slot_to_char(5, u32::from('Y'));
    assert_eq!(atlas.char_to_slot.get(&u32::from('Y')).copied(), Some(5));
    assert!(!atlas.char_to_slot.contains_key(&u32::from('X')));
}

#[test]
fn allocate_slot_grows_to_new_page_before_lru() {
    let mut atlas = GlyphAtlasState::new();
    // Simulate "page full": next_slot == slots_per_page triggers a page growth (if available).
    let spp = atlas.slots_per_page();
    atlas.pages_used = 1;
    atlas.next_slot = spp;

    let slot = atlas.allocate_slot_for_char(u32::from('C')).expect("slot allocated");
    assert_eq!(atlas.pages_used, 2, "should grow to a second page");

    // New allocation must land on page=1, in_page=1.
    let decoded = atlas.decode_slot(slot).expect("decoded slot");
    assert_eq!(decoded.0, 1);
    assert_eq!(decoded.1, 1);
}
