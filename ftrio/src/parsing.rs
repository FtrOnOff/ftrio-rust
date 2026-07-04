//! The read path: the `ToggleParser` trait, the concrete file reader, the strategy-routing engine,
//! and per-user override resolution.

mod overrides;
mod strategy_parser;
mod toggle_parser;

pub use overrides::OverrideResolver;
pub use strategy_parser::StrategyToggleParser;
pub use toggle_parser::{AppSettingsToggleParser, ToggleParser};
