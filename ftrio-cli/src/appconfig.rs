//! Reading `appsettings*.json` and classifying toggle state, mirroring the .NET `ToggleStateParser`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// A parsed configuration file (or merged base+overlay for one environment).
pub struct AppConfig {
    /// A human label for the config (file name, or `appsettings ({env})`).
    pub name: String,
    /// Toggle key → raw display value.
    pub toggles: BTreeMap<String, String>,
    /// The active blue-green slot, if configured.
    pub current_slot: Option<String>,
    /// The known blue-green slots (defaults to `blue`/`green`).
    pub known_slots: Vec<String>,
    /// Toggle key → the user ids that have per-user overrides.
    pub overrides: BTreeMap<String, Vec<String>>,
}

impl AppConfig {
    /// The raw value for a toggle key, if present.
    pub fn raw_value(&self, key: &str) -> Option<&str> {
        self.toggles.get(key).map(String::as_str)
    }

    /// The users with overrides for a key.
    pub fn override_users(&self, key: &str) -> &[String] {
        self.overrides.get(key).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// Read and parse a JSON file.
pub fn read_json(path: &Path) -> Option<Value> {
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Load a single config file.
pub fn load_single(path: &Path) -> Option<AppConfig> {
    let value = read_json(path)?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("appsettings.json")
        .to_string();
    Some(from_value(name, &value))
}

/// Load the base + environment overlay merged into one config (the `--env` model).
pub fn load_environment(config_dir: &Path, environment: &str) -> AppConfig {
    let base_path = config_dir.join("appsettings.json");
    let mut root = read_json(&base_path).unwrap_or_else(|| Value::Object(Map::new()));
    let overlay_path = config_dir.join(format!("appsettings.{environment}.json"));
    if let Some(overlay) = read_json(&overlay_path) {
        deep_merge(&mut root, overlay);
    }
    from_value(format!("appsettings ({environment})"), &root)
}

/// Find `appsettings*.json` files directly under a directory.
pub fn find_config_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("appsettings") && name.ends_with(".json") {
                    files.push(path);
                }
            }
        }
    }
    files.sort();
    files
}

/// Extract the `Toggles` section as raw display strings from an arbitrary value.
pub fn toggles_section(value: &Value) -> BTreeMap<String, String> {
    let mut toggles = BTreeMap::new();
    if let Some(object) = value.get("Toggles").and_then(Value::as_object) {
        for (key, raw) in object {
            if let Some(display) = value_to_display(raw) {
                toggles.insert(key.clone(), display);
            }
        }
    }
    toggles
}

fn from_value(name: String, value: &Value) -> AppConfig {
    let toggles = toggles_section(value);

    let current_slot = value
        .get("FtrIO")
        .and_then(|f| f.get("BlueGreen"))
        .and_then(|b| b.get("CurrentSlot"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let known_slots = value
        .get("FtrIO")
        .and_then(|f| f.get("BlueGreen"))
        .and_then(|b| b.get("KnownSlots"))
        .and_then(Value::as_str)
        .map(|slots| {
            slots
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["blue".to_string(), "green".to_string()]);

    let mut overrides = BTreeMap::new();
    if let Some(object) = value.get("TogglesOverrides").and_then(Value::as_object) {
        for (key, users) in object {
            if let Some(users_object) = users.as_object() {
                let user_ids: Vec<String> = users_object.keys().cloned().collect();
                overrides.insert(key.clone(), user_ids);
            }
        }
    }

    AppConfig {
        name,
        toggles,
        current_slot,
        known_slots,
        overrides,
    }
}

/// Stringify a JSON value to its lowercase display form (mirroring the runtime's stringification).
fn value_to_display(value: &Value) -> Option<String> {
    match value {
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// Deep-merge `overlay` into `base`, overlay winning.
fn deep_merge(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                match base_map.get_mut(&key) {
                    Some(existing) => deep_merge(existing, overlay_value),
                    None => {
                        base_map.insert(key, overlay_value);
                    }
                }
            }
        }
        (base_slot, overlay_value) => *base_slot = overlay_value,
    }
}

/// The classified state of a toggle, matching the .NET `ToggleStateParser` label set.
pub fn classify_state(
    raw: Option<&str>,
    current_slot: Option<&str>,
    known_slots: &[String],
) -> String {
    let Some(raw) = raw else {
        return "MISSING".to_string();
    };
    let lower = raw.trim().to_ascii_lowercase();

    if matches!(lower.as_str(), "true" | "1") {
        return "ON".to_string();
    }
    if matches!(lower.as_str(), "false" | "0") {
        return "OFF".to_string();
    }
    if lower.ends_with('%') {
        return "PERCENTAGE".to_string();
    }
    if lower.starts_with("ab:") {
        return "AB-TEST".to_string();
    }
    if lower.starts_with("users:") {
        return "TARGETED".to_string();
    }
    if lower.starts_with("attribute:") {
        return "RULE-BASED".to_string();
    }
    if known_slots
        .iter()
        .any(|slot| slot.eq_ignore_ascii_case(&lower))
    {
        // Resolve to ON/OFF when the current slot is known, else report the raw slot state.
        return match current_slot {
            Some(current) if current.eq_ignore_ascii_case(&lower) => "ON".to_string(),
            Some(_) => "OFF".to_string(),
            None => "BLUE/GREEN".to_string(),
        };
    }
    "UNKNOWN".to_string()
}

/// The fixed display order for the per-environment summary counts.
pub const STATE_ORDER: &[&str] = &[
    "ON",
    "OFF",
    "PERCENTAGE",
    "BLUE/GREEN",
    "AB-TEST",
    "TARGETED",
    "RULE-BASED",
    "MISSING",
    "UNKNOWN",
];
