//! Output styling helpers shared by every `*v` CLI: spinners, ANSI colors,
//! humanized sizes and durations, shell-quoted strings, plural endings, and
//! a `--quiet` gate.
//!
//! The quiet flag is process-global because every entry point reads the same
//! single `cli.quiet` value once.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

static QUIET: AtomicBool = AtomicBool::new(false);

pub fn set_quiet(q: bool) { QUIET.store(q, Ordering::Relaxed); }
pub fn is_quiet() -> bool { QUIET.load(Ordering::Relaxed) }

/// Print only when not in `--quiet` mode. Equivalent to `println!` otherwise.
#[macro_export]
macro_rules! say {
    ($($arg:tt)*) => {{
        if !$crate::presentation::is_quiet() {
            println!($($arg)*);
        }
    }};
}

/// A green-tick spinner with a steady tick. Hidden under `--quiet` so the
/// caller doesn't have to branch.
pub fn spinner(msg: impl Into<String>) -> ProgressBar {
    if is_quiet() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("  {spinner:.green} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(msg.into());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

pub fn success_mark() -> &'static str { "\x1b[32m✓\x1b[0m" }
pub fn dim(s: &str) -> String { format!("\x1b[2m{s}\x1b[0m") }
pub fn green(s: &str) -> String { format!("\x1b[32m{s}\x1b[0m") }
pub fn yellow(s: &str) -> String { format!("\x1b[33m{s}\x1b[0m") }
pub fn cyan(s: &str) -> String { format!("\x1b[36m{s}\x1b[0m") }
pub fn bold(s: &str) -> String { format!("\x1b[1m{s}\x1b[0m") }

pub fn plural(n: usize) -> &'static str { if n == 1 { "" } else { "s" } }

pub fn format_duration_ms(ms: u128) -> String {
    if ms < 1_000 { format!("{ms}ms") }
    else if ms < 60_000 { format!("{:.2}s", ms as f64 / 1_000.0) }
    else { let s = ms / 1_000; format!("{}m{:02}s", s / 60, s % 60) }
}

pub fn humanize_bytes(bytes: u64) -> String {
    const U: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    if bytes == 0 { return "0 B".into(); }
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if v >= 100.0 || i == 0 {
        format!("{:.0} {}", v, U[i])
    } else {
        format!("{:.1} {}", v, U[i])
    }
}

/// POSIX-shell-quote a single argument (single-quote, escape embedded
/// quotes). Useful for emitting `eval "$(rv env)"`-style output.
pub fn quote_sh(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// PowerShell single-quote (double up embedded single quotes).
pub fn quote_ps(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}
