//! `ftrio export-manifest`: scan for toggle usage and write a manifest JSON.
//!
//! The manifest schema matches the .NET one exactly (a cross-tool contract consumed by
//! `release-check` and CI): `{ "generatedAt": <UTC ISO-8601>, "toggles": [ { key, source, file,
//! line } ] }`. `source` labels are the Rust-attribute forms (`#[toggle]`, `#[toggle_async]`,
//! `ManualCall`, `ManualCallAsync`).

use std::path::PathBuf;

use serde_json::json;

use crate::scan;
use crate::util::{utc_iso8601, write_atomically};

/// Arguments for `export-manifest`.
pub struct ExportOptions {
    pub source: PathBuf,
    pub output: Option<PathBuf>,
    pub pretty: bool,
}

/// Run the export. Returns exit code 0 on success, 2 on a write error.
pub fn run(options: ExportOptions) -> i32 {
    let usages = scan::scan_path(&options.source);

    let toggles: Vec<_> = usages
        .iter()
        .map(|usage| {
            json!({
                "key": usage.key,
                "source": usage.source,
                "file": usage.file,
                "line": usage.line,
            })
        })
        .collect();

    let manifest = json!({
        "generatedAt": utc_iso8601(),
        "toggles": toggles,
    });

    let serialized = if options.pretty {
        serde_json::to_string_pretty(&manifest)
    } else {
        serde_json::to_string(&manifest)
    }
    .expect("manifest serialises");

    let output_path = options
        .output
        .unwrap_or_else(|| PathBuf::from("toggles.manifest.json"));

    match write_atomically(&output_path, &serialized) {
        Ok(()) => {
            println!(
                "Wrote {} toggle(s) to {}",
                usages.len(),
                output_path.display()
            );
            0
        }
        Err(error) => {
            eprintln!("failed to write manifest: {error}");
            2
        }
    }
}
