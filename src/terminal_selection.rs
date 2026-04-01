#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerminalSelection {
    pub range: Option<((u16, u16), (u16, u16))>,
    pub pending_anchor: Option<(u16, u16)>,
    pub selecting: bool,
}

impl TerminalSelection {
    pub fn clear(&mut self) {
        self.range = None;
        self.pending_anchor = None;
        self.selecting = false;
    }

    pub fn begin(&mut self, anchor: (u16, u16)) {
        self.pending_anchor = Some(anchor);
        self.selecting = false;
    }

    /// Update with the current hover cell; starts selection only after moving off anchor.
    pub fn update(&mut self, cell: (u16, u16)) {
        if !self.selecting {
            if let Some(anchor) = self.pending_anchor {
                if anchor != cell {
                    self.range = Some((anchor, cell));
                    self.selecting = true;
                }
            }
            return;
        }
        if let Some((a, _)) = self.range {
            self.range = Some((a, cell));
        }
    }

    /// Finish current selection interaction.
    /// If user only clicked without drag, clears any existing selection.
    pub fn end(&mut self, keep_highlight: bool) {
        if self.selecting {
            self.selecting = false;
            self.pending_anchor = None;
            if !keep_highlight {
                self.range = None;
            }
            return;
        }
        if self.pending_anchor.is_some() {
            self.pending_anchor = None;
            self.range = None;
        }
    }

    pub fn end_drag_keep_selection(&mut self) {
        self.selecting = false;
        self.pending_anchor = None;
    }

    pub fn is_selecting(&self) -> bool {
        self.selecting
    }
}

