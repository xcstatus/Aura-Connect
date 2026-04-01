//! Styled terminal viewport: libghostty [`VtStyledRow`] → Iced `rich_text` / `span`.

use crate::backend::ghostty_vt::ffi;
use crate::backend::ghostty_vt::{CursorState, VtStyledCell, VtStyledRow, VtStyledRun};
use crate::settings::{TerminalPlainTextUpdate, TerminalRenderMode};
use crate::terminal_core::{ScrollState, StyledFragment};
use crate::theme::layout::{SCROLLBAR_WIDTH, TERMINAL_SCROLLBAR_OVERLAY_PAD_RIGHT};
use iced::font::{self, Font};
use iced::widget::{column, rich_text, scrollable, span, text, Row};
use iced::widget::text::LineHeight;
use iced::widget::text::Span;
use iced::widget::{container, row, Space, Stack};
use iced::widget::scrollable::{Direction as ScrollDirection, Scrollbar};
use iced::alignment::{Horizontal, Vertical};
use iced::{Color, Element, Length, Padding, Theme};

use super::message::Message;
use super::state::IcedState;
use super::terminal_viewport;
use super::engine_adapter::EngineAdapter;

/// Iced font for terminal body (pick-list names map to `Font::with_name`; unknown → monospace).
/// Lock each VT row to exactly `line_px` tall so Iced layout matches PTY `ch` (see hit-test in
/// `terminal_viewport`). Otherwise `rich_text`/`Paragraph::min_bounds()` can be ~2× the line height,
/// which makes consecutive visible lines ~`2*ch` apart while `floor(local_y/ch)` still uses `ch`.
#[inline]
fn fixed_terminal_row<'a>(line_px: iced::Pixels, inner: Element<'a, Message>) -> Element<'a, Message> {
    container(inner)
        .height(Length::Fixed(line_px.0))
        .width(Length::Fill)
        .align_y(Vertical::Top)
        .into()
}

/// Selection highlight: full cell fill (span-level bg only wraps glyphs and leaves gaps between cells).
const SELECTION_CELL_FILL: Color = Color::from_rgba(60.0 / 255.0, 120.0 / 255.0, 200.0 / 255.0, 0.45);

/// Fixed-size terminal cell; `fill` paints the **entire** `cell_w × line_h` (used for selection).
#[inline]
fn fixed_width_cell<'a>(
    inner: Element<'a, Message>,
    cell_w: f32,
    line_h: f32,
    fill: Option<Color>,
) -> Element<'a, Message> {
    let mut c = container(inner)
        .width(Length::Fixed(cell_w))
        .height(Length::Fixed(line_h))
        .align_x(Horizontal::Left)
        .align_y(Vertical::Top)
        .padding(Padding::ZERO);
    if let Some(col) = fill {
        c = c.style(move |_| container::Style::default().background(iced::Background::Color(col)));
    }
    c.into()
}

#[inline]
fn rich_text_single<'a>(
    s: Span<'a, (), Font>,
    base_font: Font,
    font_px: f32,
    line_px: iced::Pixels,
) -> Element<'a, Message> {
    rich_text(vec![s])
        .font(base_font)
        .size(font_px)
        .line_height(LineHeight::Absolute(line_px))
        .width(Length::Shrink)
        .into()
}

/// One cell from a styled run: uses lightweight [`text`] when underline/strikethrough are off.
#[inline]
fn cell_from_run<'a>(
    run: &VtStyledRun,
    piece: &'a str,
    font_px: f32,
    line_px: iced::Pixels,
    base_font: Font,
    cell_w: f32,
    invisible: bool,
) -> Element<'a, Message> {
    let line_h = line_px.0;
    let piece = if piece.is_empty() { " " } else { piece };
    let mut fg = ghost_rgb(&run.fg);
    if run.dim {
        fg = fg.scale_alpha(0.72);
    }
    if invisible {
        fg = fg.scale_alpha(0.0);
    }

    let inner: Element<'a, Message> = if run.underline || run.strikethrough {
        let s = span_from_fragment(run, piece, font_px, base_font, invisible);
        rich_text_single(s, base_font, font_px, line_px)
    } else {
        text(piece)
            .size(font_px)
            .font(run_font(run, base_font))
            .line_height(LineHeight::Absolute(line_px))
            .color(fg)
            .into()
    };

    let fill = if run.has_bg && !run.underline && !run.strikethrough {
        Some(ghost_rgb(&run.bg))
    } else {
        None
    };
    fixed_width_cell(inner, cell_w, line_h, fill)
}

