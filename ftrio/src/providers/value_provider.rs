//! Pluggable raw-value sources for the strategy parser.
//!
//! Mirrors `IToggleValueProvider`. A source yields the raw string for a toggle key (before any
//! strategy runs), reports whether any backing config exists at all (which drives the offline-safe
//! default), and optionally resolves per-user overrides.

use crate::error::ToggleError;

/// A source of raw toggle values. Mirrors `IToggleValueProvider`.
pub trait ToggleValueProvider: Send + Sync {
    /// The raw string value for a key, or `None` if the key is absent from this source. `Err` is
    /// reserved for a genuine read failure, not mere absence.
    fn get_raw_value(&self, toggle_key: &str) -> Result<Option<String>, ToggleError>;

    /// Whether any backing configuration exists at all. `true` by default (env vars, HTTP, etc. are
    /// always "present"); the file source reports `false` when no `appsettings.json` is on disk.
    fn config_present(&self) -> bool {
        true
    }

    /// A per-user override for this toggle, if the source supports overrides. Defaults to `None`.
    fn get_override(&self, toggle_key: &str, user_id: &str) -> Option<bool> {
        let _ = (toggle_key, user_id);
        None
    }
}

/// Reads toggle values from environment variables named `FTRIO__Toggles__<Key>`.
#[derive(Debug, Default, Clone, Copy)]
pub struct EnvironmentVariableToggleParser;

impl EnvironmentVariableToggleParser {
    /// Construct the environment-variable source.
    pub fn new() -> Self {
        EnvironmentVariableToggleParser
    }

    /// The environment variable name for a toggle key.
    fn variable_name(toggle_key: &str) -> String {
        format!("FTRIO__Toggles__{toggle_key}")
    }
}

impl ToggleValueProvider for EnvironmentVariableToggleParser {
    fn get_raw_value(&self, toggle_key: &str) -> Result<Option<String>, ToggleError> {
        Ok(std::env::var(Self::variable_name(toggle_key)).ok())
    }
}

/// Tries several sources in order, first-wins. A key resolves from the first source that has it;
/// only if every source misses does the caller see `DoesNotExist`. Mirrors `CompositeToggleParser`.
pub struct CompositeToggleParser {
    sources: Vec<Box<dyn ToggleValueProvider>>,
}

impl CompositeToggleParser {
    /// Construct from an ordered list of sources. Earlier sources take precedence.
    pub fn new(sources: Vec<Box<dyn ToggleValueProvider>>) -> Self {
        CompositeToggleParser { sources }
    }
}

impl ToggleValueProvider for CompositeToggleParser {
    fn get_raw_value(&self, toggle_key: &str) -> Result<Option<String>, ToggleError> {
        for source in &self.sources {
            if let Some(value) = source.get_raw_value(toggle_key)? {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }

    fn config_present(&self) -> bool {
        self.sources.iter().any(|source| source.config_present())
    }

    fn get_override(&self, toggle_key: &str, user_id: &str) -> Option<bool> {
        self.sources
            .iter()
            .find_map(|source| source.get_override(toggle_key, user_id))
    }
}
