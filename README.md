# anyv-core

> Shared substrate for `*v` language toolchain managers.

`anyv-core` is the boring layer underneath [`gv`](https://github.com/O6lvl4/gv)
(Go) and [`rv`](https://github.com/O6lvl4/rv) (Ruby): the parts of a
toolchain manager that look identical regardless of language. By depending on
this crate you skip ~500 lines of plumbing and inherit the conventions that
already work for `gv`/`rv`.

It's deliberately **not a generic VM framework**. There's no trait-based
backend abstraction to please. The per-language semantics — Gemfile vs
`go.mod`, `ruby-build` vs `go.dev`, `rubygems.org` vs `sum.golang.org` — stay
in each `*v-core` crate. anyv-core only owns the cross-cutting concerns.

---

## What you get

| Module | Provides |
|---|---|
| `paths` | XDG-style layout with `<APP>_HOME` env override (`GV_HOME`, `RV_HOME`, …) |
| `presentation` | Spinner, ANSI colors, `humanize_bytes`, `format_duration_ms`, `plural`, `quote_sh`/`quote_ps`, `set_quiet` + `say!` macro |
| `extract` | `extract_archive(p, dest)` — auto-detects `.tar.gz` / `.zip` |
| `fs` | `dir_size` (no symlink double-counting), `walk_files` |
| `argv0` | `rewrite_for_x_dispatch("foo")` — the `foox` shim trick (`gvx` / `rvx`) |
| `target` | `target_triple()` — Rust target detection for self-update |
| `selfupdate` | `SelfUpdate { repo, bin_name, current_version }.run(&client, check)` — GitHub release fetch + sha256 verify + atomic binary replace |

## What stays in your `*v-core`

| Concern | Why language-specific |
|---|---|
| Toolchain installer | Each language has its own delivery (Go: tarball + sha from `go.dev`; Ruby: source compile via `ruby-build`; Python: `python-build-standalone`; Node: prebuilt from `nodejs.org`). |
| Package registry client | `proxy.golang.org` / `sum.golang.org`, `rubygems.org`, `pypi.org`, npm registry — all different APIs and trust models. |
| Manifest reader | `go.mod` / `go.work`, `Gemfile` / `.ruby-version`, `pyproject.toml`, `package.json` engines field. |
| Resolution chain | The priority order (env → manifest → file → global → latest) is universal *in shape*, but the file names and parsers differ. |
| Lock schema | `gv.lock` records module hashes; `rv.lock` records gem checksums. The fields aren't compatible. |

---

## Building your own `*v` from zero

Concrete walkthrough — let's invent `nv` for Node.js. The same pattern produced
both `gv` and `rv`.

### 1. Workspace skeleton

```
nv/
├── Cargo.toml          # workspace, members = [crates/nv-core, crates/nv-cli]
└── crates/
    ├── nv-core/
    │   ├── Cargo.toml  # depends on anyv-core
    │   └── src/
    │       ├── lib.rs
    │       ├── paths.rs       # 6 lines — see below
    │       ├── manifest.rs    # parses package.json `engines.node`, .nvmrc
    │       ├── resolve.rs     # the resolution chain
    │       ├── install.rs     # downloads from nodejs.org
    │       ├── registry.rs    # name → npm package map
    │       ├── npm.rs         # npm registry client
    │       ├── lock.rs        # nv.lock schema
    │       └── tool.rs        # global npm-tool installs
    └── nv-cli/
        ├── Cargo.toml  # depends on nv-core + anyv-core
        └── src/main.rs
```

### 2. `nv-core/Cargo.toml`

```toml
[package]
name = "nv-core"
version = "0.1.0"
edition = "2021"

[dependencies]
anyv-core = { git = "https://github.com/O6lvl4/anyv-core", tag = "v0.1.0" }
anyhow = "1"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
tokio = { version = "1", features = ["full"] }
```

### 3. `paths.rs` — the entire file

```rust
//! nv's filesystem paths come from anyv-core, parameterized with "nv".
pub use anyv_core::paths::{ensure_dir, Paths as AnyvPaths};
use anyhow::Result;

pub type Paths = AnyvPaths;

pub fn discover() -> Result<Paths> {
    AnyvPaths::discover("nv")
}
```

That's it. You now have `~/.local/share/nv/`, `~/.config/nv/`, `~/.cache/nv/`,
plus `versions/`, `store/`, `tools/` — all wired up — and the `NV_HOME` env
override for sandboxed tests.

### 4. `nv-cli/main.rs` — the imports

```rust
use anyv_core::fs::dir_size;
use anyv_core::presentation::{
    bold as color_bold, cyan as color_cyan, dim, format_duration_ms,
    green as color_green, humanize_bytes as humanize, plural, quote_ps, quote_sh,
    set_quiet, spinner, success_mark, yellow as color_yellow,
};
use anyv_core::say;
use anyv_core::selfupdate::{Outcome, SelfUpdate};
use clap::{Parser, Subcommand};
```

### 5. `main()` — argv[0] dispatch + quiet flag

```rust
fn main() -> ExitCode {
    // `nvx` shim: when invoked under that name, prepend `x` to argv.
    let cli = match anyv_core::argv0::rewrite_for_x_dispatch("nv") {
        Some(rewritten) => Cli::parse_from(rewritten),
        None => Cli::parse(),
    };
    set_quiet(cli.quiet);
    // …rest of main…
}
```

After this, the `nvx node-something` command is a one-line `Cli` variant
(`X { argv: Vec<String> }`) plus a `cmd_x` that invokes whatever your tool
runner does.

### 6. `cmd_self_update` — the entire body

```rust
async fn cmd_self_update(check: bool) -> Result<ExitCode> {
    let updater = SelfUpdate {
        repo: "yourname/nv",
        bin_name: "nv",
        current_version: env!("CARGO_PKG_VERSION"),
    };
    let info = updater.run(&http_client()?, check).await?;
    match info.outcome {
        Outcome::AlreadyUpToDate => println!(
            "{} nv is already up to date {}",
            success_mark(),
            dim(&format!("(installed: {}, latest: {})", info.current, info.latest))
        ),
        Outcome::NewerAvailable => println!(
            "{} a newer release is available: {} → {}",
            success_mark(), info.current, color_bold(&info.latest)
        ),
        Outcome::Updated => println!(
            "{} nv {} → {}",
            success_mark(), dim(&info.current), color_bold(&info.latest)
        ),
    }
    Ok(ExitCode::SUCCESS)
}
```

Replaces ~120 lines of GitHub API + sha-verify + extract + atomic replace.

### 7. `spinner` + `say!` for output

```rust
let pb = spinner(format!("installing node {version}"));
let result = node_install::install(paths, version)?;
pb.finish_and_clear();
say!("{} installed node {}", success_mark(), result.version);
```

`pb` is `ProgressBar::hidden()` under `--quiet`, so you don't have to branch.
`say!` is silent under `--quiet`. Both no-ops are free.

### 8. The language-specific parts (the work that's actually yours)

These are the modules you write:

- **`install.rs`**: download Node tarball from `nodejs.org/dist/vX.Y.Z/`,
  verify sha256 from `SHASUMS256.txt`, extract via
  `anyv_core::extract::extract_archive`, link into
  `<paths.versions()>/X.Y.Z`.
- **`manifest.rs`**: walk up looking for `package.json` (read `engines.node`)
  and `.nvmrc`. Return a `VersionHit { version, source, origin }`.
- **`resolve.rs`**: env `NV_VERSION` → manifest → `~/.config/nv/global` →
  latest installed.
- **`npm.rs`**: hit `https://registry.npmjs.org/<pkg>/latest` for tool
  metadata.
- **`tool.rs`**: `npm install -g --prefix <per-tool-dir> <pkg>@<ver>` and
  record the result in `nv.lock`.
- **`registry.rs`**: a static list of `("eslint", "eslint", "eslint")` etc.
  for the short-form `nv tool add eslint`.

---

## Module reference

### `paths`

```rust
pub fn Paths::discover(app: &'static str) -> Result<Paths>;
pub fn ensure_dir(p: &Path) -> Result<()>;

impl Paths {
    pub fn store(&self) -> PathBuf;       // <data>/store
    pub fn versions(&self) -> PathBuf;    // <data>/versions
    pub fn version_dir(&self, v: &str) -> PathBuf;
    pub fn tools(&self) -> PathBuf;       // <data>/tools
    pub fn global_version_file(&self) -> PathBuf; // <config>/global
    pub fn ensure_dirs(&self) -> Result<()>;
}
```

`Paths::discover("foo")` reads `FOO_HOME` first, then falls back to:

- macOS: `~/Library/Application Support/dev.O6lvl4.foo/`
- Linux: `~/.local/share/foo/`, `~/.config/foo/`, `~/.cache/foo/`
- Windows: `%APPDATA%\O6lvl4\foo\` family

### `presentation`

```rust
pub fn set_quiet(q: bool);
pub fn is_quiet() -> bool;
pub fn spinner(msg: impl Into<String>) -> ProgressBar;

pub fn success_mark() -> &'static str;
pub fn dim(s: &str) -> String;
pub fn green(s: &str) -> String;
pub fn yellow(s: &str) -> String;
pub fn cyan(s: &str) -> String;
pub fn bold(s: &str) -> String;

pub fn plural(n: usize) -> &'static str;
pub fn format_duration_ms(ms: u128) -> String;     // "234ms" / "4.56s" / "2m07s"
pub fn humanize_bytes(bytes: u64) -> String;       // "893 MiB"
pub fn quote_sh(s: &str) -> String;
pub fn quote_ps(s: &str) -> String;
```

The `say!` macro is exported at the crate root (`anyv_core::say!`). It expands
to `println!` when `is_quiet()` is false, otherwise nothing.

### `extract`

```rust
pub fn extract_archive(archive: &Path, dest: &Path) -> Result<()>;
pub fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<()>;
pub fn extract_zip(archive: &Path, dest: &Path) -> Result<()>;
```

`extract_archive` dispatches by filename extension (`.zip` → zip, anything else
→ tar+gzip). On unix the zip path also restores the unix-mode bits so
binaries stay executable.

### `fs`

```rust
pub fn dir_size(path: &Path) -> Result<(u64, usize)>;
pub fn walk_files(root: &Path, max_depth: usize) -> Vec<PathBuf>;
```

`dir_size` returns `(bytes, top_level_entry_count)`. Symlinks are *not*
followed and their target sizes don't count — important for `cache info` /
`cache prune` to avoid double-counting a content-addressed store that's
referenced from `versions/`.

`walk_files` skips `.git`, `node_modules`, `vendor`, dotfile dirs.

### `argv0`

```rust
pub fn rewrite_for_x_dispatch(app: &str) -> Option<Vec<OsString>>;
```

When invoked as `<app>x` (e.g. `gvx`, `rvx`), returns
`["<app>", "x", ...rest]`. Caller passes the result to `Cli::parse_from`.
Returns `None` for the canonical name so the unmodified argv is parsed.

`install.sh` typically `ln -sfn nv $INSTALL_DIR/nvx`; the same binary handles
both names.

### `target`

```rust
pub fn target_triple() -> Option<&'static str>;
```

Returns one of `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`aarch64-unknown-linux-musl`, `x86_64-unknown-linux-musl`,
`x86_64-pc-windows-msvc`, or `None`.

### `selfupdate`

```rust
pub struct SelfUpdate {
    pub repo: &'static str,            // "yourname/nv"
    pub bin_name: &'static str,        // "nv"
    pub current_version: &'static str, // env!("CARGO_PKG_VERSION")
}

pub enum Outcome { AlreadyUpToDate, NewerAvailable, Updated }

pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub outcome: Outcome,
    pub binary_path: Option<PathBuf>,
}

impl SelfUpdate {
    pub async fn latest_tag(&self, client: &reqwest::Client) -> Result<String>;
    pub async fn run(&self, client: &reqwest::Client, check_only: bool) -> Result<UpdateInfo>;
}
```

Expects release artifacts at
`https://github.com/<repo>/releases/download/<tag>/<bin_name>-<tag>-<triple>.tar.gz`
(`.zip` on Windows) plus a sibling `.sha256`. That layout is what gv's and
rv's `release.yml` produce, so if you copy their workflow you're done.

Atomic-replace handles the running-binary problem: rename across inode on
unix, rename-aside-then-move on Windows.

---

## Conventions you inherit by depending on anyv-core

These aren't enforced by code, but if you follow them your CLI will feel
identical to `gv`/`rv` and users won't have to learn a second mental model:

1. **Subcommand layout**: `install`, `list (--remote)`, `current`, `which`,
   `use-global`, `run`, `add tool <spec>` (alias for `tool add`),
   `tool {list, registry, add, remove}`, `sync (--frozen)`, `init`,
   `tree`, `outdated`, `upgrade`, `lock`, `cache {info, prune}`,
   `dir <kind>`, `uninstall`, `env (--shell)`, `self-update (--check)`,
   `completions <shell>`, `doctor`, `x` (with `<app>x` shim).

2. **Project file**: `<app>.toml` at the project root, with `[<lang>]` for
   the toolchain version and `[tools]` for pinned utilities.

3. **Lock file**: `<app>.lock` next to the project file. `--frozen` mode
   refuses network resolution and uses lock-as-truth.

4. **Resolution chain order**: env var → language manifest → tool-specific
   version file → user global → latest installed.

5. **Output conventions**: `✓` for completed actions, `+` for new items,
   `~` for changed, `=` for unchanged, `-` for pruned. `--quiet` keeps the
   diff lines but suppresses spinners and summary banners.

6. **Tool sharing across projects**: per-toolchain + per-tool-version
   directory under `<paths.tools()>/<toolchain-version>/<tool>/<tool-version>/`.
   Different projects can pin different rubocop versions without colliding.

---

## Versioning

`anyv-core` follows semver. Breaking changes (e.g. a `Paths::ensure_dirs`
signature change) bump the minor version pre-1.0. Pin via tag, not branch:

```toml
anyv-core = { git = "https://github.com/O6lvl4/anyv-core", tag = "v0.1.0" }
```

When you upgrade, the changelog will tell you which call sites in your
`*v-core` need adjusting. This crate's audience is small (maintainers of
sibling `*v` tools), so breaking changes are routine and well-signaled.

---

## Reference implementations

If a section above feels abstract, read these end-to-end — they're each
under 1500 lines and anyv-core slots in roughly the same way:

- [O6lvl4/gv](https://github.com/O6lvl4/gv) — Go (`go.mod`,
  `go.dev` releases, `proxy.golang.org` + `sum.golang.org`).
  [v0.6.0 commit moving to anyv-core](https://github.com/O6lvl4/gv/commit/92364be)
  is the cleanest before/after diff: −425 lines, +110 lines.
- [O6lvl4/rv](https://github.com/O6lvl4/rv) — Ruby (`Gemfile` /
  `.ruby-version`, `ruby-build`, `rubygems.org`).
  [v0.2.0 commit moving to anyv-core](https://github.com/O6lvl4/rv/commit/cf1ade1).

## License

MIT
