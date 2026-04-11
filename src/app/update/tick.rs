use iced::Task;

use crate::settings;

use super::super::message::Message;
use super::super::state::PrewarmStatus;
use super::super::state::{IcedState, ConnectionStage};

/// Slow tick threshold: ticks taking longer than this are logged as warnings.
const SLOW_TICK_NS: u64 = 5_000_000; // 5ms

/// 重连 tick 间隔（毫秒）：每 1 秒触发一次重连倒计时检查
const RECONNECT_TICK_MS: i64 = 1000;

/// 预热 tick 间隔（毫秒）：每 500ms 检查悬停状态
const PREWARM_TICK_MS: i64 = 500;

/// Compute dynamic tick interval (ms) — must match the logic in `subscription.rs`.
fn compute_tick_ms(state: &IcedState) -> u32 {
    use crate::settings;
    let now = settings::unix_time_ms();
    let idle_ms = now.saturating_sub(state.last_activity_ms).max(0) as u64;
    let target_fps = state.model.settings.terminal.target_fps.clamp(10, 240);
    if !state.window_focused {
        250
    } else if idle_ms <= 1_000 {
        (1000 / target_fps).max(16)
    } else if idle_ms <= 5_000 {
        (1000 / target_fps.min(30)).max(33)
    } else {
        (1000 / target_fps.min(10)).max(100)
    }
}

/// Handle Tick message: pump SSH sessions, update perf counters, cursor blink.
pub(crate) fn handle_tick(state: &mut IcedState) -> Task<Message> {
    // 安全边界：欢迎页且无页签时，所有需要终端的操作都跳过
    if state.tab_panes.is_empty() {
        return Task::none();
    }

    let now = settings::unix_time_ms();
    let tick_start = std::time::Instant::now();

    state.tick_count += 1;
    state.perf.ticks += 1;

    // Ensure tab arrays match current tab count (grows with new tabs).
    state.perf.ensure_tabs(state.tab_panes.len());

    let tick_ms = compute_tick_ms(state) as f32;
    state.tick_tab_anims(tick_ms);
    state.tick_modal_anims(tick_ms);

    let bg_pump_every_ms: i64 = if state.window_focused { 200 } else { 250 };
    let exit_task = pump_all_sessions(state, now, bg_pump_every_ms);
    handle_cursor_blink(state, now);

    // 重连计时器：每 1 秒触发一次
    let reconnect_task = handle_reconnect_timer(state, now);

    // 预热计时器：每 500ms 检查悬停状态
    let prewarm_task = handle_prewarm_timer(state, now);

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

    // 滚动动画插值
    handle_tab_scroll_animation(state);

    // 合并两个 Task
    reconnect_task.chain(prewarm_task).chain(exit_task.unwrap_or_else(Task::none))
}

/// Handle tab scroll animation: ease current offset toward target.
fn handle_tab_scroll_animation(state: &mut IcedState) {
    if let Some(target) = state.tab_scroll_target {
        let diff = target - state.tab_scroll_offset;
        if diff.abs() < 1.0 {
            state.tab_scroll_offset = target;
            state.tab_scroll_target = None;
        } else {
            // 缓动：每次向目标移动 20%
            state.tab_scroll_offset += diff * 0.2;
        }
    }
}

/// Handle reconnect timer: checks if it's time to trigger a reconnect tick.
/// Returns a Task if a reconnect should be attempted.
fn handle_reconnect_timer(state: &mut IcedState, now: i64) -> Task<Message> {
    // 只在 Reconnecting 状态检查重连计时
    if !matches!(state.connection_stage, ConnectionStage::Reconnecting { .. }) {
        return Task::none();
    }

    // 检查是否达到 1 秒间隔
    if now.saturating_sub(state.last_reconnect_tick_ms) < RECONNECT_TICK_MS {
        return Task::none();
    }

    state.last_reconnect_tick_ms = now;

    // 触发重连 tick
    super::connection::handle_reconnect_tick(state)
}

