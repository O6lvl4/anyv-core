//! Detect the current Rust target triple. Used by self-update to pick the
//! right release artifact, mirroring what cargo would have built natively.

/// Returns the canonical target triple of the running binary, or `None` on
/// platforms we don't release for.
pub fn target_triple() -> Option<&'static str> {
    Some(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos",   "aarch64") => "aarch64-apple-darwin",
        ("macos",   "x86_64")  => "x86_64-apple-darwin",
        ("linux",   "aarch64") => "aarch64-unknown-linux-musl",
        ("linux",   "x86_64")  => "x86_64-unknown-linux-musl",
        ("windows", "x86_64")  => "x86_64-pc-windows-msvc",
        _ => return None,
    })
}
