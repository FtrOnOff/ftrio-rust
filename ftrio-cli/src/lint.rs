//! `ftrio lint`: the Roslyn analyzer `ToggleConfigAnalyzer` (`FTRIO001`), ported as a build-time CLI.
//!
//! Rust has no in-compiler hook available here, so the analyzer's intent is ported exactly as the
//! Python port did it: walk `.rs` files with `syn`, resolve each `#[toggle]`/`#[toggle_async]` key,
//! load `appsettings.json`, and report any decorated function whose key is missing from the
//! `Toggles` section. Exits non-zero on findings so CI can gate on it (the analyzer's `Error`
//! severity). The diagnostic id `FTRIO001` and the message intent are preserved.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::appconfig;
use crate::scan;

/// Arguments for `lint`.
pub struct LintOptions {
    pub path: PathBuf,
    pub verbose: bool,
}

/// Run the lint. Returns exit code 1 if any finding is reported, else 0.
pub fn run(options: LintOptions) -> i32 {
    let usages = scan::scan_path(&options.path);
    let attribute_usages: Vec<_> = usages
        .iter()
        .filter(|usage| usage.source.starts_with("#[toggle"))
        .collect();

    let known_keys = load_known_keys(&options.path);

    let mut finding_count = 0;
    for usage in &attribute_usages {
        if known_keys.contains(&usage.key.to_ascii_lowercase()) {
            if options.verbose {
                println!("ok: '{}' -> Toggles:{}", usage.method, usage.key);
            }
            continue;
        }
        finding_count += 1;
        println!(
            "FTRIO001: Function '{}' is decorated with {} but has no entry in the Toggles section \
             of appsettings.json",
            usage.method, usage.source
        );
        println!("  at {}:{}", usage.file, usage.line);
    }

    if finding_count == 0 {
        if options.verbose {
            println!("No FTRIO001 findings.");
        }
        0
    } else {
        eprintln!("\n{finding_count} FTRIO001 finding(s).");
        1
    }
}

/// Load the lowercase set of toggle keys from the nearest `appsettings.json`.
fn load_known_keys(path: &Path) -> HashSet<String> {
    let mut keys = HashSet::new();
    let Some(config_path) = find_appsettings(path) else {
        eprintln!("warning: no appsettings.json found; every decorated key will be reported.");
        return keys;
    };
    if let Some(config) = appconfig::load_single(&config_path) {
        for key in config.toggles.keys() {
            keys.insert(key.to_ascii_lowercase());
        }
    }
    keys
}

/// Find the nearest `appsettings.json` at or under `path`, skipping build/cache directories.
fn find_appsettings(path: &Path) -> Option<PathBuf> {
    let search_root = if path.is_file() {
        path.parent()?.to_path_buf()
    } else {
        path.to_path_buf()
    };

    let direct = search_root.join("appsettings.json");
    if direct.is_file() {
        return Some(direct);
    }

    let mut stack = vec![search_root];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                let name = entry_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                if matches!(name, "target" | ".git" | "node_modules") {
                    continue;
                }
                stack.push(entry_path);
            } else if entry_path.file_name().and_then(|n| n.to_str()) == Some("appsettings.json") {
                return Some(entry_path);
            }
        }
    }
    None
}
