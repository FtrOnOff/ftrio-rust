//! `ftrio release-check`: verify every manifest toggle is present in a target config.
//!
//! Exit-code contract (preserved from the source): `0` ready, `1` blocked (missing keys, unless
//! `--warn-only`), `2` usage/manifest error, `3` config error.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::appconfig;
use crate::util::write_atomically;

/// Arguments for `release-check`.
pub struct ReleaseOptions {
    pub manifest: PathBuf,
    pub config: Option<PathBuf>,
    pub config_url: Option<String>,
    pub environment_name: Option<String>,
    pub markdown: Option<PathBuf>,
    pub warn_only: bool,
}

/// Run the release check.
pub fn run(options: ReleaseOptions) -> i32 {
    // --- load the manifest (usage/manifest errors → exit 2) ---
    let Some(manifest) = appconfig::read_json(&options.manifest) else {
        eprintln!(
            "error: could not read manifest {}",
            options.manifest.display()
        );
        return 2;
    };
    let Some(manifest_keys) = manifest_keys(&manifest) else {
        eprintln!("error: manifest has no `toggles` array");
        return 2;
    };

    // --- load the target config (config errors → exit 3) ---
    let config_value = match load_config(&options) {
        Ok(value) => value,
        Err(message) => {
            eprintln!("config error: {message}");
            return 3;
        }
    };
    let toggles = appconfig::toggles_section(&config_value);
    let lowercased: std::collections::HashMap<String, String> = toggles
        .iter()
        .map(|(key, value)| (key.to_ascii_lowercase(), value.clone()))
        .collect();

    // --- classify each manifest key ---
    let mut present = Vec::new();
    let mut missing = Vec::new();
    for key in &manifest_keys {
        match lowercased.get(&key.to_ascii_lowercase()) {
            Some(value) => present.push((key.clone(), value.clone())),
            None => missing.push(key.clone()),
        }
    }

    print_console_report(&present, &missing);
    if let Some(markdown_path) = &options.markdown {
        let body = markdown_report(&present, &missing);
        if let Err(error) = write_atomically(markdown_path, &body) {
            eprintln!("failed to write markdown report: {error}");
        }
    }

    if missing.is_empty() {
        println!(
            "\nRelease ready: all {} toggle(s) present.",
            manifest_keys.len()
        );
        return 0;
    }

    if options.warn_only {
        println!(
            "\n{} toggle(s) missing (warn-only, not blocking).",
            missing.len()
        );
        return 0;
    }

    println!("\nRelease blocked: {} toggle(s) missing.", missing.len());
    1
}

/// Extract the toggle keys from a manifest value.
fn manifest_keys(manifest: &Value) -> Option<Vec<String>> {
    let array = manifest.get("toggles")?.as_array()?;
    let keys = array
        .iter()
        .filter_map(|entry| entry.get("key").and_then(Value::as_str))
        .map(str::to_string)
        .collect();
    Some(keys)
}

/// Load the target config from a URL or a file, applying an environment overlay if requested.
fn load_config(options: &ReleaseOptions) -> Result<Value, String> {
    if let Some(url) = &options.config_url {
        let body = ureq::get(url)
            .call()
            .map_err(|error| format!("fetch failed: {error}"))?
            .into_string()
            .map_err(|error| format!("read failed: {error}"))?;
        return serde_json::from_str(&body).map_err(|error| format!("parse failed: {error}"));
    }

    let Some(config_path) = &options.config else {
        return Err("either --config or --config-url is required".to_string());
    };
    let mut root = appconfig::read_json(config_path)
        .ok_or_else(|| format!("could not read {}", config_path.display()))?;

    if let Some(environment) = &options.environment_name {
        if let Some(overlay) = environment_overlay(config_path, environment) {
            deep_merge(&mut root, overlay);
        }
    }
    Ok(root)
}

/// Read the `appsettings.{env}.json` sibling of the config file, if present.
fn environment_overlay(config_path: &Path, environment: &str) -> Option<Value> {
    let directory = config_path.parent()?;
    let overlay_path = directory.join(format!("appsettings.{environment}.json"));
    appconfig::read_json(&overlay_path)
}

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

fn print_console_report(present: &[(String, String)], missing: &[String]) {
    println!("Release check\n");
    for (key, value) in present {
        println!("  [present] {key} = {value}");
    }
    for key in missing {
        println!("  [MISSING] {key}");
    }

    if !missing.is_empty() {
        println!("\nAdd the following to the Toggles section of appsettings.json:");
        println!("{}", suggestion_block(missing));
    }
}

/// The "add to appsettings.json" suggestion block for the missing keys.
fn suggestion_block(missing: &[String]) -> String {
    let entries: Vec<String> = missing
        .iter()
        .map(|key| format!("    \"{key}\": false"))
        .collect();
    format!("{{\n  \"Toggles\": {{\n{}\n  }}\n}}", entries.join(",\n"))
}

fn markdown_report(present: &[(String, String)], missing: &[String]) -> String {
    let mut body =
        String::from("# Release check\n\n| Toggle Key | Status | Value |\n|---|---|---|\n");
    for (key, value) in present {
        body.push_str(&format!("| {key} | present | {value} |\n"));
    }
    for key in missing {
        body.push_str(&format!("| {key} | MISSING | - |\n"));
    }
    body
}
