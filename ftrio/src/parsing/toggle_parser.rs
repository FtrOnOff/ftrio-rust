//! The `ToggleParser` trait and the concrete file reader `AppSettingsToggleParser`.
//!
//! In the .NET source the interface `IToggleParser` and the concrete class `ToggleParser` share a
//! name; the Rust port renames the concrete type to say what it reads, exactly as the Python port
//! did. `AppSettingsToggleParser` is the boolean-only base reader; the richer value grammar is
//! layered on by [`crate::StrategyToggleParser`].

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::config::AppSettings;
use crate::context::FtrIoContextAccessor;
use crate::error::ToggleError;
use crate::providers::ToggleValueProvider;
use crate::strategies::parse_boolean_value;

/// The read path every gated call funnels through. Mirrors `IToggleParser`.
pub trait ToggleParser: Send + Sync {
    /// Resolve a toggle to a final on/off decision, applying overrides and (in richer parsers) the
    /// strategy chain.
    fn get_toggle_status(&self, toggle_key: &str) -> Result<bool, ToggleError>;

    /// Read the raw value from the source and parse it as a boolean. The base reader exposes only
    /// this; strategy parsers extend `get_toggle_status` beyond it.
    fn parse_bool_value_from_source(&self, toggle_key: &str) -> Result<bool, ToggleError>;

    /// The per-user override for this toggle, if any. Defaults to `None`, mirroring the optional
    /// override hook on the .NET interface.
    fn get_override(&self, toggle_key: &str) -> Option<bool> {
        let _ = toggle_key;
        None
    }
}

/// The concrete `appsettings.json` reader. Boolean-only on its own; also usable as a
/// [`ToggleValueProvider`] source for [`crate::StrategyToggleParser`].
pub struct AppSettingsToggleParser {
    base_path: PathBuf,
    environment: Option<String>,
    reload_on_change: bool,
    context_accessor: Option<Arc<dyn FtrIoContextAccessor>>,
    cached_settings: Mutex<Option<Arc<AppSettings>>>,
}

impl AppSettingsToggleParser {
    /// Construct a reader for `base_path` (typically `appsettings.json`).
    ///
    /// Runs the bootstrap pass immediately: it reads `FtrIO:ReloadOnChange` and `FtrIO:Environment`
    /// from the base file, then resolves the effective environment for the live overlay.
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        let base_path = base_path.into();
        let bootstrap = AppSettings::load(&base_path, None);
        let reload_on_change = bootstrap.get_bool("FtrIO:ReloadOnChange").unwrap_or(false);
        let environment = resolve_environment(&bootstrap);
        AppSettingsToggleParser {
            base_path,
            environment,
            reload_on_change,
            context_accessor: None,
            cached_settings: Mutex::new(None),
        }
    }

    /// Attach a context accessor so per-user overrides can be resolved.
    pub fn with_context_accessor(mut self, accessor: Arc<dyn FtrIoContextAccessor>) -> Self {
        self.context_accessor = Some(accessor);
        self
    }

    /// The resolved environment name (`FtrIO:Environment` / `ASPNETCORE_ENVIRONMENT` / …), if any.
    pub fn environment(&self) -> Option<&str> {
        self.environment.as_deref()
    }

    /// Load the current settings, honouring reload-on-change. When reload is on we re-read on every
    /// access (an accepted alternative to a file watcher, per the spec); otherwise we cache once.
    fn current_settings(&self) -> Arc<AppSettings> {
        if self.reload_on_change {
            return Arc::new(AppSettings::load(
                &self.base_path,
                self.environment.as_deref(),
            ));
        }
        let mut cache = self
            .cached_settings
            .lock()
            .expect("settings mutex poisoned");
        if let Some(existing) = cache.as_ref() {
            return existing.clone();
        }
        let loaded = Arc::new(AppSettings::load(
            &self.base_path,
            self.environment.as_deref(),
        ));
        *cache = Some(loaded.clone());
        loaded
    }

    /// The current user id from the attached context, if any.
    fn current_user_id(&self) -> Option<String> {
        self.context_accessor
            .as_ref()
            .and_then(|accessor| accessor.get_user_id())
    }
}

impl ToggleParser for AppSettingsToggleParser {
    fn get_toggle_status(&self, toggle_key: &str) -> Result<bool, ToggleError> {
        let settings = self.current_settings();
        if !settings.present() {
            return Ok(true); // offline-safe default
        }
        if let Some(overridden) = ToggleParser::get_override(self, toggle_key) {
            return Ok(overridden);
        }
        match settings.get_toggle_value(toggle_key) {
            Some(raw_value) => parse_boolean_value(&raw_value),
            None => Err(ToggleError::DoesNotExist {
                toggle_key: toggle_key.to_string(),
            }),
        }
    }

    fn parse_bool_value_from_source(&self, toggle_key: &str) -> Result<bool, ToggleError> {
        let settings = self.current_settings();
        if !settings.present() {
            return Ok(true);
        }
        match settings.get_toggle_value(toggle_key) {
            Some(raw_value) => parse_boolean_value(&raw_value),
            None => Err(ToggleError::DoesNotExist {
                toggle_key: toggle_key.to_string(),
            }),
        }
    }

    fn get_override(&self, toggle_key: &str) -> Option<bool> {
        let user_id = self.current_user_id()?;
        self.current_settings().get_override(toggle_key, &user_id)
    }
}

impl ToggleValueProvider for AppSettingsToggleParser {
    fn get_raw_value(&self, toggle_key: &str) -> Result<Option<String>, ToggleError> {
        Ok(self.current_settings().get_toggle_value(toggle_key))
    }

    fn config_present(&self) -> bool {
        self.current_settings().present()
    }

    fn get_override(&self, toggle_key: &str, user_id: &str) -> Option<bool> {
        self.current_settings().get_override(toggle_key, user_id)
    }
}

/// Resolve the environment name in the .NET precedence order, plus the additive lowest-precedence
/// `FTRIO_ENVIRONMENT` alias (matching the Python port).
fn resolve_environment(bootstrap: &AppSettings) -> Option<String> {
    if let Some(from_config) = bootstrap.get_string("FtrIO:Environment") {
        return Some(from_config);
    }
    for variable in [
        "ASPNETCORE_ENVIRONMENT",
        "DOTNET_ENVIRONMENT",
        "FTRIO_ENVIRONMENT",
    ] {
        if let Ok(value) = std::env::var(variable) {
            if !value.trim().is_empty() {
                return Some(value);
            }
        }
    }
    None
}

/// Read the `FtrIO:BlueGreen` settings from a base file, used to configure the default strategy
/// chain. Returns `(current_slot, known_slots)`.
pub(crate) fn read_blue_green_settings(base_path: &Path) -> (Option<String>, Vec<String>) {
    let bootstrap = AppSettings::load(base_path, None);
    let current_slot = bootstrap.get_string("FtrIO:BlueGreen:CurrentSlot");
    let known_slots = bootstrap
        .get_string("FtrIO:BlueGreen:KnownSlots")
        .map(|slots| {
            slots
                .split(',')
                .map(str::trim)
                .filter(|slot| !slot.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    (current_slot, known_slots)
}
