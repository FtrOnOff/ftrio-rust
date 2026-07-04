//! Azure App Configuration provider (feature `azure`).
//!
//! A faithful-substitute stub mirroring the .NET `Providers.AzureAppConfig` project. To keep the
//! core lean and free of a heavy cloud SDK dependency, this provider is constructed from an already
//! materialised `appsettings.json`-shaped snapshot (e.g. one a caller fetched from Azure App
//! Configuration and assembled into the wire shape). Recorded as a deviation in `PORTING_NOTES.md`.

use serde_json::Value;

use super::ToggleValueProvider;
use crate::config::AppSettings;
use crate::error::ToggleError;

/// A value source backed by an Azure App Configuration snapshot in the `appsettings.json` shape.
pub struct AzureAppConfigToggleParser {
    settings: AppSettings,
}

impl AzureAppConfigToggleParser {
    /// Construct from a snapshot value (`FtrIO` / `Toggles` / `TogglesOverrides`).
    pub fn from_snapshot(snapshot: Value) -> Self {
        AzureAppConfigToggleParser {
            settings: AppSettings::from_value(snapshot),
        }
    }
}

impl ToggleValueProvider for AzureAppConfigToggleParser {
    fn get_raw_value(&self, toggle_key: &str) -> Result<Option<String>, ToggleError> {
        Ok(self.settings.get_toggle_value(toggle_key))
    }

    fn config_present(&self) -> bool {
        self.settings.present()
    }

    fn get_override(&self, toggle_key: &str, user_id: &str) -> Option<bool> {
        self.settings.get_override(toggle_key, user_id)
    }
}
