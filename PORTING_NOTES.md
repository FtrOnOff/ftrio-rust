# Porting notes: FtrIO (.NET) → Rust

This is the port log, in the same discipline as the Python port's file. Guiding rule: reproduce
FtrIO's **behaviour** exactly; where a 1:1 translation is impossible, use the closest idiomatic Rust
substitute and record it here with a one-line justification.

## Interface → trait rename table

| C# interface / type | Rust trait / type | Note |
|---|---|---|
| `IToggleParser` | `ToggleParser` (trait) | default `get_override` returns `None` |
| `ToggleParser` (concrete) | `AppSettingsToggleParser` | rename says what it reads |
| `IToggleDecisionStrategy` | `ToggleDecisionStrategy` (trait) | `can_handle` + `should_execute` |
| `IToggleValueProvider` | `ToggleValueProvider` (trait) | pluggable raw-value source |
| `IToggleBuffer` | `ToggleBuffer` (trait) | staged writes + flush |
| `IFtrIOContextAccessor` | `FtrIoContextAccessor` (trait) | acronym-as-one-word casing |
| `IFeatureToggle<T>` / `FeatureToggle<T>` | free fns `execute_if_toggle_on<T: Default>` etc. | generics via type params; nameless-closure caveat |

Additional named components kept from the source: `OverrideResolver`, `StrategyToggleParser`,
`ToggleParserBuilder`, `CompositeToggleParser`, `EnvironmentVariableToggleParser`,
`ToggleProviderBuffer`, `ToggleParserProvider` (module `toggle_parser_provider`).

## `[Toggle]` / `[ToggleAsync]` → `#[toggle]` / `#[toggle_async]` proc-macros

This is the **closest analogue of the three ports**. .NET uses AspectInjector IL weaving (runtime
code transformation); Python uses a runtime decorator (runtime wrapping); Rust uses a procedural
attribute macro — **compile-time** code transformation. The attribute-based toggle is thus rendered
*natively*, not by substitution.

- **Off-path return is `Default::default()`.** The faithful equivalent of the aspect returning
  `default(T)` (null/zero) and Python returning `None`. Documented constraint: **a gated function's
  return type must implement `Default`** (`()` for void-like fns, `None` for `Option<T>`, `0` for
  integers, etc.).
- **Panic-on-error mapping.** `get_toggle_status` returns `Result<bool, ToggleError>`; the macro
  `unwrap`s it, so a missing key or unparseable value panics out of the decorated function — faithful
  to the woven aspect throwing an exception out of the decorated method. Misconfiguration is a
  programmer error `ftrio lint` catches at build time, so a panic is the idiomatic and faithful
  mapping. The functional `try_*` API exposes the `Result` for callers who want to handle it.
- **`#[toggle_async]` checks at call time.** It rewrites `async fn f(..) -> T` into a synchronous
  `fn f(..) -> impl Future<Output = T>` that runs the gating check *eagerly* and returns a future.
  This matches the .NET woven `Around` advice and the Python async wrapper: a misconfiguration
  surfaces at the call site, not as a faulted future. Consequence: a `#[toggle_async]` fn that
  borrows its arguments would need the returned `impl Future` to capture those lifetimes; the
  demonstrated async fns take owned/no arguments.

## Roslyn analyzer → `ftrio lint`

The .NET `ToggleConfigAnalyzer` emits `FTRIO001` in-compiler (severity `Error`) when a
`[Toggle]`-decorated method has no matching key in `Toggles`. Rust has no in-compiler hook available
here, so — exactly as the Python port did — the intent is ported as a build-time CLI: walk `.rs`
files with `syn`, resolve each `#[toggle]`/`#[toggle_async]` key, load `appsettings.json`, and report
any decorated function whose key is missing. **Exits non-zero on findings** so CI can gate on it. The
diagnostic id `FTRIO001` and the message intent are preserved verbatim.

## `Microsoft.Extensions.Configuration` → `serde_json` config module

- **Two-pass read.** Bootstrap pass reads `FtrIO:ReloadOnChange` and `FtrIO:Environment`; live pass
  reads `Toggles` / `TogglesOverrides` with the environment overlay.
- **Colon-delimited access** (`FtrIO:BlueGreen:CurrentSlot`) over a flattened view, mirroring the
  `IConfiguration` indexer.
- **Overlay merge:** `appsettings.{env}.json` deep-merges over the base, later source wins.
- **Lowercase bool stringification.** JSON booleans/numbers stringify to their lowercase form
  (`true`/`false`) before hitting the strategy chain. Because every downstream comparison is
  case-insensitive, this is behaviourally identical to the .NET path.
- **Env-var names retained verbatim** for cross-runtime parity: `ASPNETCORE_ENVIRONMENT`,
  `DOTNET_ENVIRONMENT`, and the env-var provider prefix `FTRIO__Toggles__<Key>`.
- **Reload-on-change** is implemented as re-read-on-access (an accepted alternative to a file
  watcher, per the spec; observable behaviour matches). The `notify` crate was therefore not needed;
  the core stays lean.

## Method overloads → builder + `Default` + named constructors