#[inline]
fn terminal_grid_cell_from_fragment<'a>(
    frag: &'a StyledFragment,
    font_px: f32,
    line_px: iced::Pixels,
    base_font: Font,
    cell_w: f32,
) -> Element<'a, Message> {
    let line_h = line_px.0;
    let piece = if frag.text.is_empty() { " " } else { frag.text.as_str() };
    let mut fg = ghost_rgb(&frag.fg);
    if frag.dim {
        fg = fg.scale_alpha(0.72);
    }
    if frag.invisible {
        fg = fg.scale_alpha(0.0);
    }

    let inner: Element<'a, Message> = if frag.underline || frag.strikethrough {
        let s = span_from_fragment_cached(frag, font_px, base_font);
        rich_text_single(s, base_font, font_px, line_px)
    } else {
        text(piece)
            .size(font_px)
            .font(frag_font(frag, base_font))
            .line_height(LineHeight::Absolute(line_px))
            .color(fg)
            .into()
    };

    let fill = if frag.has_bg && !frag.underline && !frag.strikethrough {
        Some(ghost_rgb(&frag.bg))
    } else {
        None
    };
    fixed_width_cell(inner, cell_w, line_h, fill)
}

/// Selected cell: white glyph, **no** span background — full cell uses [`SELECTION_CELL_FILL`].
#[inline]
fn cell_selected<'a>(
    run: &VtStyledRun,
    piece: &'a str,
    font_px: f32,
    line_px: iced::Pixels,
    base_font: Font,
    cell_w: f32,
    invisible: bool,
) -> Element<'a, Message> {
    let line_h = line_px.0;
    let piece = if piece.is_empty() { " " } else { piece };
    let fg = if invisible {
        Color::from_rgba(0.0, 0.0, 0.0, 0.0)
    } else {
        Color::WHITE
    };
    let inner: Element<'a, Message> = if run.underline || run.strikethrough {
        let s = span(piece)
            .color(fg)
            .size(font_px)
            .font(run_font(run, base_font))
            .underline(run.underline)
            .strikethrough(run.strikethrough);
        rich_text_single(s, base_font, font_px, line_px)
    } else {
        text(piece)
            .size(font_px)
            .font(run_font(run, base_font))
            .line_height(LineHeight::Absolute(line_px))
            .color(fg)
            .into()
    };
    fixed_width_cell(inner, cell_w, line_h, Some(SELECTION_CELL_FILL))
}

fn iced_terminal_font(t: &crate::settings::TerminalSettings) -> Font {
    if !t.apply_terminal_metrics {
        return Font::MONOSPACE;
    }
    match t.font_family.trim() {
        "JetBrains Mono" => Font::with_name("JetBrains Mono"),
        "SF Mono" => Font::with_name("SF Mono"),
        "Cascadia Code" => Font::with_name("Cascadia Code"),
        _ => Font::MONOSPACE,
    }
}

pub(crate) fn terminal_main_area<'a>(state: &'a IcedState) -> Element<'a, Message> {
    let engine = EngineAdapter::active(state);
    let spec = terminal_viewport::terminal_viewport_spec_for_settings(&state.model.settings.terminal);
    let font_px = spec.term_font_px;
    let line_px = iced::Pixels(spec.term_cell_h().max(1.0));
    let base_font = iced_terminal_font(&state.model.settings.terminal);
    let (cols, _) = engine.grid_size();
    let (_, cell_w_hit) =
        terminal_viewport::terminal_scroll_cell_geometry(state.window_size, &spec, cols);
    let selection = engine.selection();
    let scroll = engine.scroll();
    let in_scrollback = engine.is_in_scrollback();
    match state.model.settings.terminal.terminal_render_mode {
        TerminalRenderMode::Plain => terminal_with_scrollbar(
            plain_terminal(state, font_px, line_px, base_font),
            scroll,
            in_scrollback,
        ),
        TerminalRenderMode::Styled => terminal_with_scrollbar(
            styled_terminal(
                state,
                selection,
                font_px,
                line_px,
                base_font,
                cell_w_hit,
            ),
            scroll,
            in_scrollback,
        ),
    }
}

