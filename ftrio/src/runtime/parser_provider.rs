//! The ambient toggle parser the `#[toggle]` macro and the functional API resolve against.
//!
//! Mirrors `ToggleParserProvider`: a process-wide, thread-safe, replaceable parser instance. Until
//! configured it lazily builds the conventional default (a [`StrategyToggleParser`] over
//! `appsettings.json`), so the attribute works out of the box, and, with no config on disk, still
//! defaults every toggle to on.

use std::sync::{Arc, OnceLock, RwLock};

use crate::parsing::{StrategyToggleParser, ToggleParser};

/// The default `appsettings.json` path used when nothing has been configured.
const DEFAULT_CONFIG_PATH: &str = "appsettings.json";

type SharedParser = Arc<dyn ToggleParser>;

fn cell() -> &'static RwLock<Option<SharedParser>> {
    static INSTANCE: OnceLock<RwLock<Option<SharedParser>>> = OnceLock::new();
    INSTANCE.get_or_init(|| RwLock::new(None))
}

/// Replace the ambient parser. Call once during startup to install a parser built with the
/// [`crate::ToggleParserBuilder`] (with your context accessor, strategies, etc.).
pub fn configure(parser: SharedParser) {
    *cell().write().expect("parser provider lock poisoned") = Some(parser);
}

/// The current ambient parser, lazily initialising the conventional default on first use.
pub fn instance() -> SharedParser {
    if let Some(existing) = cell()
        .read()
        .expect("parser provider lock poisoned")
        .as_ref()
    {
        return existing.clone();
    }
    let mut guard = cell().write().expect("parser provider lock poisoned");
    if guard.is_none() {
        let default: SharedParser =
            Arc::new(StrategyToggleParser::from_app_settings(DEFAULT_CONFIG_PATH));
        *guard = Some(default);
    }
    guard.as_ref().expect("just initialised").clone()
}

/// Reset the ambient parser to its uninitialised state.
///
/// **Additive, not present in the .NET source** (recorded in `PORTING_NOTES.md`). It exists purely
/// for test isolation, so one test's configured parser does not leak into the next.
pub fn reset() {
    *cell().write().expect("parser provider lock poisoned") = None;
}
