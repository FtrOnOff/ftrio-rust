//! Staged toggle writes with periodic, atomic flush.
//!
//! Mirrors `ToggleProviderBuffer` and its concurrency model. The .NET version stages writes in a
//! `ConcurrentDictionary`, flushes on a `System.Threading.Timer`, guards the flush with
//! `Monitor.TryEnter` (skip if busy), writes atomically with `File.Replace`/`File.Move`, and flushes
//! a final time on `Dispose`. The Rust analogues, one-for-one:
//! - `ConcurrentDictionary` → `Mutex<HashMap>` (last-write-wins per key before a flush),
//! - `System.Threading.Timer` → a background interval thread,
//! - `Monitor.TryEnter` → `try_lock` (skip this tick if a writer holds the lock),
//! - `File.Replace`/`File.Move` → temp file + `std::fs::rename`,
//! - `IDisposable`/`Dispose` → `Drop` (final flush).

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::{Map, Value};

/// Stages toggle writes and flushes them to `appsettings.json`. Mirrors `IToggleBuffer`.
pub trait ToggleBuffer: Send + Sync {
    /// Stage a toggle value; the latest staged value for a key wins before the next flush.
    fn stage_toggle(&self, toggle_key: &str, value: bool);

    /// Flush all staged values to disk now, atomically.
    fn flush(&self) -> io::Result<()>;
}

/// The concrete buffer: a background thread flushes staged values on an interval, and a final flush
/// runs on drop.
pub struct ToggleProviderBuffer {
    staged: Arc<Mutex<HashMap<String, bool>>>,
    file_path: Arc<PathBuf>,
    stop_flag: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl ToggleProviderBuffer {
    /// Create a buffer writing to `file_path`, flushing every `flush_interval`.
    pub fn new(file_path: impl Into<PathBuf>, flush_interval: Duration) -> Self {
        let staged: Arc<Mutex<HashMap<String, bool>>> = Arc::new(Mutex::new(HashMap::new()));
        let file_path = Arc::new(file_path.into());
        let stop_flag = Arc::new(AtomicBool::new(false));

        let worker = {
            let staged = staged.clone();
            let file_path = file_path.clone();
            let stop_flag = stop_flag.clone();
            // Poll the stop flag on a short tick so `Drop` can join promptly even when the flush
            // interval is long; only actually flush once a full interval has elapsed.
            let tick = std::cmp::min(flush_interval, Duration::from_millis(25));
            thread::spawn(move || {
                let mut elapsed = Duration::ZERO;
                loop {
                    thread::sleep(tick);
                    if stop_flag.load(Ordering::SeqCst) {
                        break;
                    }
                    elapsed += tick;
                    if elapsed < flush_interval {
                        continue;
                    }
                    elapsed = Duration::ZERO;
                    // Skip this tick if a writer holds the lock (the `Monitor.TryEnter` analogue).
                    if let Ok(mut guard) = staged.try_lock() {
                        if !guard.is_empty() {
                            let snapshot: HashMap<String, bool> = guard.drain().collect();
                            drop(guard);
                            let _ = write_toggles_atomically(&file_path, &snapshot);
                        }
                    }
                }
            })
        };

        ToggleProviderBuffer {
            staged,
            file_path,
            stop_flag,
            worker: Some(worker),
        }
    }

    /// Drain the staged values under the lock and write them atomically.
    fn flush_staged(&self) -> io::Result<()> {
        let snapshot: HashMap<String, bool> = {
            let mut guard = self.staged.lock().expect("buffer mutex poisoned");
            if guard.is_empty() {
                return Ok(());
            }
            guard.drain().collect()
        };
        write_toggles_atomically(&self.file_path, &snapshot)
    }
}

impl ToggleBuffer for ToggleProviderBuffer {
    fn stage_toggle(&self, toggle_key: &str, value: bool) {
        self.staged
            .lock()
            .expect("buffer mutex poisoned")
            .insert(toggle_key.to_string(), value);
    }

    fn flush(&self) -> io::Result<()> {
        self.flush_staged()
    }
}

impl Drop for ToggleProviderBuffer {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        // Final flush on drop — the `IDisposable`/`Dispose` analogue.
        let _ = self.flush_staged();
    }
}

/// Merge staged toggles into the `Toggles` section and write the file atomically, preserving every
/// other section. Creates `Toggles` if absent.
fn write_toggles_atomically(file_path: &Path, staged: &HashMap<String, bool>) -> io::Result<()> {
    let mut root: Value = fs::read_to_string(file_path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_else(|| Value::Object(Map::new()));
    if !root.is_object() {
        root = Value::Object(Map::new());
    }

    let object = root.as_object_mut().expect("root is an object");
    let toggles = object
        .entry("Toggles")
        .or_insert_with(|| Value::Object(Map::new()));
    if !toggles.is_object() {
        *toggles = Value::Object(Map::new());
    }
    let toggles_map = toggles.as_object_mut().expect("Toggles is an object");
    for (key, value) in staged {
        toggles_map.insert(key.clone(), Value::Bool(*value));
    }

    let serialized = serde_json::to_string_pretty(&root).map_err(io::Error::other)?;
    let temp_path = temp_sibling(file_path);
    fs::write(&temp_path, serialized)?;
    fs::rename(&temp_path, file_path)?;
    Ok(())
}

/// A sibling temp path (`<file>.tmp`) used for the atomic write-then-rename.
fn temp_sibling(path: &Path) -> PathBuf {
    let mut raw = path.as_os_str().to_owned();
    raw.push(".tmp");
    PathBuf::from(raw)
}
