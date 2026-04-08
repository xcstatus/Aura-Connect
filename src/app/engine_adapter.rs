use crate::terminal::controller::{EngineSnapshot, TerminalController, TerminalKey, TerminalModifiers};
use crate::terminal::ScrollState;

use super::state::IcedState;

/// Thin adapter so Iced views consume engine snapshot data instead of
/// reaching into `TerminalController` internals directly.
pub(crate) struct EngineAdapter<'a> {
    _phantom: std::marker::PhantomData<&'a IcedState>,
    snap: EngineSnapshot,
}

impl<'a> EngineAdapter<'a> {
    pub(crate) fn active(state: &'a IcedState) -> Self {
        let snap = state.active_terminal().query_snapshot();
        Self {
            _phantom: std::marker::PhantomData,
            snap,
        }
    }

    #[inline]
    pub(crate) fn grid_size(&self) -> (u16, u16) {
        (self.snap.cols, self.snap.rows)
    }

    #[inline]
    pub(crate) fn selection(&self) -> Option<((u16, u16), (u16, u16))> {
        self.snap.selection
    }

    #[inline]
    pub(crate) fn scroll(&self) -> ScrollState {
        self.snap.scroll
    }

    #[inline]
    pub(crate) fn is_in_scrollback(&self) -> bool {
        let s = self.snap.scroll;
        let total = s.total_rows;
        let len = s.viewport_rows.min(total);
        let bottom_off = total.saturating_sub(len);
        s.offset_rows < bottom_off
    }
}

pub(crate) struct EngineAdapterMut<'a> {
    terminal: &'a mut TerminalController,
}

impl<'a> EngineAdapterMut<'a> {
    pub(crate) fn new(terminal: &'a mut TerminalController) -> Self {
        Self { terminal }
    }

    #[inline]
    pub(crate) fn grid_size(&self) -> (u16, u16) {
        self.terminal.grid_size()
    }

    #[inline]
    pub(crate) fn selection_range(&self) -> Option<((u16, u16), (u16, u16))> {
        self.terminal.selection_range()
    }

    #[inline]
    pub(crate) fn selection_begin(&mut self, at: (u16, u16)) {
        self.terminal.selection_mut().begin(at);
    }

    #[inline]
    pub(crate) fn selection_update(&mut self, at: (u16, u16)) {
        self.terminal.selection_mut().update(at);
    }

    #[inline]
    pub(crate) fn selection_end(&mut self, keep_highlight: bool) {
        self.terminal.selection_mut().end(keep_highlight);
    }

    #[inline]
    pub(crate) fn selection_end_drag_keep(&mut self) {
        self.terminal.selection_mut().end_drag_keep_selection();
    }

    #[inline]
    pub(crate) fn visible_text_for_copy(&self) -> String {
        self.terminal.visible_text_for_copy()
    }

    #[inline]
    pub(crate) fn selection_text_for_copy(&mut self, a: (u16, u16), b: (u16, u16)) -> String {
        self.terminal.selection_text_for_copy(a, b)
    }

    #[inline]
    pub(crate) fn encode_focus_event(&mut self, focused: bool) -> Vec<u8> {
        self.terminal.encode_focus_event(focused)
    }

    #[inline]
    pub(crate) fn encode_paste(&mut self, text: &str) -> Vec<u8> {
        self.terminal.encode_paste(text)
    }

    #[inline]
    pub(crate) fn encode_keypress_from_physical(
        &mut self,
        code: iced::keyboard::key::Code,
        mods: TerminalModifiers,
        text: Option<&str>,
    ) -> Option<Vec<u8>> {
        self.terminal
            .encode_keypress_from_physical(code, mods, text)
    }

    #[inline]
    pub(crate) fn encode_unidentified_text_keypress(&mut self, text: &str) -> Option<Vec<u8>> {
        self.terminal.encode_unidentified_text_keypress(text)
    }

    #[inline]
    pub(crate) fn encode_text_raw_fallback_whitelisted(&mut self, text: &str) -> Option<Vec<u8>> {
        self.terminal.encode_text_raw_fallback_whitelisted(text)
    }

    #[inline]
    pub(crate) fn encode_text_raw_committed_utf8(&self, text: &str) -> Option<Vec<u8>> {
        self.terminal.encode_text_raw_committed_utf8(text)
    }

    #[inline]
    pub(crate) fn on_named_key(
        &mut self,
        session: &mut dyn crate::backend::ssh_session::AsyncSession,
        key: TerminalKey,
        mods: TerminalModifiers,
    ) -> anyhow::Result<()> {
        self.terminal.on_key(session, key, mods)
    }

    #[inline]
    pub(crate) fn scroll_viewport_delta_rows(&mut self, delta_rows: isize) {
        self.terminal.scroll_viewport_delta_rows(delta_rows);
    }
}
