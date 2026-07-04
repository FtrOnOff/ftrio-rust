# FtrIO (Rust)

Attribute-based feature toggles for Rust — a faithful port of the .NET
[FtrIO](https://github.com/FtrOnOff/FtrIO) library, with the
[Python port](https://github.com/FtrOnOff/ftrio-python) as a second reference.

The flags are a file you own, sitting right next to your code. No dashboard, no SaaS, no network
round-trip on the hot path: **radical ownership** of your feature toggles.

## The smallest example

Decorate a function with `#[toggle]`. It runs only when its toggle is on; otherwise it returns
`Default::default()`.

```rust
use ftrio::toggle;

#[toggle] // key derives from the fn name: "send_welcome_email"
fn send_welcome_email() {
    println!("welcome!");
}
```

```json
// appsettings.json
{
  "Toggles": {
    "send_welcome_email": true
  }
}
```

With **no `appsettings.json` on disk at all**, every toggle defaults to **on** — the offline-safe
default, so a service always runs.

## Installation

```bash
cargo add ftrio
```

Optional providers are behind features, mirroring the separate .NET provider projects:

```bash
cargo add ftrio --features http    # HttpToggleParser
cargo add ftrio --features azure   # AzureAppConfigToggleParser
```

## The attribute

`#[toggle]` and `#[toggle_async]` are procedural attribute macros — compile-time code
transformation, the closest analogue of the .NET AspectInjector attribute of any target language.

```rust
use ftrio::{toggle, toggle_async};

#[toggle(key = "SendWelcomeEmail")] // explicit key overrides the derived one
fn some_function() -> i32 {
    42
}

#[toggle_async] // gating runs synchronously at call time; the result is awaitable either way
async fn refresh_cache() -> usize {
    // ... async work ...
    128
}
```

Because the off-path returns `Default::default()`, **a gated function's return type must implement
`Default`** (`()`, `Option<T>`, integers, `String`, etc.). A misconfiguration (missing key,
unparseable value) panics out of the decorated function — the same way the .NET woven aspect throws.
The `ftrio lint` step is there to catch that at build time.

## The builder pipeline

Assemble a parser with the strategies you want, then install it as the ambient instance:

```rust
use std::sync::Arc;
use ftrio::{toggle_parser_provider, ToggleParserBuilder};

let parser = ToggleParserBuilder::new()
    .with_base_path("appsettings.json")
    .with_percentage_rollout()
    .with_blue_green(Some("green".into()), vec!["blue".into(), "green".into()])
    .with_context_strategies() // user targeting, attribute rules, A/B testing
    .with_overrides()          // requires a context accessor
    .with_context_accessor(Arc::new(MyContext))
    .build()
    .expect("with_overrides needs a context accessor");

toggle_parser_provider::configure(Arc::new(parser));
```

The value grammar (all case-insensitive):

| Grammar | Example | Strategy |
|---|---|---|
| boolean | `true`, `false`, `1`, `0` | `BooleanStrategy` |
| percentage | `50%` | `PercentageRolloutStrategy` |
| slot | `blue`, `green` | `BlueGreenStrategy` |
| user list | `users:alice,bob` | `UserTargetingStrategy` |
| attribute rule | `attribute:plan equals premium` | `AttributeRuleStrategy` |
| A/B | `ab:50`, `ab:50:round2` | `AbTestStrategy` |

Resolution order: no config file → `true`; then a per-user `TogglesOverrides` entry wins
unconditionally; otherwise the first strategy whose grammar matches decides, with `BooleanStrategy`
always last.

## Providers and the buffer model

- `AppSettingsToggleParser` — the `appsettings.json` file reader.
- `EnvironmentVariableToggleParser` — reads `FTRIO__Toggles__<Key>`.
- `CompositeToggleParser` — tries several sources in order, first-wins.
- `ToggleProviderBuffer` — stages toggle writes (`Mutex<HashMap>`, last-write-wins), flushes on a
  background interval thread with an atomic temp-file-plus-rename, and performs a **final flush on
  `Drop`**.

## The `ftrio` CLI

```bash
ftrio                              # toggle report: cross-reference code against appsettings*.json
ftrio --env Production --markdown report.md
ftrio export-manifest --pretty     # write toggles.manifest.json
ftrio release-check --manifest toggles.manifest.json --config appsettings.json
ftrio lint                         # FTRIO001: decorated fns with no Toggles entry (exits non-zero)
```

`ftrio lint` is the port of the Roslyn analyzer `ToggleConfigAnalyzer`: it walks `.rs` files with
`syn`, resolves each `#[toggle]` key, and fails the build if the key is missing from `Toggles`.

## Configuration

`appsettings.json` sections: `FtrIO` (settings), `Toggles` (key → value), `TogglesOverrides`
(`toggle_key → { user_id → bool }`).

`FtrIO` keys: `ReloadOnChange` (bool), `FlushInterval` (int seconds, default 5), `Environment`
(string), `BlueGreen:CurrentSlot`, `BlueGreen:KnownSlots` (comma-separated). Environment resolution:
`FtrIO:Environment`, then `ASPNETCORE_ENVIRONMENT`, then `DOTNET_ENVIRONMENT`, then (additive)
`FTRIO_ENVIRONMENT`.

## Playground

`ftrio-playground` is a standalone, educational crate (not part of the published workspace, never
published). It consumes `ftrio` the way a real user would, so run it from its own directory:

```bash
cd ftrio-playground
cargo run                # infinite loop, cycling users every 2s; Ctrl+C to exit
cargo run -- --no-config # offline-safe default: everything on (one-shot)
```

It ships its own `appsettings.json` next to the code (with `ReloadOnChange` on, so you can edit it
live) and prints, for each gated function, the key, its raw value, the resolved decision, and the
context used — then runs (or skips) the decorated body.

## Development

```bash
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings -D clippy::upper_case_acronyms
cargo fmt --all --check
```

These gates are the Rust analogue of the style-guide conformance the Python port enforced with
`ruff`. Acronyms are one word (`FtrIo`, `AbTestStrategy`, `HttpToggleParser`).

## Releasing and changelog

The `export-manifest` → `release-check` pair is a cross-tool contract: export a manifest of the
toggles your code uses, then gate a release on every one of them existing in the target environment's
config (exit `0` ready, `1` blocked, `2` manifest error, `3` config error).

See [`PORTING_NOTES.md`](PORTING_NOTES.md) for the full record of how each .NET mechanism maps to
Rust.