/// Handle prewarm timer: checks if hover has elapsed enough time to start connecting.
fn handle_prewarm_timer(state: &mut IcedState, _now: i64) -> Task<Message> {
    // 检查是否需要处理预热
    let needs_connect = {
        let Some(prewarm) = &mut state.prewarm_state else {
            return Task::none();
        };

        match prewarm.status {
            PrewarmStatus::Idle | PrewarmStatus::Connecting | PrewarmStatus::Ready | PrewarmStatus::Failed => {
                // 这些状态不需要计时
                false
            }
            PrewarmStatus::WaitingHover => {
                // 检查是否达到 500ms
                let elapsed = prewarm
                    .start_time
                    .map(|t| t.elapsed().as_millis() as i64)
                    .unwrap_or(0);

                if elapsed < 500 {
                    false
                } else {
                    true
                }
            }
        }
    };

    if needs_connect {
        // 重新获取 prewarm_state，设置状态并启动连接
        if let Some(prewarm) = &mut state.prewarm_state {
            let profile_id = prewarm.profile_id.clone();
            prewarm.status = PrewarmStatus::Connecting;
            return super::connection::start_prewarm_connect(state, profile_id);
        }
    }

    Task::none()
}

/// Pump all registered SSH sessions (one per tab).
/// Returns a Task if any session has exited and needs to close its tab.
fn pump_all_sessions(state: &mut IcedState, now: i64, bg_pump_every_ms: i64) -> Option<Task<Message>> {
    let active = state.active_tab;

    for (i, pane) in state.tab_panes.iter_mut().enumerate() {
        let Some(session) = state.tab_manager.session_mut(i) else {
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
                    // 如果连接已断开（收到 EOF 但 exit_status 尚未到达），立即关闭页签
                    if !session.is_connected() && !state.tabs.is_empty() {
                        state.tab_manager.detach_session(i);
                        return Some(super::update(state, Message::SessionExited(i)));
                    }
                }
            }
            Err(e) => {
                log::debug!(target: "term-perf", "pump_output error tab={}: {e}", i);
            }
        }

        // Detect session exit: user ran `exit` and the shell terminated.
        // Detach the session from tab_manager (which notifies shared_manager),
        // then return a Task to close the tab.
        if session.exit_status().is_some() && !state.tabs.is_empty() {
            state.tab_manager.detach_session(i);
            return Some(super::update(state, Message::SessionExited(i)));
        }
    }
    None
}

/// Handle cursor blink and frame tick — active tab only.
fn handle_cursor_blink(state: &mut IcedState, now: i64) {
    let blink_due = now.saturating_sub(state.last_blink_tick_ms) >= 500;
    if blink_due && state.window_focused {
        let active = state.active_tab;
        if let Some(pane) = state.tab_panes.get_mut(active) {
            let frame_start = std::time::Instant::now();
            pane.terminal.on_frame_tick();
            let frame_elapsed = frame_start.elapsed().as_nanos() as u64;
            state.perf.vt_frame_ns_total += frame_elapsed;
            if active < state.perf.tab_vt_frame_ns.len() {
                state.perf.tab_vt_frame_ns[active] += frame_elapsed;
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
        "perf tick={:.1}/s pump={:.1}/s bytes={:.0}/s empty={:.1}% slow={} vt_upd={:.3}ms vt_frame={:.3}ms (per-tick avg) focus={} tabs={}",
        tick_rate,
        pump_rate,
        bytes_rate,
        empty_pct,
        slow_ticks_delta,
        state.perf.vt_update_ns_total as f64 / 1_000_000.0 / (state.perf.ticks.max(1) as f64),
        state.perf.vt_frame_ns_total as f64 / 1_000_000.0 / (state.perf.ticks.max(1) as f64),
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

    let ticks_nonzero = state.perf.ticks.max(1);
    let vt_update_ms = state.perf.vt_update_ns_total as f64 / 1_000_000.0 / ticks_nonzero as f64;
    let vt_frame_ms = state.perf.vt_frame_ns_total as f64 / 1_000_000.0 / ticks_nonzero as f64;

    // Per-tab VT timing details (also per-tick averages).
    let tab_vt_details: String = (0..state.tab_panes.len())
        .map(|i| {
            let p_ns = *state.perf.tab_vt_ns.get(i).unwrap_or(&0);
            let f_ns = *state.perf.tab_vt_frame_ns.get(i).unwrap_or(&0);
            format!(
                "{i}={:.3}/{:.3}",
                p_ns as f64 / 1_000_000.0 / ticks_nonzero as f64,
                f_ns as f64 / 1_000_000.0 / ticks_nonzero as f64,
            )
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
