//! 动画缓动函数与插值引擎。
//!
//! 规范见 `doc/动画使用规范.md`。
//!
//! 所有函数接收 `t ∈ [0, 1]`，返回 `f32 ∈ [0, 1]`。
//! `t=0` 表示动画开始，`t=1` 表示动画结束。

use iced::Color;

/// 标准 ease-out：快速启动，慢速收尾。
///
/// 适用于：按钮悬停进入、模态打开、颜色渐变进入。
#[inline]
pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// 标准 ease-in：慢速启动，快速收尾。
///
/// 适用于：按钮按下释放、模态关闭退出。
#[inline]
pub fn ease_in(t: f32) -> f32 {
    t.clamp(0.0, 1.0).powi(3)
}

/// 标准 ease-in-out：两头慢中间快。
///
/// 适用于：Tab 切换、内容切换、状态变化。
#[inline]
pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t.powi(3)
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// 线性插值。
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

/// 颜色插值。
#[inline]
pub fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: lerp(from.r, to.r, t),
        g: lerp(from.g, to.g, t),
        b: lerp(from.b, to.b, t),
        a: lerp(from.a, to.a, t),
    }
}

/// 根据已流逝的 tick 数计算当前动画进度 `t ∈ [0, 1]`。
///
/// - `enter_tick`: 进入动画时的 tick_count。
/// - `current_tick`: 当前帧的 tick_count。
/// - `tick_ms`: 每 tick 的毫秒数（通过帧间隔推算）。
/// - `duration_ms`: 动画总时长毫秒。
///
/// 返回值始终 clamp 到 [0.0, 1.0]。
#[inline]
pub fn anim_t(enter_tick: u64, current_tick: u64, tick_ms: f32, duration_ms: f32) -> f32 {
    if duration_ms <= 0.0 {
        return 1.0;
    }
    let elapsed_ms = (current_tick.saturating_sub(enter_tick)) as f32 * tick_ms;
    (elapsed_ms / duration_ms).min(1.0)
}

/// 动画是否已完成。
#[inline]
pub fn anim_done(enter_tick: u64, current_tick: u64, tick_ms: f32, duration_ms: f32) -> bool {
    anim_t(enter_tick, current_tick, tick_ms, duration_ms) >= 1.0
}

/// 带反向动画的进度计算。
///
/// 当 `reverse=true` 时，返回 `1.0 - t`，用于退出动画。
#[inline]
pub fn anim_t_bidir(
    enter_tick: u64,
    current_tick: u64,
    tick_ms: f32,
    duration_ms: f32,
    reverse: bool,
) -> f32 {
    let t = anim_t(enter_tick, current_tick, tick_ms, duration_ms);
    if reverse { 1.0 - t } else { t }
}
