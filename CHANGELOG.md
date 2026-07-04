# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-07-04

The first release of **FtrIO**, attribute-based feature toggles for Rust. A faithful port of the
.NET [FtrIO](https://github.com/FtrOnOff/FtrIO) library, with
[ftrio-python](https://github.com/FtrOnOff/ftrio-python) as a second reference.

Your feature flags are a file you own, sitting next to your code — no dashboard, no SaaS, no network
call on the hot path. And with **no `appsettings.json` at all, every toggle defaults to on**, so a
service always runs.

### Highlights

- **`#[toggle]` and `#[toggle_async]` attributes** — decorate a function and it runs only when its
  toggle is on, otherwise returns `Default::default()`. Compile-time macros, the closest Rust
  analogue of the .NET AspectInjector attribute.
- **Six decision strategies**, all case-insensitive: boolean (`true`/`false`/`1`/`0`), percentage
  (`50%`), blue-green (`blue`/`green`), user targeting (`users:alice,bob`), attribute rules
  (`attribute:plan equals premium`), and A/B testing (`ab:50`, `ab:50:salt`).
- **Per-user overrides** that win unconditionally, before any strategy.
- **Byte-exact, cross-language A/B bucketing** — the same user buckets identically across the Rust,
  .NET, and Python runtimes (verified against pinned vectors).
- **Fluent builder**, an **ambient parser**, and an explicit **functional API**.
- **Pluggable value sources**: environment variables, a first-wins composite, and optional HTTP /
  Azure providers behind cargo features.
- **Write-back buffer** with atomic flushes and a final flush on drop.
- **`ftrio` CLI**: a toggle report, `export-manifest`, `release-check`, and `lint` (`FTRIO001`,
  exits non-zero on a decorated function with no config entry).

### Install

```toml
[dependencies]
ftrio = "0.1.0"
```

Or the CLI:

```bash
cargo install ftrio-cli
```

### Quickstart

```rust
use ftrio::toggle;

#[toggle] // key derives from the fn name: "send_welcome_email"
fn send_welcome_email() {
    println!("welcome!");
}
```

```json
// appsettings.json
{ "Toggles": { "send_welcome_email": true } }
```

### Crates in this release

| Crate | Purpose |
|---|---|
| `ftrio` | Core library (re-exports the macros — this is the only crate you add) |
| `ftrio-macros` | The `#[toggle]` / `#[toggle_async]` proc-macros (transitive) |
| `ftrio-cli` | The `ftrio` CLI |

### Good to know

- A gated function's return type must implement `Default` (that's the off-path value).
- A misconfiguration (missing key, unparseable value) **panics** out of the decorated function — run
  `ftrio lint` in CI to catch it at build time. The `try_*` functional API returns a `Result` if
  you'd rather handle it.
- MSRV: Rust 1.74. License: MIT.

See [`PORTING_NOTES.md`](PORTING_NOTES.md) for how each .NET mechanism maps to Rust.

[0.1.0]: https://github.com/FtrOnOff/ftrio-rust/releases/tag/v0.1.0
