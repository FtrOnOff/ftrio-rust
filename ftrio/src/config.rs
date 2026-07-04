//! Reproduces the relevant behaviour of `Microsoft.Extensions.Configuration` with `serde_json`.
//!
//! The `.NET` runtime layers configuration sources and exposes a colon-delimited indexer
//! (`FtrIO:BlueGreen:CurrentSlot`) over the flattened result. We reproduce the parts FtrIO depends
//! on: a base `appsettings.json`, an environment overlay `appsettings.{env}.json` (later source
//! wins), colon-delimited access over a flattened view, and the offline-safe "no file at all"
//! signal that makes every toggle default to on.
//!
//! JSON booleans and numbers are stringified to their **lowercase** string form (`true`/`false`)
//! before reaching the strategy chain. Because every downstream comparison is case-insensitive, this
//! matches the .NET behaviour exactly; it is documented here and in `PORTING_NOTES.md`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// A merged, read-only view of the FtrIO configuration.
#[derive(Debug, Clone)]
pub struct AppSettings {
    root: Value,
    present: bool,
}

impl AppSettings {
    /// Load the base file and (optionally) the environment overlay, merging them so the overlay
    /// wins — the two-source model `IConfiguration` uses. If neither file exists, `present()` is
    /// `false`, which drives the offline-safe default.
    pub fn load(base_file: &Path, environment: Option<&str>) -> Self {
        let mut root = Value::Object(Map::new());
        let mut present = false;

        if let Some(base) = read_json_file(base_file) {
            deep_merge(&mut root, base);
            present = true;
        }

        if let Some(environment_name) = environment {
            let overlay = environment_overlay_path(base_file, environment_name);
            if let Some(overlay_value) = read_json_file(&overlay) {
                deep_merge(&mut root, overlay_value);
                present = true;
            }
        }

        AppSettings { root, present }
    }

    /// Build a settings view directly from an already-parsed JSON value (used by the HTTP/Azure
    /// providers, which fetch the appsettings shape rather than read a file).
    #[cfg_attr(not(any(feature = "http", feature = "azure")), allow(dead_code))]
    pub fn from_value(root: Value) -> Self {
        AppSettings {
            root,
            present: true,
        }
    }

    /// Whether any backing configuration exists at all. When `false`, every toggle resolves to
    /// `true` (the offline-safe default).
    pub fn present(&self) -> bool {
        self.present
    }

    /// Colon-delimited access mirroring the `IConfiguration` indexer, returning the value in its
    /// lowercase-stringified form. `FtrIO:BlueGreen:CurrentSlot` walks `root["FtrIO"]["BlueGreen"]
    /// ["CurrentSlot"]`.
    pub fn get_string(&self, colon_key: &str) -> Option<String> {
        navigate(&self.root, colon_key.split(':')).and_then(value_to_config_string)
    }

    /// A boolean setting (e.g. `FtrIO:ReloadOnChange`), accepting JSON bool or the string/number
    /// forms `true`/`false`/`1`/`0`.
    pub fn get_bool(&self, colon_key: &str) -> Option<bool> {
        navigate(&self.root, colon_key.split(':')).and_then(coerce_bool)
    }

    /// The raw value of a toggle from the `Toggles` section, lowercase-stringified.
    pub fn get_toggle_value(&self, toggle_key: &str) -> Option<String> {
        navigate(&self.root, ["Toggles", toggle_key]).and_then(value_to_config_string)
    }

    /// A per-user override from `TogglesOverrides[toggle_key][user_id]`, accepting `true`/`false`/
    /// `1`/`0` (bool, string, or number).
    pub fn get_override(&self, toggle_key: &str, user_id: &str) -> Option<bool> {
        navigate(&self.root, ["TogglesOverrides", toggle_key, user_id]).and_then(coerce_bool)
    }
}

/// Read and parse a JSON file, returning `None` if it does not exist or cannot be parsed.
fn read_json_file(path: &Path) -> Option<Value> {
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Derive `appsettings.{env}.json` next to the base file, preserving the base file's stem so a
/// custom base name still overlays correctly.
fn environment_overlay_path(base_file: &Path, environment_name: &str) -> PathBuf {
    let stem = base_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("appsettings");
    let extension = base_file
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("json");
    let overlay_name = format!("{stem}.{environment_name}.{extension}");
    match base_file.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(overlay_name),
        _ => PathBuf::from(overlay_name),
    }
}

/// Deep-merge `overlay` into `base` so overlay object keys win while sibling keys are preserved.
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

/// Walk nested objects following the given path segments.
fn navigate<'a, I>(root: &'a Value, segments: I) -> Option<&'a Value>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut current = root;
    for segment in segments {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}

/// Stringify a config value to the lowercase form the strategy chain expects.
fn value_to_config_string(value: &Value) -> Option<String> {
    match value {
        Value::Bool(boolean) => Some(boolean.to_string()), // already lowercase "true"/"false"
        Value::Number(number) => Some(number.to_string()),
        Value::String(string) => Some(string.clone()),
        _ => None,
    }
}

/// Coerce a value to a boolean, accepting bool, the numbers `1`/`0`, and the strings `true`/`false`/
/// `1`/`0` (case-insensitive).
fn coerce_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(boolean) => Some(*boolean),
        Value::Number(number) => match number.as_i64() {
            Some(1) => Some(true),
            Some(0) => Some(false),
            _ => None,
        },
        Value::String(string) => match string.trim().to_ascii_lowercase().as_str() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        },
        _ => None,
    }
}
