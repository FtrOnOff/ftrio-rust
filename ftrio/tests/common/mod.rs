//! Shared helpers for the integration tests. Included per test binary via `mod common;`.

#![allow(dead_code)]

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use ftrio::{FtrIoContextAccessor, ToggleError, ToggleValueProvider};

/// A context accessor with a fixed user and a small attribute set (`plan = premium`).
pub struct TestContext;

impl FtrIoContextAccessor for TestContext {
    fn get_user_id(&self) -> Option<String> {
        Some("alice".to_string())
    }
    fn get_attribute(&self, attribute_name: &str) -> Option<String> {
        match attribute_name {
            "plan" => Some("premium".to_string()),
            "country" => Some("NL".to_string()),
            _ => None,
        }
    }
}

/// A context accessor with no user and no attributes.
pub struct EmptyContext;

impl FtrIoContextAccessor for EmptyContext {
    fn get_user_id(&self) -> Option<String> {
        None
    }
    fn get_attribute(&self, _attribute_name: &str) -> Option<String> {
        None
    }
}

/// An in-memory value source, so strategy/parser tests need no files on disk.
pub struct MapProvider {
    values: HashMap<String, String>,
    overrides: HashMap<(String, String), bool>,
    present: bool,
}

impl MapProvider {
    pub fn new() -> Self {
        MapProvider {
            values: HashMap::new(),
            overrides: HashMap::new(),
            present: true,
        }
    }

    pub fn absent() -> Self {
        MapProvider {
            values: HashMap::new(),
            overrides: HashMap::new(),
            present: false,
        }
    }

    pub fn with_value(mut self, key: &str, value: &str) -> Self {
        self.values.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_override(mut self, key: &str, user_id: &str, value: bool) -> Self {
        self.overrides
            .insert((key.to_string(), user_id.to_string()), value);
        self
    }
}

impl Default for MapProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ToggleValueProvider for MapProvider {
    fn get_raw_value(&self, toggle_key: &str) -> Result<Option<String>, ToggleError> {
        Ok(self.values.get(toggle_key).cloned())
    }
    fn config_present(&self) -> bool {
        self.present
    }
    fn get_override(&self, toggle_key: &str, user_id: &str) -> Option<bool> {
        self.overrides
            .get(&(toggle_key.to_string(), user_id.to_string()))
            .copied()
    }
}

/// Write an `appsettings.json` into a per-process temp directory and return its path.
pub fn temp_config(tag: &str, contents: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ftrio_test_{}_{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("appsettings.json");
    std::fs::write(&path, contents).expect("write temp config");
    path
}

/// A minimal executor: our test futures never truly pend, so a no-op waker with a poll loop is
/// enough to drive `#[toggle_async]` results to completion without pulling in a runtime.
pub fn block_on<F: Future>(future: F) -> F::Output {
    fn noop_waker() -> Waker {
        const VTABLE: RawWakerVTable = RawWakerVTable::new(
            |_| RawWaker::new(std::ptr::null(), &VTABLE),
            |_| {},
            |_| {},
            |_| {},
        );
        unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
    }

    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::hint::spin_loop(),
        }
    }
}
