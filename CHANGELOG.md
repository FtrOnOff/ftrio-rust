# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2], 2026-07-08

Cross-port conformance and a config-lookup correctness fix.

### Changed

- Config keys now match case-insensitively, mirroring .NET's `Microsoft.Extensions.Configuration`
  (`OrdinalIgnoreCase`). A lookup of `newcheckout` resolves a config key written `NewCheckout`, and
  colon paths like `FtrIO:BlueGreen:CurrentSlot` match regardless of case; an exact match is tried
  first as a fast path. Brings the Rust port in line with the .NET and Python runtimes.

### Added

- Hidden `ftrio conformance-resolve` subcommand: the per-port hook for the language-agnostic
  `ftrio-conformance` suite. It reads one resolution case as JSON on stdin, runs it through the real
  resolution path, and prints `{"result": true|false}` or `{"error": "..."}`, so a cross-port driver
  can build a parity matrix across the Rust, .NET, and Python runtimes.

## [0.1.1], 2026-07-04

Correctness release: no API or behavior changes.

### Fixed

- Corrected the declared MSRV. `rust-version` was `1.74`, but the resolved dependency tree requires
  a newer toolchain, `clap` and `getrandom 0.4` use edition 2024 (rustc ≥ 1.85), and the `icu_*`
  crates (via `ureq → url → idna`) declare a floor of **1.86**. Bumped `rust-version` to `1.86`
  across the workspace and the standalone playground, so an older toolchain now gets a clear
  "requires rustc 1.86" error instead of a confusing edition-2024 failure in a transitive dependency.

### Added

- A CI `msrv` job that builds the workspace (all features) on exactly Rust 1.86, so the MSRV claim is
  verified on every push and cannot silently drift as dependencies bump.

## [0.1.0], 2026-07-04

The first release of **FtrIO**, attribute-based feature toggles for Rust. A faithful port of the
.NET [FtrIO](https://github.com/FtrOnOff/FtrIO) library, with
[ftrio-python](https://github.com/FtrOnOff/ftrio-python) as a second reference.

Your feature flags are a file you own, sitting next to your code, no dashboard, no SaaS, no network
call on the hot path. And with **no `appsettings.json` at all, every toggle defaults to on**, so a
service always runs.

### Highlights

- **`#[toggle]` and `#[toggle_async]` attributes**, decorate a function and it runs only when its
  toggle is on, otherwise returns `Default::default()`. Compile-time macros, the closest Rust
  analogue of the .NET AspectInjector attribute.
- **Six decision strategies**, all case-insensitive: boolean (`true`/`false`/`1`/`0`), percentage
  (`50%`), blue-green (`blue`/`green`), user targeting (`users:alice,bob`), attribute rules
  (`attribute:plan equals premium`), and A/B testing (`ab:50`, `ab:50:salt`).
- **Per-user overrides** that win unconditionally, before any strategy.
- **Byte-exact, cross-language A/B bucketing**, the same user buckets identically across the Rust,
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
| `ftrio` | Core library (re-exports the macros, this is the only crate you add) |
| `ftrio-macros` | The `#[toggle]` / `#[toggle_async]` proc-macros (transitive) |
| `ftrio-cli` | The `ftrio` CLI |

### Good to know

- A gated function's return type must implement `Default` (that's the off-path value).
- A misconfiguration (missing key, unparseable value) **panics** out of the decorated function, run
  `ftrio lint` in CI to catch it at build time. The `try_*` functional API returns a `Result` if
  you'd rather handle it.
- MSRV: Rust 1.86 (the dependency tree requires edition 2024). License: MIT.

See [`PORTING_NOTES.md`](PORTING_NOTES.md) for how each .NET mechanism maps to Rust.

[0.1.2]: https://github.com/FtrOnOff/ftrio-rust/releases/tag/v0.1.2
[0.1.1]: https://github.com/FtrOnOff/ftrio-rust/releases/tag/v0.1.1
[0.1.0]: https://github.com/FtrOnOff/ftrio-rust/releases/tag/v0.1.0
