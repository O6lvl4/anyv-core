# anyv-core

> Shared substrate for `*v` language toolchain managers.

`anyv-core` is the boring, battle-tested layer underneath
[`gv`](https://github.com/O6lvl4/gv) (Go) and
[`rv`](https://github.com/O6lvl4/rv) (Ruby). It collects the cross-cutting
concerns that always look the same regardless of language:

- `paths` — XDG layout with `<APP>_HOME` env override
- `presentation` — spinners, ANSI colors, humanized sizes/durations,
  `say!` macro gated by `--quiet`
- `extract` — `.tar.gz` and `.zip` archive unpack with the same entry point
- `fs` — disk-walk + size accounting (no symlink double-counting)
- `argv0` — `gvx` / `rvx` shim trick: rewrite argv when invoked under a
  non-canonical name
- `target` — Rust target-triple detection
- `selfupdate` — GitHub-driven self-update with sha256 verification and
  atomic binary replace, generic over the binary name

Per-language semantics (Gemfile vs `go.mod`, `ruby-build` vs `go.dev`,
`rubygems.org` vs `sum.golang.org`) live in each `*v-core` crate. This
crate deliberately stays at the substrate level — not a generic VM
framework.

## Usage

```toml
[dependencies]
anyv-core = { git = "https://github.com/O6lvl4/anyv-core", tag = "v0.1.0" }
```

## License

MIT
