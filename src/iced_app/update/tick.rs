use iced::Task;

use crate::settings;

use super::super::message::Message;
use super::super::state::IcedState;

/// Slow tick threshold: ticks taking longer than this are logged as warnings.
const SLOW_TICK_NS: u64 = 5_000_000; // 5ms

/// Handle Tick message: pump SSH sessions, update perf counters, cursor blink.
pub(crate) fn handle_tick(state: &mut IcedState) -> Task<Message> {
    let now = settings::unix_time_ms();
    let tick_start = std::time::Instant::now();

    state.tick_count += 1;
    state.perf.ticks += 1;

    // Ensure tab arrays match current tab count (grows with new tabs).
    state.perf.ensure_tabs(state.tab_panes.len());

    let bg_pump_every_ms: i64 = if state.window_focused { 200 } else { 250 };
    pump_all_sessions(state, now, bg_pump_every_ms);
    handle_cursor_blink(state, now);

    let tick_elapsed = tick_start.elapsed().as_nanos() as u64;
    state.perf.tick_ns_total += tick_elapsed;
    state.perf.tick_durations_ns.push(tick_elapsed);

    // Slow tick warning.
    if tick_elapsed > SLOW_TICK_NS {
        state.perf.slow_ticks += 1;
        log::warn!(
            target: "term-perf",
            "slow_tick: {:.2}ms (threshold={}ms)",
            tick_elapsed as f64 / 1_000_000.0,
            SLOW_TICK_NS as f64 / 1_000_000.0
        );
    }

    handle_perf_log(state);
    Task::none()
}

/// Pump all registered SSH sessions (one per tab).
fn pump_all_sessions(state: &mut IcedState, now: i64, bg_pump_every_ms: i64) {
    let active = state.active_tab;

    for (i, pane) in state.tab_panes.iter_mut().enumerate() {
        let Some(session) = state.session_manager.session_mut(i) else {
            continue;
        };

        let should_pump = if i == active {
            true
        } else {
            now.saturating_sub(pane.last_pump_ms) >= bg_pump_every_ms
        };

        if !should_pump {
            continue;
        }

        pane.last_pump_ms = now;
        state.perf.pump_calls += 1;

        // Capture VT update time separately for diagnostics.
        let vt_start = std::time::Instant::now();
        let pump_result = pane.terminal.pump_output(session);
        let vt_elapsed = vt_start.elapsed().as_nanos() as u64;

        state.perf.vt_update_ns_total += vt_elapsed;
        state.perf.tab_vt_ns[i] += vt_elapsed;

        match pump_result {
            Ok(n) => {
                state.perf.tab_pump_calls[i] += 1;
                if n > 0 {
                    state.last_activity_ms = now;
                    state.perf.bytes_in += n as u64;
                    state.perf.tab_bytes_in[i] += n as u64;
                } else {
                    // Empty read: no data this pump cycle.
                    state.perf.pump_empty_reads += 1;
                }
            }
            Err(e) => {
                log::debug!(target: "term-perf", "pump_output error tab={}: {e}", i);
            }
        }
    }
}

/// The vt_start.elapsed() timer variable — need to track it separately.
fn handle_cursor_blink(state: &mut IcedState, now: i64) {
    let blink_due = now.saturating_sub(state.last_blink_tick_ms) >= 500;
    if blink_due && state.window_focused {
        for (i, pane) in state.tab_panes.iter_mut().enumerate() {
            let frame_start = std::time::Instant::now();
            pane.terminal.on_frame_tick();
            let frame_elapsed = frame_start.elapsed().as_nanos() as u64;
            state.perf.vt_frame_ns_total += frame_elapsed;
            if i < state.perf.tab_vt_frame_ns.len() {
                state.perf.tab_vt_frame_ns[i] += frame_elapsed;
            }
        }
        state.last_blink_tick_ms = now;
    }
}

