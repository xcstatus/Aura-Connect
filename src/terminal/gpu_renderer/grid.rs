use super::*;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

impl CellGrid {
    pub(super) fn ensure_size(&mut self, cols: usize, rows: usize) {
        if self.cols == cols && self.rows == rows && self.cells.len() == cols * rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        self.cells.clear();
        self.cells.resize(cols * rows, Cell::default());
        self.seen_complex_grapheme = false;
    }

    pub(super) fn update_row(&mut self, cols: usize, y: usize, row: &VtStyledRow) {
        let base = y * cols;
        if base >= self.cells.len() {
            return;
        }
        // Fill row with spaces first; then overwrite by runs.
        for x in 0..cols {
            let idx = base + x;
            if idx < self.cells.len() {
                self.cells[idx] = Cell {
                    ch: ' ' as u32,
                    fg: [230, 230, 230, 255],
                    bg: [10, 10, 15, 255],
                    flags: 0,
                };
            }
        }

        let mut x = 0usize;
        for run in &row.runs {
            if x >= cols {
                break;
            }
            if run.text.is_empty() || run.cols == 0 {
                continue;
            }
            let fg = [run.fg.r, run.fg.g, run.fg.b, 255];
            let bg = if run.has_bg {
                [run.bg.r, run.bg.g, run.bg.b, 255]
            } else {
                // Keep default background; sparse BG pipeline will skip these cells.
                [10, 10, 15, 255]
            };
            let mut flags = 0u8;
            if run.has_bg {
                flags |= 1 << 0;
            }
            if run.bold {
                flags |= 1 << 1;
            }
            if run.underline {
                flags |= 1 << 2;
            }
            if run.dim {
                flags |= 1 << 3;
            }
            if run.strikethrough {
                flags |= 1 << 4;
            }

            let mut run_cols_left = run.cols as usize;
            let mut graphemes = run.text.graphemes(true);

            while run_cols_left > 0 && x < cols {
                let (cell_ch, cell_w) = if let Some(g) = graphemes.next() {
                    // If a single cell contains multiple codepoints (e.g. e + combining mark,
                    // emoji ZWJ sequences), our current GPU atlas cannot rasterize it as one glyph.
                    if g.chars().nth(1).is_some() {
                        self.seen_complex_grapheme = true;
                    }
                    let mut w = UnicodeWidthStr::width(g).max(1);
                    if w > run_cols_left {
                        w = run_cols_left;
                    }
                    if w > cols - x {
                        w = cols - x;
                    }
                    (g.chars().next().unwrap_or(' '), w.max(1))
                } else {
                    // If run.cols is larger than grapheme count (e.g. wide-char trail),
                    // keep filling style background so column accounting stays aligned.
                    (' ', 1)
                };

                let idx = base + x;
                if idx < self.cells.len() {
                    self.cells[idx] = Cell {
                        ch: cell_ch as u32,
                        fg,
                        bg,
                        flags,
                    };
                }

                // Fill continuation cells for wide graphemes as styled spaces.
                for i in 1..cell_w {
                    let idx = base + x + i;
                    if idx < self.cells.len() {
                        self.cells[idx] = Cell {
                            ch: ' ' as u32,
                            fg,
                            bg,
                            flags,
                        };
                    }
                }

                x += cell_w;
                run_cols_left = run_cols_left.saturating_sub(cell_w);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::ghostty_vt::{VtStyledRow, VtStyledRun, ffi};

    fn mk_run(text: &str, cols: u16) -> VtStyledRun {
        VtStyledRun {
            text: text.to_string(),
            cols,
            fg: ffi::GhosttyColorRgb { r: 1, g: 2, b: 3 },
            bg: ffi::GhosttyColorRgb { r: 4, g: 5, b: 6 },
            has_bg: true,
            bold: false,
            underline: false,
            dim: false,
            strikethrough: false,
        }
    }

    #[test]
    fn wide_grapheme_consumes_two_cells() {
        let mut grid = CellGrid::default();
        grid.ensure_size(4, 1);
        let row = VtStyledRow {
            runs: vec![mk_run("你", 2)],
        };
        grid.update_row(4, 0, &row);
        assert_eq!(grid.cells[0].ch, '你' as u32);
        assert_eq!(grid.cells[1].ch, ' ' as u32);
    }

    #[test]
    fn combining_grapheme_stays_single_cell() {
        let mut grid = CellGrid::default();
        grid.ensure_size(4, 1);
        let row = VtStyledRow {
            runs: vec![mk_run("e\u{0301}", 1)],
        };
        grid.update_row(4, 0, &row);
        assert_eq!(grid.cells[0].ch, 'e' as u32);
        assert_eq!(grid.cells[1].ch, ' ' as u32);
    }
}
