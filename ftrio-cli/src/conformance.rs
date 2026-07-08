//! The hidden `conformance-resolve` subcommand.
//!
//! It reads exactly one conformance resolution case as JSON on stdin, runs it through the real FtrIO
//! resolution logic, and prints the outcome as JSON on stdout: `{"result": true}` / `{"result":
//! false}` for a decision, or `{"error": "DoesNotExist"}` (etc.) for a named error. It is the
//! per-port hook the language-agnostic conformance driver drives to build a cross-port matrix; it is
//! hidden because it is tooling for the test harness, not a user-facing command.

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use ftrio::{FtrIoContextAccessor, ToggleError, ToggleParser, ToggleParserBuilder};
use serde_json::{json, Value};

struct CaseContext {
    user_id: Option<String>,
    attributes: HashMap<String, String>,
}

impl FtrIoContextAccessor for CaseContext {
    fn get_user_id(&self) -> Option<String> {
        self.user_id.clone()
    }
    fn get_attribute(&self, attribute_name: &str) -> Option<String> {
        self.attributes.get(attribute_name).cloned()
    }
}

static CASE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn parse_blue_green(config: &Value) -> (Option<String>, Vec<String>) {
    let blue_green = config.get("FtrIO").and_then(|ftrio| ftrio.get("BlueGreen"));
    let current_slot = blue_green
        .and_then(|section| section.get("CurrentSlot"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let known_slots = blue_green
        .and_then(|section| section.get("KnownSlots"))
        .and_then(|value| value.as_str())
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

fn resolve(case: &Value) -> Result<bool, String> {
    let toggle_key = case
        .get("toggleKey")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "toggleKey missing".to_string())?;

    let config = case.get("config").cloned().unwrap_or(Value::Null);
    let context = case.get("context").cloned().unwrap_or(Value::Null);

    let unique = CASE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!("ftrio_cr_{}_{}", process::id(), unique));
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let app_settings_path: PathBuf = directory.join("appsettings.json");

    let (current_slot, known_slots) = if config.is_null() {
        (None, Vec::new())
    } else {
        let serialized =
            serde_json::to_string_pretty(&config).map_err(|error| error.to_string())?;
        std::fs::write(&app_settings_path, serialized).map_err(|error| error.to_string())?;
        parse_blue_green(&config)
    };

    let accessor = CaseContext {
        user_id: context
            .get("userId")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        attributes: context
            .get("attributes")
            .and_then(|value| value.as_object())
            .map(|map| {
                map.iter()
                    .filter_map(|(key, value)| {
                        value.as_str().map(|text| (key.clone(), text.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default(),
    };

    let parser = ToggleParserBuilder::new()
        .with_base_path(&app_settings_path)
        .with_percentage_rollout()
        .with_blue_green(current_slot, known_slots)
        .with_context_strategies()
        .with_context_accessor(Arc::new(accessor))
        .with_overrides()
        .build()
        .map_err(|error| error.to_string())?;

    match parser.get_toggle_status(toggle_key) {
        Ok(status) => Ok(status),
        Err(ToggleError::DoesNotExist { .. }) => Err("DoesNotExist".to_string()),
        Err(ToggleError::ParsedOutOfRange { .. }) => Err("ParsedOutOfRange".to_string()),
        Err(ToggleError::AttributeMissing { .. }) => Err("AttributeMissing".to_string()),
    }
}

/// Read one case from stdin, resolve it, print the outcome as JSON. Returns the process exit code.
pub fn run() -> i32 {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("conformance-resolve: failed to read stdin");
        return 2;
    }
    let case: Value = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("conformance-resolve: invalid JSON on stdin: {error}");
            return 2;
        }
    };
    let output = match resolve(&case) {
        Ok(status) => json!({ "result": status }),
        Err(error_name) => json!({ "error": error_name }),
    };
    println!("{output}");
    0
}
