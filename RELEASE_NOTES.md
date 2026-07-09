# FtrIO for Rust, v0.1.2

Cross-port conformance and a config-lookup correctness fix. One small user-facing behaviour change
(case-insensitive config keys); the rest is test-harness tooling.

## Changed

- **Config keys now match case-insensitively**, mirroring .NET's `Microsoft.Extensions.Configuration`
  (whose key comparer is `OrdinalIgnoreCase`). A lookup of `newcheckout` now resolves a config key
  written `NewCheckout`, and colon paths like `FtrIO:BlueGreen:CurrentSlot` match regardless of case.
  An exact match is still tried first as a fast path. This brings the Rust port in line with the .NET
  and Python runtimes.

## Added

- **`ftrio conformance-resolve`** (hidden subcommand): the per-port hook for the language-agnostic
  `ftrio-conformance` suite. It reads one resolution case as JSON on stdin (`toggleKey`, `config`,
  `context`), runs it through the real FtrIO resolution path, and prints the outcome as JSON:
  `{"result": true|false}`, or `{"error": "DoesNotExist" | "ParsedOutOfRange" | "AttributeMissing"}`.
  A cross-port driver feeds the same cases to every runtime to build a parity matrix, proving the
  Rust, .NET, and Python ports agree. It is hidden because it is test tooling, not a user command.

## Upgrade notes

- No breaking changes and no code changes required. The only behavioural shift is that config-key
  matching is now case-insensitive; since the other runtimes already behaved this way, existing
  cross-language configs become more consistent, not less.

---

For the full history, see [`CHANGELOG.md`](CHANGELOG.md).
