//! argv[0] dispatch — the `gvx`/`rvx` shim trick. When a CLI is invoked under
//! a non-canonical name, rewrite the argv list before clap parses it so the
//! same binary handles "ephemeral run" without a separate executable.

use std::ffi::OsString;
use std::path::Path;

/// If the binary was invoked as `<app>x` (e.g. `gvx`, `rvx`), return the
/// rewritten argv with `[<app>, "x", ...rest]`. Otherwise return `None` and
/// the caller proceeds with the unmodified argv.
///
/// Example:
///   binary `gv`, invoked as `gvx golangci-lint run` → returns
///   `Some(["gv", "x", "golangci-lint", "run"])`.
pub fn rewrite_for_x_dispatch(app: &str) -> Option<Vec<OsString>> {
    let argv0 = std::env::args_os().next()?;
    let stem = Path::new(&argv0).file_stem()?.to_string_lossy().into_owned();
    let expected = format!("{app}x");
    if stem != expected {
        return None;
    }
    let mut out: Vec<OsString> = Vec::with_capacity(std::env::args_os().len() + 1);
    out.push(OsString::from(app));
    out.push(OsString::from("x"));
    for a in std::env::args_os().skip(1) {
        out.push(a);
    }
    Some(out)
}
