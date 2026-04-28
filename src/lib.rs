//! anyv-core — shared substrate for `*v` language toolchain managers.
//!
//! This crate is the boring, battle-tested layer underneath
//! [`gv`](https://github.com/O6lvl4/gv) and
//! [`rv`](https://github.com/O6lvl4/rv): paths, presentation helpers,
//! archive extraction, GitHub-driven self-update, and argv[0] dispatch.
//!
//! It deliberately does *not* try to be a generic VM framework. The
//! per-language semantics (Gemfile vs go.mod, ruby-build vs go.dev,
//! rubygems.org vs sum.golang.org) live in each `*v-core` crate.
//! This crate covers the cross-cutting concerns that always look the
//! same regardless of language.

pub mod argv0;
pub mod extract;
pub mod fs;
pub mod paths;
pub mod presentation;
pub mod selfupdate;
pub mod target;

pub use paths::Paths;
