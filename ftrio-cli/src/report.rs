//! The default command: a toggle report cross-referencing scanned usage against config.

use std::path::{Path, PathBuf};

use crate::appconfig::{self, AppConfig};
use crate::scan::{self, ToggleUsage};
use crate::table::render_table;

/// Arguments for the default report command.
pub struct ReportOptions {
    pub source: PathBuf,
    pub config: Option<PathBuf>,
    pub environment: Option<String>,
    pub markdown: Option<PathBuf>,
    pub show_overrides: bool,
}

/// Run the report. Always returns exit code 0.
pub fn run(options: ReportOptions) -> i32 {
    let usages = scan::scan_path(&options.source);
    let configs = resolve_configs(&options);

    let mut markdown_sections = Vec::new();

    if configs.is_empty() {
        println!("No appsettings*.json found; showing scanned toggles only.\n");
        let missing_config = AppConfig {
            name: "(no config)".to_string(),
            toggles: Default::default(),
            current_slot: None,
            known_slots: vec!["blue".to_string(), "green".to_string()],
            overrides: Default::default(),
        };
        markdown_sections.push(render_environment(
            &missing_config,
            &usages,
            options.show_overrides,
        ));
    } else {
        for config in &configs {
            markdown_sections.push(render_environment(config, &usages, options.show_overrides));
        }
    }

    if let Some(markdown_path) = &options.markdown {
        let body = markdown_sections.join("\n");
        if let Err(error) = std::fs::write(markdown_path, body) {
            eprintln!("failed to write markdown report: {error}");
        } else {
            println!("\nMarkdown report written to {}", markdown_path.display());
        }
    }

    0
}

/// Resolve which config files to report against.
fn resolve_configs(options: &ReportOptions) -> Vec<AppConfig> {
    let config_root = options
        .config
        .clone()
        .unwrap_or_else(|| options.source.clone());

    if let Some(environment) = &options.environment {
        let directory = if config_root.is_file() {
            config_root
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or(config_root)
        } else {
            config_root
        };
        return vec![appconfig::load_environment(&directory, environment)];
    }

    if config_root.is_file() {
        return appconfig::load_single(&config_root).into_iter().collect();
    }

    appconfig::find_config_files(&config_root)
        .iter()
        .filter_map(|path| appconfig::load_single(path))
        .collect()
}

/// Render one environment's table plus summary, print it, and return the markdown form.
fn render_environment(config: &AppConfig, usages: &[ToggleUsage], show_overrides: bool) -> String {
    let mut headers = vec!["Toggle Key", "Method", "Source", "State", "File", "Line"];
    if show_overrides {
        headers.push("Overrides");
    }

    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut state_counts: Vec<(String, usize)> = Vec::new();

    for usage in usages {
        let state = appconfig::classify_state(
            config.raw_value(&usage.key),
            config.current_slot.as_deref(),
            &config.known_slots,
        );
        increment(&mut state_counts, &state);

        let mut row = vec![
            usage.key.clone(),
            display_or_dash(&usage.method),
            usage.source.clone(),
            state,
            usage.file.clone(),
            usage.line.to_string(),
        ];
        if show_overrides {
            let users = config.override_users(&usage.key);
            row.push(if users.is_empty() {
                "-".to_string()
            } else {
                users.join(",")
            });
        }
        rows.push(row);
    }

    let table = render_table(&headers, &rows);
    let summary = summary_line(usages.len(), &state_counts);

    println!("=== {} ===", config.name);
    println!("{table}");
    println!("{summary}\n");

    format!("## {}\n\n{}\n\n{}\n", config.name, table, summary)
}

fn display_or_dash(value: &str) -> String {
    if value.is_empty() {
        "-".to_string()
    } else {
        value.to_string()
    }
}

fn increment(counts: &mut Vec<(String, usize)>, state: &str) {
    if let Some(entry) = counts.iter_mut().find(|(name, _)| name == state) {
        entry.1 += 1;
    } else {
        counts.push((state.to_string(), 1));
    }
}

/// Build the `N toggle(s). X ON, Y OFF, ...` summary line in the fixed state order.
fn summary_line(total: usize, counts: &[(String, usize)]) -> String {
    let mut parts = Vec::new();
    for state in appconfig::STATE_ORDER {
        if let Some((_, count)) = counts.iter().find(|(name, _)| name == state) {
            parts.push(format!("{count} {state}"));
        }
    }
    format!("{total} toggle(s). {}", parts.join(", "))
}
