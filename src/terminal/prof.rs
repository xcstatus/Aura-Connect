#[cfg(feature = "term-prof")]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "term-prof")]
use std::time::Duration;

#[cfg(feature = "term-prof")]
pub mod term_counters {
    use super::*;

    // Counters are intentionally coarse and lock-free.
    pub static TEXT_CALLS: AtomicU64 = AtomicU64::new(0);
    pub static BG_RECTS: AtomicU64 = AtomicU64::new(0);
    pub static ROWS_DRAWN: AtomicU64 = AtomicU64::new(0);
    pub static RUNS_DRAWN: AtomicU64 = AtomicU64::new(0);
    pub static CELLS_DRAWN: AtomicU64 = AtomicU64::new(0);

    pub fn add_text_calls(n: u64) {
        TEXT_CALLS.fetch_add(n, Ordering::Relaxed);
    }
    pub fn add_bg_rects(n: u64) {
        BG_RECTS.fetch_add(n, Ordering::Relaxed);
    }
    pub fn add_rows(n: u64) {
        ROWS_DRAWN.fetch_add(n, Ordering::Relaxed);
    }
    pub fn add_runs(n: u64) {
        RUNS_DRAWN.fetch_add(n, Ordering::Relaxed);
    }
    pub fn add_cells(n: u64) {
        CELLS_DRAWN.fetch_add(n, Ordering::Relaxed);
    }

    pub fn snapshot() -> (u64, u64, u64, u64, u64) {
        (
            TEXT_CALLS.load(Ordering::Relaxed),
            BG_RECTS.load(Ordering::Relaxed),
            ROWS_DRAWN.load(Ordering::Relaxed),
            RUNS_DRAWN.load(Ordering::Relaxed),
            CELLS_DRAWN.load(Ordering::Relaxed),
        )
    }
}

/// Background counter deltas while profiling (macOS may skip `Drop` on abrupt exit).
#[cfg(feature = "term-prof")]
pub(crate) fn spawn_term_prof_heartbeat_thread() {
    std::thread::spawn(|| {
        let mut last = (0u64, 0u64, 0u64, 0u64, 0u64);
        loop {
            std::thread::sleep(Duration::from_secs(2));
            let now = term_counters::snapshot();
            let delta_cells = now.4.saturating_sub(last.4);
            let delta_text = now.0.saturating_sub(last.0);
            if delta_cells >= 10_000 || delta_text >= 5_000 {
                eprintln!(
                    "[term-prof] paint counters (delta): text_calls=+{} bg_rects=+{} rows_drawn=+{} runs_drawn=+{} cells_drawn=+{}",
                    now.0.saturating_sub(last.0),
                    now.1.saturating_sub(last.1),
                    now.2.saturating_sub(last.2),
                    now.3.saturating_sub(last.3),
                    delta_cells,
                );
                last = now;
            }
        }
    });
}
