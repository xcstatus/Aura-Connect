//! Centralized terminal diagnostics env switches.
//!
//! Naming convention:
//! - Master switch: `RUST_SSH_TERM_DIAG=1`
//! - Scoped switch: `RUST_SSH_TERM_DIAG_<SCOPE>=1`
//!   e.g. `RUST_SSH_TERM_DIAG_HIT_TEST=1`

fn env_truthy(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|v| {
            let t = v.trim().to_ascii_lowercase();
            matches!(t.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

/// Global/Scoped diagnostic gate.
pub fn enabled(scope: &str) -> bool {
    if env_truthy("RUST_SSH_TERM_DIAG") {
        return true;
    }
    let key = format!("RUST_SSH_TERM_DIAG_{}", scope.trim().to_ascii_uppercase());
    env_truthy(&key)
}