fn terminal_with_scrollbar<'a>(
    body: Element<'a, Message>,
    scroll: ScrollState,
    in_scrollback: bool,
) -> Element<'a, Message> {
    let scrollbar = terminal_scrollbar_overlay(scroll);
    let badge = if in_scrollback {
        terminal_scrollback_badge().into()
    } else {
        Space::new().into()
    };
    Stack::with_children([body, scrollbar, badge])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn terminal_scrollbar_overlay<'a>(scroll: ScrollState) -> Element<'a, Message> {
    let total = scroll.total_rows.max(1);
    let viewport = scroll.viewport_rows.max(1).min(total);
    let max_off = total.saturating_sub(viewport).max(1);
    let off = scroll.offset_rows.min(max_off);
    let thumb_ratio = (viewport as f32 / total as f32).clamp(0.02, 1.0);
    let offset_ratio = (off as f32 / max_off as f32).clamp(0.0, 1.0);

    // Use fixed pixel sizes; this is purely a visual affordance.
    let track_h = 220.0_f32;
    let thumb_h = (track_h * thumb_ratio).max(14.0);
    let travel = (track_h - thumb_h).max(0.0);
    let top_pad = travel * offset_ratio;

    let track = container(
        column![
            Space::new().height(Length::Fixed(top_pad)),
            container(Space::new().width(Length::Fill).height(Length::Fixed(thumb_h)))
                .style(|theme: &Theme| {
                    let c = theme.extended_palette().background.base.text.scale_alpha(0.25);
                    container::Style::default().background(iced::Background::Color(c))
                })
                .width(Length::Fill),
            Space::new().height(Length::Fixed((track_h - top_pad - thumb_h).max(0.0))),
        ]
        .spacing(0),
    )
    .width(Length::Fixed(SCROLLBAR_WIDTH))
    .height(Length::Fixed(track_h))
    .style(|theme: &Theme| {
        let c = theme.extended_palette().background.base.text.scale_alpha(0.08);
        container::Style::default().background(iced::Background::Color(c))
    })
    .padding(0);

    container(row![Space::new().width(Length::Fill), track].spacing(0))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding {
            top: 10.0,
            right: TERMINAL_SCROLLBAR_OVERLAY_PAD_RIGHT,
            bottom: 10.0,
            left: 0.0,
        })
        .align_y(iced::alignment::Vertical::Top)
        .into()
}

fn terminal_scrollback_badge<'a>() -> Element<'a, Message> {
    container(iced::widget::text("回滚").size(11))
        .padding([4, 8])
        .style(|theme: &Theme| {
            let bg = theme.extended_palette().background.strong.color.scale_alpha(0.92);
            container::Style::default().background(iced::Background::Color(bg))
        })
        .into()
}

fn plain_terminal<'a>(
    state: &'a IcedState,
    font_px: f32,
    line_px: iced::Pixels,
    base_font: Font,
) -> Element<'a, Message> {
    match state.model.settings.terminal.plain_text_update {
        TerminalPlainTextUpdate::Incremental => scrollable(
            iced::widget::text(state.active_terminal().plain_text_for_view())
                .size(font_px)
                .font(base_font)
                .line_height(LineHeight::Absolute(line_px)),
        )
        .direction(ScrollDirection::Vertical(Scrollbar::hidden()))
        .height(Length::Fill)
        .into(),
        TerminalPlainTextUpdate::Full => {
            let terminal_text = state.active_terminal().lines().join("\n");
            scrollable(
                iced::widget::text(terminal_text)
                    .size(font_px)
                    .font(base_font)
                    .line_height(LineHeight::Absolute(line_px)),
            )
            .direction(ScrollDirection::Vertical(Scrollbar::hidden()))
            .height(Length::Fill)
            .into()
        }
    }
}

