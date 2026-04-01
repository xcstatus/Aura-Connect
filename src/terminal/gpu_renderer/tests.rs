use super::*;
use crate::backend::ghostty_vt::{VtStyledRow, VtStyledRun};

#[test]
fn build_glyph_instances_skips_plain_space() {
    let mut r = TerminalGpuRenderer::new();
    let row = VtStyledRow {
        runs: vec![VtStyledRun {
            text: " ".to_string(),
            cols: 1,
            ..VtStyledRun::default()
        }],
    };
    r.update_cells_from_rows(1, 1, &[row], &[]);
    r.build_glyph_instances(1, 1);
    let d = r.glyph_build_diag();
    assert_eq!(d.total_cells, 1);
    assert_eq!(d.non_space_cells, 0);
    assert_eq!(d.glyph_instances, 0);
}

#[test]
fn build_glyph_instances_keeps_underline_on_space() {
    let mut r = TerminalGpuRenderer::new();
    let row = VtStyledRow {
        runs: vec![VtStyledRun {
            text: " ".to_string(),
            cols: 1,
            underline: true,
            ..VtStyledRun::default()
        }],
    };
    r.update_cells_from_rows(1, 1, &[row], &[]);
    r.build_glyph_instances(1, 1);
    let d = r.glyph_build_diag();
    assert_eq!(d.glyph_instances, 1);
    assert_eq!(d.underline_space_quads, 1);
}
