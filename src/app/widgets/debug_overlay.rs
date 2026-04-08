//! Performance debug overlay widget for Iced.
//!
//! Displays real-time tick/pump/VT timing statistics in a semi-transparent panel.
//! Toggle visibility with the `ToggleDebugOverlay` message (keyboard shortcut: Ctrl+Shift+D).

use iced::widget::text;
use iced::widget::Container;
use iced::widget::Space;
use iced::widget::Column;

use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::theme::{DebugTokens, DesignTokens, RustSshThemeId};

const OVERLAY_PAD: f32 = 12.0;

/// Build the debug overlay content from current state.
pub(crate) fn make_debug_overlay(state: &IcedState) -> iced::Element<'_, Message> {
    let perf = &state.perf;
    let n = state.tab_panes.len();
    let focused = state.window_focused;

    // 获取当前主题的 tokens
    let theme_id = match state.model.settings.general.theme.as_str() {
        "Light" => RustSshThemeId::Light,
        "Warm" => RustSshThemeId::Warm,
        "GitHub" => RustSshThemeId::GitHub,
        _ => RustSshThemeId::Dark,
    };
    let tokens = DesignTokens::for_id(theme_id);
    let debug = DebugTokens::from_design_tokens(&tokens);

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

    // 捕获 debug tokens 到闭包中（Copy trait 使其可以移动）
    let debug_bg = debug.bg;
    let debug_title = debug.text_title;
    let debug_muted = debug.text_muted;
    let debug_good = debug.text_good;
    let debug_warn = debug.text_warn;
    let debug_error = debug.text_error;
    let debug_tab_first = debug.tab_first;
    let debug_tab_other = debug.tab_other;

    // 基于性能指标选择颜色
    let tick_line_color = if slow_pct > 10.0 {
        debug_error
    } else if slow_pct > 5.0 {
        debug_warn
    } else {
        debug_good
    };

    let perf_line_color = if empty_pct > 50.0 || slow_pct > 10.0 {
        debug_warn
    } else {
        debug_muted
    };

    // 构建每个 tab 的行
    let tab_lines: Vec<iced::Element<'_, Message>> = (0..n)
        .map(|i| {
            let pump_calls = (*perf.tab_pump_calls.get(i).unwrap_or(&0)).max(1) as f64;
            let vt_ns = *perf.tab_vt_ns.get(i).unwrap_or(&0) as f64;
            let frame_ns = *perf.tab_vt_frame_ns.get(i).unwrap_or(&0) as f64;
            let color = if i == 0 { debug_tab_first } else { debug_tab_other };

            text(format!(
                "[{}] pump={} bytes={} vt={:.1}ms frame={:.1}ms",
                i,
                *perf.tab_pump_calls.get(i).unwrap_or(&0),
                fmt_bytes(*perf.tab_bytes_in.get(i).unwrap_or(&0)),
                vt_ns / 1_000_000.0 / pump_calls,
                frame_ns / 1_000_000.0 / pump_calls,
            ))
            .style(move |_| iced::widget::text::Style {
                color: Some(color),
                ..Default::default()
            })
            .into()
        })
        .collect();

    let hist_vals: Vec<String> = perf.tick_histogram_ms().iter().map(|v| format!("{:.1}", v)).collect();
    let hist_text = text(hist_vals.join(" "))
        .size(10.0)
        .style(move |_| iced::widget::text::Style {
            color: Some(debug_muted),
            ..Default::default()
        });

    let mut col = Column::new()
        .push(
            text("DebugOverlay").style(move |_| iced::widget::text::Style {
                color: Some(debug_title),
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
                color: Some(tick_line_color),
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
                color: Some(perf_line_color),
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
        .push(
            text("────────────────────────────").style(move |_| iced::widget::text::Style {
                color: Some(debug_muted),
                ..Default::default()
            }),
        )
        .push(Space::new())
        .push(
            text("Per-tab:").style(move |_| iced::widget::text::Style {
                color: Some(debug_title),
                ..Default::default()
            }),
        )
        .push(Space::new());

    for line in tab_lines {
        col = col.push(line).push(Space::new());
    }

    col = col
        .push(Space::new())
        .push(
            text("────────────────────────────").style(move |_| iced::widget::text::Style {
                color: Some(debug_muted),
                ..Default::default()
            }),
        )
        .push(Space::new())
        .push(
            text("Tick history (ms):").style(move |_| iced::widget::text::Style {
                color: Some(debug_muted),
                ..Default::default()
            }),
        )
        .push(Space::new())
        .push(hist_text)
        .push(Space::new())
        .push(
            text("Ctrl+Shift+D to toggle").style(move |_| iced::widget::text::Style {
                color: Some(debug_muted),
                ..Default::default()
            }),
        );

    Container::new(col)
        .padding(iced::Padding::from(OVERLAY_PAD))
        .max_width(500.0)
        .style(move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(debug_bg)),
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
