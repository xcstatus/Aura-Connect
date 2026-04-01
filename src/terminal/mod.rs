//! Terminal core interfaces.
//!
//! M0 boundary split:
//! - TerminalEngine: pure terminal state + I/O (write/resize/scroll/query)
//! - TerminalRenderer: prepare/paint, resource cache, incremental updates

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CursorPos {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDelta {
    Lines(i32),
    Pages(i32),
    ToTop,
    ToBottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScrollState {
    /// Total scrollback rows (including visible viewport).
    pub total_rows: u64,
    /// Current top row offset into the scrollback.
    pub offset_rows: u64,
    /// Visible viewport height in rows.
    pub viewport_rows: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TerminalSnapshot {
    pub size: TerminalSize,
    pub cursor: CursorPos,
    pub scroll: ScrollState,
}

/// Terminal state machine / engine boundary.
///
/// Responsibilities:
/// - Accept byte stream input (ANSI / VT sequences)
/// - Track grid state, cursor, selection, scrollback
/// - Expose query APIs for UI/renderer
pub trait TerminalEngine {
    fn write(&mut self, bytes: &[u8]);
    fn resize(&mut self, size: TerminalSize);
    fn scroll(&mut self, delta: ScrollDelta);
    fn query(&self) -> TerminalSnapshot;
}

// NOTE:
// Active Iced path currently uses `terminal_core::TerminalController` as the engine.
// `TerminalController::query_snapshot()` now provides a compact engine snapshot
// (size/cursor/scroll/selection) that can be adapted to this trait boundary as
// we continue renderer decoupling.

/// Renderer boundary.
///
/// Responsibilities:
/// - Prepare GPU/CPU resources based on engine snapshot or diff
/// - Cache resources (glyph atlas, textures, pipelines)
/// - Paint using the app's render backend
pub trait TerminalRenderer {
    type PreparedFrame;

    fn prepare(&mut self, snapshot: &TerminalSnapshot) -> Self::PreparedFrame;
    fn paint(&mut self, prepared: &Self::PreparedFrame);

    fn clear_cache(&mut self);
}

pub mod backend;
pub(crate) mod diagnostics;
pub mod display_test;
pub mod session_bridge;
mod glyph_atlas;
pub mod gpu_renderer;

#[cfg(feature = "ghostty-vt")]
pub mod vt_widget;