fn styled_terminal<'a>(
    state: &'a IcedState,
    selection: Option<((u16, u16), (u16, u16))>,
    font_px: f32,
    line_px: iced::Pixels,
    base_font: Font,
    cell_w: f32,
) -> Element<'a, Message> {
    let rows = state.active_terminal().styled_rows();
    let cursor = state.active_terminal().cursor_snapshot();
    let blink_on = cursor_blink_on(state, cursor);
    let mut col = column![].spacing(0).width(Length::Fill);
    for (y, row) in rows.iter().enumerate() {
        if selection.is_some() {
            col = col.push(fixed_terminal_row(
                line_px,
                styled_row_line(
                    row,
                    y,
                    cursor,
                    blink_on,
                    selection,
                    font_px,
                    line_px,
                    base_font,
                    cell_w,
                ),
            ));
            continue;
        }

        let cy = y as u16;
        let cursor_on_row = blink_on && cursor.is_some_and(|c| c.visible && c.has_pos && c.y == cy);
        if cursor_on_row {
            // Cursor overlay needs cell-precise logic: keep the existing path for this row only.
            col = col.push(fixed_terminal_row(
                line_px,
                styled_row_line(
                    row,
                    y,
                    cursor,
                    blink_on,
                    None,
                    font_px,
                    line_px,
                    base_font,
                    cell_w,
                ),
            ));
            continue;
        }

        // Fast path: use core-maintained fragments (updated only on dirty rows).
        let frags = state.active_terminal().styled_row_fragments(y);
        col = col.push(fixed_terminal_row(
            line_px,
            styled_row_line_from_fragments(frags, font_px, line_px, base_font, cell_w),
        ));
    }
    scrollable(col)
        .direction(ScrollDirection::Vertical(Scrollbar::hidden()))
        .height(Length::Fill)
        .into()
}

fn cursor_blink_on(state: &IcedState, cursor: Option<&CursorState>) -> bool {
    let Some(c) = cursor else {
        return true;
    };
    if !c.blinking {
        return true;
    }
    (state.tick_count / 32) % 2 == 0
}

fn ghost_rgb(c: &ffi::GhosttyColorRgb) -> Color {
    Color::from_rgb8(c.r, c.g, c.b)
}

fn run_font(run: &VtStyledRun, base: Font) -> Font {
    if run.bold {
        Font {
            weight: font::Weight::Bold,
            ..base
        }
    } else {
        base
    }
}

fn frag_font(f: &StyledFragment, base: Font) -> Font {
    if f.bold {
        Font {
            weight: font::Weight::Bold,
            ..base
        }
    } else {
        base
    }
}

fn span_from_fragment_cached<'a>(
    frag: &StyledFragment,
    font_px: f32,
    base_font: Font,
) -> Span<'a, (), Font> {
    let piece = if frag.text.is_empty() { " ".to_string() } else { frag.text.clone() };
    let mut fg = ghost_rgb(&frag.fg);
    if frag.dim {
        fg = fg.scale_alpha(0.72);
    }
    if frag.invisible {
        fg = fg.scale_alpha(0.0);
    }
    let mut s = span(piece)
        .color(fg)
        .size(font_px)
        .font(frag_font(frag, base_font))
        .underline(frag.underline)
        .strikethrough(frag.strikethrough);
    if frag.has_bg {
        s = s.background(iced::Background::Color(ghost_rgb(&frag.bg)));
    }
    s
}

