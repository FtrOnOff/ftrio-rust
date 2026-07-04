//! HTTP toggle provider (feature `http`).
//!
//! Fetches the exact `appsettings.json` shape from a URL — the cross-language wire contract — and
//! serves it as a [`ToggleValueProvider`]. Mirrors the .NET `Providers.Http` project.

use super::ToggleValueProvider;
use crate::config::AppSettings;
use crate::error::ToggleError;

/// A value source backed by a snapshot fetched over HTTP.
pub struct HttpToggleParser {
    settings: AppSettings,
}

impl HttpToggleParser {
    /// Fetch the config snapshot from `url`. The response body must be the `appsettings.json` shape
    /// (`FtrIO` / `Toggles` / `TogglesOverrides`).
    ///
    /// Returns a boxed error on a network/parse failure — those are not toggle-resolution errors, so
    /// they are kept out of [`ToggleError`], which stays a faithful three-variant analogue of the
    /// .NET exception trio.
    pub fn fetch(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let body = ureq::get(url).call()?.into_string()?;
        let value = serde_json::from_str(&body)?;
        Ok(HttpToggleParser {
            settings: AppSettings::from_value(value),
        })
    }
}

impl ToggleValueProvider for HttpToggleParser {
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
