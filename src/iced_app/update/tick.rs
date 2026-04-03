use iced::Task;

use crate::settings;

use super::super::state::IcedState;

/// Handle Tick message: pump SSH sessions, update perf counters, cursor blink.
pub(crate) fn handle_tick(state: &mut IcedState) -> Task<super::Message> {
    let now = settings::unix_time_ms();
    state.tick_count = state.tick_count.wrapping_add(1);
    state.perf.ticks += 1;

    let bg_pump_every_ms: i64 = if state.window_focused { 200 } else { 250 };

    if state.model.settings.quick_connect.single_shared_session {
        pump_single_session(state, now);
    } else {
        pump_multi_sessions(state, now, bg_pump_every_ms);
    }

    handle_cursor_blink(state, now);
    handle_perf_log(state);

    Task::none()
}

fn pump_single_session(state: &mut IcedState, now: i64) {
    let active = state.active_tab;
    let mut bytes_in = 0u64;
    let mut pumped = false;

    if let Some(pane) = state.tab_panes.get_mut(active) {
        if let Some(session) = pane.session.as_mut() {
            pumped = true;
            if let Ok(n) = pane.terminal.pump_output(session.as_mut()) {
                if n > 0 {
                    bytes_in = n as u64;
                }
            }
        }
    }

    if pumped {
        state.perf.pump_calls += 1;
    }
    if bytes_in > 0 {
        state.last_activity_ms = now;
        state.perf.bytes_in += bytes_in;
    }
}

fn pump_multi_sessions(state: &mut IcedState, now: i64, bg_pump_every_ms: i64) {
    let active = state.active_tab;

    for (i, p) in state.tab_panes.iter_mut().enumerate() {
        let Some(session) = p.session.as_mut() else {
            continue;
        };

        let should_pump = if i == active {
            true
        } else {
            now.saturating_sub(p.last_pump_ms) >= bg_pump_every_ms
        };

        if !should_pump {
            continue;
        }

        p.last_pump_ms = now;
        state.perf.pump_calls += 1;

        if let Ok(n) = p.terminal.pump_output(session.as_mut()) {
            if n > 0 {
                state.last_activity_ms = now;
                state.perf.bytes_in += n as u64;
            }
        }
    }
}

fn handle_cursor_blink(state: &mut IcedState, now: i64) {
    let blink_due = now.saturating_sub(state.last_blink_tick_ms) >= 500;

    if blink_due && state.window_focused {
        for (i, p) in state.tab_panes.iter_mut().enumerate() {
            if state.model.settings.quick_connect.single_shared_session && i != state.active_tab {
                continue;
            }
            p.terminal.on_frame_tick();
        }
        state.last_blink_tick_ms = now;
    }
}

fn handle_perf_log(state: &mut IcedState) {
    let now = settings::unix_time_ms();

    if now.saturating_sub(state.perf.last_log_ms) < 8_000 {
        return;
    }

    let dt = (now - state.perf.last_log_ms).max(1) as f64 / 1000.0;
    let ticks = state.perf.ticks - state.perf.ticks_at_log;
    let pumps = state.perf.pump_calls - state.perf.pump_calls_at_log;
    let bytes = state.perf.bytes_in - state.perf.bytes_in_at_log;

    let mut key_fb_named = 0u64;
    let mut key_fb_text = 0u64;

    for (i, p) in state.tab_panes.iter_mut().enumerate() {
        if state.model.settings.quick_connect.single_shared_session && i != state.active_tab {
            continue;
        }
        let (n_named, n_text) = p.terminal.take_key_fallback_counts();
        key_fb_named = key_fb_named.saturating_add(n_named);
        key_fb_text = key_fb_text.saturating_add(n_text);
    }

    if let Some(path) = state.perf.dump_path.clone() {
        write_perf_dump(
            &mut *state,
            &path,
            now,
            dt,
            ticks,
            pumps,
            bytes,
            key_fb_named,
            key_fb_text,
        );
    }

    log::debug!(
        target: "term-prof",
        "perf tick_rate={:.1}/s pump_calls={:.1}/s bytes_in={}/s key_fb_named={:.2}/s key_fb_text={:.2}/s focused={} shared_session={}",
        (ticks as f64) / dt,
        (pumps as f64) / dt,
        (bytes as f64 / dt) as u64,
        (key_fb_named as f64) / dt,
        (key_fb_text as f64) / dt,
        state.window_focused,
        state.model.settings.quick_connect.single_shared_session
    );

    state.perf.last_log_ms = now;
    state.perf.ticks_at_log = state.perf.ticks;
    state.perf.pump_calls_at_log = state.perf.pump_calls;
    state.perf.bytes_in_at_log = state.perf.bytes_in;
}

fn write_perf_dump(
    state: &mut IcedState,
    path: &str,
    now: i64,
    dt: f64,
    ticks: u64,
    pumps: u64,
    bytes: u64,
    key_fb_named: u64,
    key_fb_text: u64,
) {
    if !state.perf.dump_header_written {
        let _ = std::fs::create_dir_all(
            std::path::Path::new(path)
                .parent()
                .unwrap_or_else(|| std::path::Path::new(".")),
        );
        let header = "ts_ms,tick_rate_per_s,pump_calls_per_s,bytes_in_per_s,key_fb_named_per_s,key_fb_text_per_s,focused,shared_session\n";
        let _ = std::fs::write(path, header);
        state.perf.dump_header_written = true;
    }

    let line = format!(
        "{},{:.3},{:.3},{},{:.3},{:.3},{},{}\n",
        now,
        (ticks as f64) / dt,
        (pumps as f64) / dt,
        (bytes as f64 / dt) as u64,
        (key_fb_named as f64) / dt,
        (key_fb_text as f64) / dt,
        state.window_focused,
        state.model.settings.quick_connect.single_shared_session
    );

    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        use std::io::Write;
        let _ = f.write_all(line.as_bytes());
    }
}
