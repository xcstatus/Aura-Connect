//! Single logging stack: **`tracing` subscriber + `RUST_LOG`** (same filter syntax as `env_logger`).
//! Existing `log::` calls are forwarded automatically: **`tracing_subscriber::SubscriberInitExt::try_init`**
//! (with the crate default `tracing-log` feature) installs the log bridge—do not call `LogTracer::init` again.
//!
//! With **`--features term-prof`**, a `tracing_flame` layer is composed on the same registry so console
//! output and folded stacks coexist.

#[cfg(feature = "term-prof")]
use tracing_subscriber::layer::SubscriberExt;
#[cfg(feature = "term-prof")]
use tracing_subscriber::util::SubscriberInitExt;

/// Hold flush guard(s) so flame output is written on drop.
#[cfg(feature = "term-prof")]
pub struct LoggingGuard {
    _flame: tracing_flame::FlushGuard<std::io::BufWriter<std::fs::File>>,
}

#[cfg(not(feature = "term-prof"))]
pub struct LoggingGuard;

#[cfg(feature = "term-prof")]
impl Drop for LoggingGuard {
    fn drop(&mut self) {
        let (text, bg, rows, runs, cells) = crate::prof::term_counters::snapshot();
        eprintln!(
            "[term-prof] paint counters: text_calls={} bg_rects={} rows_drawn={} runs_drawn={} cells_drawn={}",
            text, bg, rows, runs, cells
        );
        tracing::info!(
            target: "term-prof",
            text_calls = text,
            bg_rects = bg,
            rows_drawn = rows,
            runs_drawn = runs,
            cells_drawn = cells,
            "[term-prof] paint counters (shutdown)"
        );
    }
}

/// Install global tracing subscriber (`try_init` also wires `log` → `tracing`).
pub fn init() -> anyhow::Result<LoggingGuard> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    #[cfg(feature = "term-prof")]
    {
        let (flame_layer, flame_guard) = tracing_flame::FlameLayer::with_file("term.folded")
            .map_err(|e| anyhow::anyhow!("tracing_flame output file: {e}"))?;

        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_target(true);

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .with(flame_layer)
            .try_init()
            .map_err(|e| anyhow::anyhow!("tracing subscriber (term-prof): {e}"))?;

        crate::prof::spawn_term_prof_heartbeat_thread();

        return Ok(LoggingGuard {
            _flame: flame_guard,
        });
    }

    #[cfg(not(feature = "term-prof"))]
    {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .with_target(true)
            .try_init()
            .map_err(|e| anyhow::anyhow!("tracing subscriber: {e}"))?;

        Ok(LoggingGuard)
    }
}
