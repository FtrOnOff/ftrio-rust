//! Source scanning: find toggle usage in `.rs` files with `syn`.
//!
//! Where the .NET/Python scanners use Roslyn/`ast`, the Rust scanner parses `.rs` files with `syn`.
//! It finds `#[toggle]` / `#[toggle_async]` attributes on functions (key = fn name, unless an
//! explicit `key = "..."` is given) and explicit `execute_if_toggle_on[_async]` calls with a
//! string-literal key.

use std::fs;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};
use syn::{Attribute, Expr, ExprCall, ExprLit, ImplItemFn, ItemFn, Lit, Meta};

/// One discovered toggle usage.
#[derive(Debug, Clone)]
pub struct ToggleUsage {
    /// The resolved toggle key.
    pub key: String,
    /// The enclosing method/function name (the fn name for attributes; empty for manual calls).
    pub method: String,
    /// The source label: `#[toggle]`, `#[toggle_async]`, `ManualCall`, `ManualCallAsync`.
    pub source: String,
    /// Path to the file, relative to the scan root.
    pub file: String,
    /// 1-based line number.
    pub line: usize,
}

/// Directories never worth scanning.
const SKIP_DIRS: &[&str] = &[
    "target",
    ".git",
    "node_modules",
    ".idea",
    ".vscode",
    "dist",
    "build",
];

/// Scan a directory tree (or a single file) for toggle usage.
pub fn scan_path(root: &Path) -> Vec<ToggleUsage> {
    let mut files = Vec::new();
    collect_rust_files(root, &mut files);

    let mut usages = Vec::new();
    for file in files {
        if let Ok(contents) = fs::read_to_string(&file) {
            if let Ok(parsed) = syn::parse_file(&contents) {
                let display = relative_display(root, &file);
                let mut collector = Collector {
                    file: display,
                    usages: Vec::new(),
                };
                collector.visit_file(&parsed);
                usages.extend(collector.usages);
            }
        }
    }
    usages
}

/// Recursively collect `.rs` files, skipping build/cache directories.
fn collect_rust_files(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path.to_path_buf());
        }
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            let name = entry_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if SKIP_DIRS.contains(&name) {
                continue;
            }
            collect_rust_files(&entry_path, out);
        } else if entry_path.extension().is_some_and(|ext| ext == "rs") {
            out.push(entry_path);
        }
    }
}

/// Display a scanned file relative to the scan root when possible.
fn relative_display(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

struct Collector {
    file: String,
    usages: Vec<ToggleUsage>,
}

impl Collector {
    fn handle_attributes(&mut self, attrs: &[Attribute], method_name: &str, line: usize) {
        for attr in attrs {
            let source = if attr.path().is_ident("toggle") {
                "#[toggle]"
            } else if attr.path().is_ident("toggle_async") {
                "#[toggle_async]"
            } else {
                continue;
            };
            let key = explicit_key(attr).unwrap_or_else(|| method_name.to_string());
            self.usages.push(ToggleUsage {
                key,
                method: method_name.to_string(),
                source: source.to_string(),
                file: self.file.clone(),
                line,
            });
        }
    }
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let line = node.sig.ident.span().start().line;
        self.handle_attributes(&node.attrs, &node.sig.ident.to_string(), line);
        visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        let line = node.sig.ident.span().start().line;
        self.handle_attributes(&node.attrs, &node.sig.ident.to_string(), line);
        visit::visit_impl_item_fn(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Some((source, key, line)) = manual_call(node) {
            self.usages.push(ToggleUsage {
                key,
                method: String::new(),
                source: source.to_string(),
                file: self.file.clone(),
                line,
            });
        }
        visit::visit_expr_call(self, node);
    }
}

/// Extract `key = "..."` from a toggle attribute, if present.
fn explicit_key(attr: &Attribute) -> Option<String> {
    let Meta::List(list) = &attr.meta else {
        return None;
    };
    let name_value: syn::MetaNameValue = list.parse_args().ok()?;
    if !name_value.path.is_ident("key") {
        return None;
    }
    match name_value.value {
        Expr::Lit(ExprLit {
            lit: Lit::Str(key), ..
        }) => Some(key.value()),
        _ => None,
    }
}

/// Recognise an `execute_if_toggle_on[_async]` (or `try_` variant) call and extract its string key.
fn manual_call(node: &ExprCall) -> Option<(&'static str, String, usize)> {
    let Expr::Path(path) = node.func.as_ref() else {
        return None;
    };
    let function_name = path.path.segments.last()?.ident.to_string();
    let source = match function_name.as_str() {
        "execute_if_toggle_on" | "try_execute_if_toggle_on" => "ManualCall",
        "execute_if_toggle_on_async" | "try_execute_if_toggle_on_async" => "ManualCallAsync",
        _ => return None,
    };
    // The key is the string-literal argument.
    for argument in &node.args {
        if let Expr::Lit(ExprLit {
            lit: Lit::Str(key), ..
        }) = argument
        {
            let line = key.span().start().line;
            return Some((source, key.value(), line));
        }
    }
    None
}
