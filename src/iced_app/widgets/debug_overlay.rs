//! Performance debug overlay widget for Iced.
//!
//! Displays real-time tick/pump/VT timing statistics in a semi-transparent panel.
//! Toggle visibility with the `ToggleDebugOverlay` message (keyboard shortcut: Ctrl+Shift+D).

use iced::widget::text;
use iced::widget::Container;
use iced::widget::Space;
use iced::widget::Column;

use crate::iced_app::message::Message;
use crate::iced_app::state::IcedState;

/// Overlay style constants.
///
/// NOTE: Debug overlay intentionally uses fixed colors for clarity.
/// These should remain high-contrast for readability across all themes.
const OVERLAY_BG: iced::Color = iced::Color {
    r: 0.08,
    g: 0.08,
    b: 0.12,
    a: 0.88,
};
const OVERLAY_PAD: f32 = 12.0;

/// Per-tab summary line.
struct TabLine {
    idx: usize,
    pump: u64,
    bytes: u64,
    vt_ms: f64,
    frame_ms: f64,
}

impl<'a> From<TabLine> for iced::Element<'a, Message> {
    fn from(val: TabLine) -> Self {
        // Colors tuned for debug readability across all themes
        // (developer-facing, so high contrast is prioritized)
        let color = if val.idx == 0 {
            iced::Color::from_rgb(0.6, 0.9, 0.6) // First tab: green tint
        } else {
            iced::Color::from_rgb(0.7, 0.7, 0.8) // Other tabs: neutral gray
        };
        let color_inner = color;
        text(format!(
            "[{}] pump={} bytes={} vt={:.1}ms frame={:.1}ms",
            val.idx,
            val.pump,
            fmt_bytes(val.bytes),
            val.vt_ms,
            val.frame_ms,
        ))
        .style(move |_| iced::widget::text::Style {
            color: Some(color_inner),
            ..Default::default()
        })
        .into()
    }
}

