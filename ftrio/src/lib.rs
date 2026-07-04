//! # FtrIO (Rust)
//!
//! Attribute-based feature toggles for Rust, a faithful port of the .NET
//! [FtrIO](https://github.com/FtrOnOff/FtrIO) library, using the
//! [Python port](https://github.com/FtrOnOff/ftrio-python) as a second reference.
//!
//! The centrepiece is the `#[toggle]` attribute. Rust has no runtime reflection or IL weaving, but
//! it has procedural attribute macros, compile-time code transformation, the *closest* analogue of
//! the .NET AspectInjector attribute of any target language. Decorate a function and it runs only
//! when its toggle is on; otherwise it returns `Default::default()`.
//!
//! ```ignore
//! use ftrio::toggle;
//!
//! #[toggle] // key derives from the fn name: "send_welcome_email"
//! fn send_welcome_email() {
//!     println!("welcome!");
//! }
//! ```
//!
//! With no `appsettings.json` on disk at all, every toggle defaults to **on**, the offline-safe
//! default. See [`toggle_parser_provider`] for how the ambient parser is resolved and configured,
//! and [`ToggleParserBuilder`] for assembling a custom strategy chain.

// The attribute macros, re-exported so consumers depend on a single crate and write `use
// ftrio::toggle;`, mirroring how the .NET package flows its AspectInjector weaver transitively.
pub use ftrio_macros::{toggle, toggle_async};

// Internal module tree, grouped by responsibility. The crate's public API is defined entirely by the
// `pub use` re-exports below, so this layout is an implementation detail: every public path
// (`ftrio::ToggleParser`, `ftrio::toggle_parser_provider::instance()`, …) stays stable regardless of
// which folder a type physically lives in.
mod config;
mod context;
mod error;
mod parsing;
pub mod providers;
mod runtime;
pub mod strategies;

// The ambient parser module, kept at the public path `ftrio::toggle_parser_provider` (used by the
// `#[toggle]` macro expansion and by consumers) even though it now lives under `runtime/`.
pub use context::FtrIoContextAccessor;
pub use error::ToggleError;
pub use parsing::{AppSettingsToggleParser, OverrideResolver, StrategyToggleParser, ToggleParser};
pub use providers::{CompositeToggleParser, EnvironmentVariableToggleParser, ToggleValueProvider};
pub use runtime::parser_provider as toggle_parser_provider;
pub use runtime::{
    execute_if_toggle_on, execute_if_toggle_on_async, try_execute_if_toggle_on,
    try_execute_if_toggle_on_async, ToggleBuffer, ToggleParserBuilder, ToggleParserBuilderError,
    ToggleProviderBuffer,
};
pub use strategies::compute_bucket;