Rust has no method overloading. The fluent `ToggleParserBuilder` plus `Default` reproduces the same
assembled strategy chains the .NET builder produced from its overloaded factory methods. Named
constructors (`AppSettingsToggleParser::new`, `BlueGreenStrategy::new`,
`StrategyToggleParser::from_app_settings`) stand in for the overloaded constructors.

## `IDisposable` → `Drop`

`ToggleProviderBuffer` implements `Drop`, which stops the background thread and performs a **final
flush** — the `Dispose` analogue. Providers that hold resources follow the same pattern.

## Concurrency primitives

| .NET | Rust |
|---|---|
| `ConcurrentDictionary` | `Mutex<HashMap>` (last-write-wins per key before a flush) |
| `System.Threading.Timer` | background interval thread |
| `Monitor.TryEnter` | `try_lock` (skip this flush tick if a writer holds the lock) |
| `File.Replace` / `File.Move` | temp file + `std::fs::rename` (atomic write) |

The background thread polls its stop flag on a short tick, so `Drop` can join promptly even when the
flush interval is long.

## Exceptions → `Result` + `ToggleError`

The three .NET exceptions collapse into one enum, preserving meaning, changing only shape:

| C# exception | `ToggleError` variant |
|---|---|
| `ToggleDoesNotExistException` | `DoesNotExist { toggle_key }` |
| `ToggleParsedOutOfRangeException` | `ParsedOutOfRange { raw_value }` |
| `ToggleAttributeMissingException` | `AttributeMissing { method_name }` |

`ToggleError` deliberately stays a three-variant analogue of the exception trio. Provider network/
parse failures (HTTP) are therefore returned as `Box<dyn Error>` from the constructor rather than
being forced into a `ToggleError` variant they do not fit.

## A/B `i32::MIN` edge case

`compute_bucket` reads the first four SHA-256 bytes as a little-endian **signed** `i32`, then takes
`unsigned_abs() % 100`. `unsigned_abs` is used so the ~1-in-4-billion input whose first four bytes
equal `i32::MIN` yields `2147483648` (matching Python's arbitrary-precision `abs`) instead of
panicking on overflow the way `i32::abs` would. .NET throws `OverflowException` on that single input.
It is **documented, not specially handled**: the six pinned cross-language vectors never hit it, and
all three runtimes agree on every vector. The vectors are lifted verbatim from
`ftrio-python/tests/unit/test_ab_determinism.py` and asserted in `ftrio/tests/ab_determinism.rs`.

## The nameless-closure divergence

A Rust closure has no name, so the ".NET derives the key from the method name" branch has no closure
analogue. The functional API (`execute_if_toggle_on`, the async and `try_*` forms) therefore
**requires an explicit key**. Name derivation lives entirely in the `#[toggle]` macro, where the
function name is available at expansion time.

## Additive items (not in the .NET source — clearly marked)

- `toggle_parser_provider::reset()` — test-only reset of the ambient parser, for test isolation
  (the Python port added the same).
- `FTRIO_ENVIRONMENT` — an additive, lowest-precedence environment alias (matching the Python port).
- `ToggleParserBuilderError` — a dedicated builder error type for the "overrides without a context
  accessor" case (the `InvalidOperationException` analogue).
- A minimal `block_on` in the tests and playground, so `#[toggle_async]` can be demonstrated without
  pulling in an async runtime.

## Naming convention

- Toggle keys derive from the gated function's own name and follow Rust `snake_case`
  (`Toggles:send_welcome_email`). The PascalCase in the .NET `appsettings.json` was a C# artefact,
  not part of the wire contract. Explicit string keys (macro `key = "..."`, functional-API literals)
  are JSON strings and are left as-is; the pinned A/B vectors are kept verbatim (cross-language hash
  inputs, not Rust identifiers).
- Acronyms are one word (`FtrIo`, `AbTestStrategy`, `HttpToggleParser`), enforced by
  `clippy::upper_case_acronyms`.
- `*Exception` → `*Error`. Verbose identifiers retained (`compute_bucket`, `rollout_percentage`,
  `current_user_id`, `context_accessor`).

## Style-guide conformance

`rustfmt` + `clippy` (including `upper_case_acronyms` denied) in CI are the Rust analogue of the
Python port's `ruff` / `pep8-naming` gate. See `.github/workflows/ci.yml`.

## Components intentionally not ported verbatim

- **AspectInjector IL weaving** — replaced natively by the proc-macro (compile-time transformation).
- **The Roslyn analyzer assembly** — replaced by `ftrio lint` (build-time CLI, `FTRIO001` preserved).
- **The Azure App Configuration SDK** — the `azure` provider is a faithful-substitute stub built from
  an already-materialised `appsettings.json`-shaped snapshot, to keep the core free of a heavy cloud
  SDK dependency.

## Dependency choices

- **`notify` vs re-read-on-access** → re-read-on-access (see the config section): fewer dependencies,
  same observable reload behaviour.
- **`ureq` TLS backend: native-tls, not the default rustls.** rustls pulls in `ring`, whose build
  requires clang/C-assembly tooling that is not present in a default MSVC install (and is awkward on
  aarch64 Windows). `native-tls` uses the platform TLS stack (SChannel on Windows) via pure-Rust FFI,
  so the workspace builds with only the standard MSVC toolchain. HTTPS still works for the `http`
  provider and `release-check --config-url`.