/// Build the debug overlay content from current state.
pub(crate) fn make_debug_overlay(state: &IcedState) -> iced::Element<'_, Message> {
    let perf = &state.perf;
    let n = state.tab_panes.len();
    let focused = state.window_focused;

    let slow_pct = if perf.ticks > 0 {
        perf.slow_ticks as f64 / perf.ticks as f64 * 100.0
    } else {
        0.0
    };
    let empty_pct = if perf.pump_calls > 0 {
        perf.pump_empty_reads as f64 / perf.pump_calls as f64 * 100.0
    } else {
        0.0
    };

    let ticks_nonzero = perf.ticks.max(1) as f64;
    let vt_ms = perf.vt_update_ns_total as f64 / 1_000_000.0 / ticks_nonzero;
    let frame_ms = perf.vt_frame_ns_total as f64 / 1_000_000.0 / ticks_nonzero;

    let avg_tick_ms = perf.tick_durations_ns.average_ns().unwrap_or(0.0) / 1_000_000.0;
    let max_tick = perf
        .tick_durations_ns
        .max()
        .map(|v| v as f64)
        .unwrap_or(0.0)
        / 1_000_000.0;

    let ticks_60 = perf.tick_durations_ns.len() as f64;
    let tick_rate = if ticks_60 > 0.0 {
        perf.ticks as f64 / ticks_60 * 60.0
    } else {
        0.0
    };
    let pump_rate = if ticks_60 > 0.0 {
        perf.pump_calls as f64 / ticks_60 * 60.0
    } else {
        0.0
    };
    let bytes_rate = if ticks_60 > 0.0 {
        perf.bytes_in as f64 / ticks_60 * 60.0
    } else {
        0.0
    };

    let tick_line_color = if slow_pct > 10.0 {
        iced::Color::from_rgb(0.95, 0.4, 0.4)
    } else if slow_pct > 5.0 {
        iced::Color::from_rgb(0.95, 0.75, 0.4)
    } else {
        iced::Color::from_rgb(0.55, 0.9, 0.55)
    };

    let perf_line_color = if empty_pct > 50.0 || slow_pct > 10.0 {
        iced::Color::from_rgb(0.95, 0.75, 0.4)
    } else {
        iced::Color::from_rgb(0.45, 0.45, 0.55)
    };

    let title_color = iced::Color::from_rgb(0.9, 0.85, 0.6);
    let muted_color = iced::Color::from_rgb(0.45, 0.45, 0.55);

    let divider = || {
        let c = muted_color;
        text("────────────────────────────")
            .style(move |_| iced::widget::text::Style {
                color: Some(c),
                ..Default::default()
            })
    };

    let tab_lines: Vec<iced::Element<'_, Message>> = (0..n)
        .map(|i| {
            let pump_calls = (*perf.tab_pump_calls.get(i).unwrap_or(&0)).max(1) as f64;
            let vt_ns = *perf.tab_vt_ns.get(i).unwrap_or(&0) as f64;
            let frame_ns = *perf.tab_vt_frame_ns.get(i).unwrap_or(&0) as f64;
            iced::Element::from(TabLine {
                idx: i,
                pump: *perf.tab_pump_calls.get(i).unwrap_or(&0),
                bytes: *perf.tab_bytes_in.get(i).unwrap_or(&0),
                vt_ms: vt_ns / 1_000_000.0 / pump_calls,
                frame_ms: frame_ns / 1_000_000.0 / pump_calls,
            })
        })
        .collect();

    let hist_vals: Vec<String> = perf
        .tick_histogram_ms()
        .iter()
        .map(|v| format!("{:.1}", v))
        .collect();
    let hist_text = text(hist_vals.join(" "))
        .size(10.0)
        .style(move |_| iced::widget::text::Style {
            color: Some(iced::Color::from_rgb(0.45, 0.45, 0.55)),
            ..Default::default()
        });

    let title_color_inner = title_color;
    let tick_line_inner = tick_line_color;
    let perf_line_inner = perf_line_color;
    let muted_inner = muted_color;

    let mut col = Column::new()
        .push(
            text("⚙ DebugOverlay").style(move |_| iced::widget::text::Style {
                color: Some(title_color_inner),
                ..Default::default()
            }),
        )
        .push(Space::new())
        .push(
            text(format!(
                "tick: {:.1}/s  pump: {:.1}/s  bytes: {:.0}/s",
                tick_rate, pump_rate, bytes_rate
            ))
            .style(move |_| iced::widget::text::Style {
                color: Some(tick_line_inner),
                ..Default::default()
            }),
        )
        .push(Space::new())
        .push(
            text(format!(
                "slow={:.1}%  empty={:.1}%  vt={:.2}ms  frame={:.2}ms",
                slow_pct, empty_pct, vt_ms, frame_ms
            ))
            .style(move |_| iced::widget::text::Style {
                color: Some(perf_line_inner),
                ..Default::default()
            }),
        )
        .push(Space::new())
        .push(text(format!(
            "tick-avg={:.2}ms  tick-max={:.2}ms  ticks={}",
            avg_tick_ms, max_tick, perf.ticks
        )))
        .push(Space::new())
        .push(text(format!("focus={}  tabs={}  overlay=ON", focused, n)))
        .push(Space::new())
        .push(divider())
        .push(Space::new())
        .push(
            text("Per-tab:").style(move |_| iced::widget::text::Style {
                color: Some(title_color),
                ..Default::default()
            }),
        )
        .push(Space::new());

    for line in tab_lines {
        col = col.push(line).push(Space::new());
    }

    col = col
        .push(Space::new())
        .push(divider())
        .push(Space::new())
        .push(
            text("Tick history (ms):").style(move |_| iced::widget::text::Style {
                color: Some(muted_inner),
                ..Default::default()
            }),
        )
        .push(Space::new())
        .push(hist_text)
        .push(Space::new())
        .push(
            text("Ctrl+Shift+D to toggle").style(move |_| iced::widget::text::Style {
                color: Some(muted_color),
                ..Default::default()
            }),
        );

    Container::new(col)
        .padding(iced::Padding::from(OVERLAY_PAD))
        .max_width(500.0)
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(OVERLAY_BG)),
            ..Default::default()
        })
        .into()
}

fn fmt_bytes(n: u64) -> String {
    if n >= 1024 * 1024 {
        format!("{:.1}M", n as f64 / (1024.0 * 1024.0))
    } else if n >= 1024 {
        format!("{:.1}K", n as f64 / 1024.0)
    } else {
        format!("{}", n)
    }
}