fn styled_row_line_from_fragments<'a>(
    frags: &'a [StyledFragment],
    font_px: f32,
    line_px: iced::Pixels,
    base_font: Font,
    cell_w: f32,
) -> Element<'a, Message> {
    if frags.is_empty() {
        let el: Element<'a, Message> = text(" ")
            .size(font_px)
            .font(base_font)
            .line_height(LineHeight::Absolute(line_px))
            .into();
        return Row::with_children([fixed_width_cell(el, cell_w, line_px.0, None)])
            .spacing(0.0)
            .into();
    }
    let mut children: Vec<Element<'a, Message>> = Vec::with_capacity(frags.len());
    for f in frags {
        children.push(terminal_grid_cell_from_fragment(f, font_px, line_px, base_font, cell_w));
    }
    Row::with_children(children).spacing(0.0).clip(true).into()
}

fn span_from_fragment<'a>(
    run: &VtStyledRun,
    piece: impl Into<String>,
    font_px: f32,
    base_font: Font,
    invisible: bool,
) -> Span<'a, (), Font> {
    let piece: String = piece.into();
    let piece = if piece.is_empty() { " ".to_string() } else { piece };
    let mut fg = ghost_rgb(&run.fg);
    if run.dim {
        fg = fg.scale_alpha(0.72);
    }
    if invisible {
        fg = fg.scale_alpha(0.0);
    }
    let mut s = span(piece)
        .color(fg)
        .size(font_px)
        .font(run_font(run, base_font))
        .underline(run.underline)
        .strikethrough(run.strikethrough);
    if run.has_bg {
        s = s.background(iced::Background::Color(ghost_rgb(&run.bg)));
    }
    s
}

fn styled_row_line<'a>(
    row: &'a VtStyledRow,
    row_ix: usize,
    cursor: Option<&'a CursorState>,
    blink_on: bool,
    selection: Option<((u16, u16), (u16, u16))>,
    font_px: f32,
    line_px: iced::Pixels,
    base_font: Font,
    cell_w: f32,
) -> Element<'a, Message> {
    let cy = row_ix as u16;
    let apply_cursor =
        blink_on && cursor.is_some_and(|c| c.visible && c.has_pos && c.y == cy);
    let cx = cursor.map(|c| c.x).unwrap_or(0);

    if row.runs.is_empty() {
        let inner: Element<'a, Message> = match (apply_cursor, cursor) {
            (true, Some(cur)) => {
                let placeholder = VtStyledRun::default();
                rich_text_single(
                    cursor_cell_span(" ", cur, &placeholder, font_px, base_font),
                    base_font,
                    font_px,
                    line_px,
                )
            }
            _ => text(" ")
                .size(font_px)
                .font(base_font)
                .line_height(LineHeight::Absolute(line_px))
                .into(),
        };
        return Row::with_children([fixed_width_cell(inner, cell_w, line_px.0, None)])
            .spacing(0.0)
            .clip(true)
            .into();
    }

    let cap: usize = row.runs.iter().map(|r| r.cells.len()).sum();
    let mut children: Vec<Element<'a, Message>> = Vec::with_capacity(cap.max(1));

    let mut col: u16 = 0;
    for run in &row.runs {
        let run_start = col;
        let run_cols = run.cells.len().min(u16::MAX as usize) as u16;
        let run_end = col.saturating_add(run_cols);

        if selection.is_some() {
            let mut x = run_start;
            for cell in &run.cells {
                let selected = cell_in_selection(selection, x, cy);
                let invisible = cell.continuation;
                let t = if cell.text.is_empty() { " " } else { cell.text.as_str() };
                let el = if selected {
                    cell_selected(run, t, font_px, line_px, base_font, cell_w, invisible)
                } else {
                    cell_from_run(run, t, font_px, line_px, base_font, cell_w, invisible)
                };
                children.push(el);
                x = x.saturating_add(1);
            }
            col = run_end;
            continue;
        }

        if !apply_cursor || cx < run_start || cx >= run_end {
            for cell in &run.cells {
                let piece = if cell.text.is_empty() {
                    " "
                } else {
                    cell.text.as_str()
                };
                children.push(cell_from_run(
                    run,
                    piece,
                    font_px,
                    line_px,
                    base_font,
                    cell_w,
                    cell.continuation,
                ));
            }
            col = run_end;
            continue;
        }

        if let Some(cur) = cursor {
            let at = cx.saturating_sub(run_start);
            let (before, mid, after) = split_run_three_cells(&run.cells, at);
            for cell in before {
                let piece = if cell.text.is_empty() {
                    " "
                } else {
                    cell.text.as_str()
                };
                let sp = span_from_fragment(run, piece, font_px, base_font, cell.continuation);
                children.push(fixed_width_cell(
                    rich_text_single(sp, base_font, font_px, line_px),
                    cell_w,
                    line_px.0,
                    None,
                ));
            }
            let mid_text = if mid.text.is_empty() { " " } else { mid.text.as_str() };
            let sp = cursor_cell_span(mid_text, cur, run, font_px, base_font);
            children.push(fixed_width_cell(
                rich_text_single(sp, base_font, font_px, line_px),
                cell_w,
                line_px.0,
                None,
            ));
            for cell in after {
                let piece = if cell.text.is_empty() {
                    " "
                } else {
                    cell.text.as_str()
                };
                let sp = span_from_fragment(run, piece, font_px, base_font, cell.continuation);
                children.push(fixed_width_cell(
                    rich_text_single(sp, base_font, font_px, line_px),
                    cell_w,
                    line_px.0,
                    None,
                ));
            }
        }
        col = run_end;
    }

    if children.is_empty() {
        let el: Element<'a, Message> = text(" ")
            .size(font_px)
            .font(base_font)
            .line_height(LineHeight::Absolute(line_px))
            .into();
        children.push(fixed_width_cell(el, cell_w, line_px.0, None));
    }

    Row::with_children(children).spacing(0.0).clip(true).into()
}