fn handle_perf_log(state: &mut IcedState) {
    let now = settings::unix_time_ms();

    if now.saturating_sub(state.perf.last_log_ms) < 8_000 {
        return;
    }

    let dt_ms = (now - state.perf.last_log_ms).max(1);

    let dt = dt_ms as f64 / 1000.0;
    let ticks = state.perf.ticks - state.perf.ticks_at_log;
    let pumps = state.perf.pump_calls - state.perf.pump_calls_at_log;
    let bytes = state.perf.bytes_in - state.perf.bytes_in_at_log;

    // Slow tick delta since last log.
    let slow_ticks_delta = state.perf.slow_ticks.saturating_sub(
        state.perf.slow_ticks_at_log.unwrap_or(0),
    );
    state.perf.slow_ticks_at_log = Some(state.perf.slow_ticks);

    // Aggregate key fallback counts across all tabs.
    let mut key_fb_named = 0u64;
    let mut key_fb_text = 0u64;
    for pane in &mut state.tab_panes {
        let (n_named, n_text) = pane.terminal.take_key_fallback_counts();
        key_fb_named = key_fb_named.saturating_add(n_named);
        key_fb_text = key_fb_text.saturating_add(n_text);
    }

    if let Some(path) = state.perf.dump_path.clone() {
        write_perf_dump(
            state,
            path,
            now,
            dt_ms,
            dt,
            ticks,
            pumps,
            bytes,
            key_fb_named,
            key_fb_text,
            slow_ticks_delta,
        );
    }

    let tick_rate = ticks as f64 / dt;
    let pump_rate = pumps as f64 / dt;
    let bytes_rate = bytes as f64 / dt;
    let empty_pct = if pumps > 0 {
        let empty_delta = state.perf.pump_empty_reads.saturating_sub(
            state.perf.pump_empty_reads_at_log.unwrap_or(0),
        );
        state.perf.pump_empty_reads_at_log = Some(state.perf.pump_empty_reads);
        empty_delta as f64 * 100.0 / pumps as f64
    } else {
        0.0
    };

    log::debug!(
        target: "term-prof",
        "perf tick={:.1}/s pump={:.1}/s bytes={:.0}/s empty={:.1}% slow={} vt_upd={:.1}ms vt_frame={:.1}ms focus={} tabs={}",
        tick_rate,
        pump_rate,
        bytes_rate,
        empty_pct,
        slow_ticks_delta,
        state.perf.vt_update_ns_total as f64 / 1_000_000.0,
        state.perf.vt_frame_ns_total as f64 / 1_000_000.0,
        state.window_focused,
        state.tab_panes.len()
    );

    state.perf.last_log_ms = now;
    state.perf.ticks_at_log = state.perf.ticks;
    state.perf.pump_calls_at_log = state.perf.pump_calls;
    state.perf.bytes_in_at_log = state.perf.bytes_in;
}

fn write_perf_dump(
    state: &mut IcedState,
    path: String,
    now: i64,
    _dt_ms: i64,
    dt: f64,
    ticks: u64,
    pumps: u64,
    bytes: u64,
    key_fb_named: u64,
    key_fb_text: u64,
    slow_ticks_delta: u64,
) {
    if !state.perf.dump_header_written {
        let _ = std::fs::create_dir_all(
            std::path::Path::new(&path)
                .parent()
                .unwrap_or_else(|| std::path::Path::new(".")),
        );
        let header = "ts_ms,tick_rate_per_s,pump_calls_per_s,bytes_in_per_s,empty_read_pct,slow_ticks,vt_update_ms,vt_frame_ms,key_fb_named_per_s,key_fb_text_per_s,focused,tabs,tab_details,tab_vt_details\n";
        let _ = std::fs::write(&path, header);
        state.perf.dump_header_written = true;
    }

    let tick_rate = ticks as f64 / dt;
    let pump_rate = pumps as f64 / dt;
    let bytes_rate = bytes as f64 / dt;

    // Per-tab pump/bytes summary.
    let tab_details: String = (0..state.tab_panes.len())
        .map(|i| {
            let p = *state.perf.tab_pump_calls.get(i).unwrap_or(&0);
            let b = *state.perf.tab_bytes_in.get(i).unwrap_or(&0);
            format!("{i}={p}/{b}")
        })
        .collect::<Vec<_>>()
        .join(",");

    let empty_delta = state.perf.pump_empty_reads.saturating_sub(
        state.perf.pump_empty_reads_at_log.unwrap_or(0),
    );
    state.perf.pump_empty_reads_at_log = Some(state.perf.pump_empty_reads);

    let empty_pct = if pumps > 0 {
        empty_delta as f64 * 100.0 / pumps as f64
    } else {
        0.0
    };

    let vt_update_ms = state.perf.vt_update_ns_total as f64 / 1_000_000.0;
    let vt_frame_ms = state.perf.vt_frame_ns_total as f64 / 1_000_000.0;

    // Per-tab VT timing details.
    let tab_vt_details: String = (0..state.tab_panes.len())
        .map(|i| {
            let p_ns = *state.perf.tab_vt_ns.get(i).unwrap_or(&0);
            let f_ns = *state.perf.tab_vt_frame_ns.get(i).unwrap_or(&0);
            format!("{i}={:.1}/{:.1}", p_ns as f64 / 1_000_000.0, f_ns as f64 / 1_000_000.0)
        })
        .collect::<Vec<_>>()
        .join(",");

    let line = format!(
        "{},{:.3},{:.3},{:.1},{:.1},{},{:.3},{:.3},{:.3},{:.3},{},{},{},{}\n",
        now,
        tick_rate,
        pump_rate,
        bytes_rate,
        empty_pct,
        slow_ticks_delta,
        vt_update_ms,
        vt_frame_ms,
        key_fb_named as f64 / dt,
        key_fb_text as f64 / dt,
        state.window_focused,
        state.tab_panes.len(),
        tab_details,
        tab_vt_details
    );

    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        use std::io::Write;
        let _ = f.write_all(line.as_bytes());
    }
}