fn normalize_sel(a: (u16, u16), b: (u16, u16)) -> ((u16, u16), (u16, u16)) {
    if (a.1, a.0) <= (b.1, b.0) {
        (a, b)
    } else {
        (b, a)
    }
}

fn cell_in_selection(sel: Option<((u16, u16), (u16, u16))>, x: u16, y: u16) -> bool {
    let Some((a, b)) = sel else { return false; };
    let ((sx, sy), (ex, ey)) = normalize_sel(a, b);
    if y < sy || y > ey {
        return false;
    }
    if sy == ey {
        x >= sx && x <= ex
    } else if y == sy {
        x >= sx
    } else if y == ey {
        x <= ex
    } else {
        true
    }
}

fn split_run_three_cells<'a>(
    cells: &'a [VtStyledCell],
    at_cell: u16,
) -> (&'a [VtStyledCell], &'a VtStyledCell, &'a [VtStyledCell]) {
    let at = (at_cell as usize).min(cells.len().saturating_sub(1));
    let (before, rest) = cells.split_at(at);
    let (mid, after) = rest.split_at(1);
    (before, &mid[0], after)
}

fn cursor_cell_span<'a>(
    mid: &'a str,
    cursor: &'a CursorState,
    run: &VtStyledRun,
    font_px: f32,
    base_font: Font,
) -> Span<'a, (), Font> {
    let fg = ghost_rgb(&run.fg);
    let cell_bg = if run.has_bg {
        ghost_rgb(&run.bg)
    } else {
        Color::from_rgb8(10, 10, 15)
    };
    let cc = ghost_rgb(&cursor.color);

    let mut s = span(if mid.is_empty() { " " } else { mid })
    .size(font_px)
    .font(run_font(run, base_font))
    .strikethrough(run.strikethrough);

    match cursor.visual_style {
        ffi::GhosttyRenderStateCursorVisualStyle_GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_UNDERLINE => {
            let mut fg_u = fg;
            if run.dim {
                fg_u = fg_u.scale_alpha(0.72);
            }
            s = s.color(fg_u).underline(true);
            if run.has_bg {
                s = s.background(iced::Background::Color(ghost_rgb(&run.bg)));
            }
        }
        ffi::GhosttyRenderStateCursorVisualStyle_GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BAR => {
            s = span("▎")
                .color(cc)
                .size(font_px)
                .font(base_font);
        }
        _ => {
            s = s.color(cell_bg).background(iced::Background::Color(cc));
        }
    }
    s
}
