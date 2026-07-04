//! The `#[toggle]` / `#[toggle_async]` attributes and the functional API, driven through the ambient
//! parser configured against a single temp `appsettings.json`.

mod common;

use std::sync::{Arc, Once};

use ftrio::{toggle, toggle_async, toggle_parser_provider, ToggleParserBuilder};

const CONFIG: &str = r#"{
  "FtrIO": { "BlueGreen": { "CurrentSlot": "blue", "KnownSlots": "blue,green" } },
  "Toggles": {
    "always_on_feature": true,
    "send_welcome_email": false,
    "explicit_keyed": true,
    "percent_full": "100%",
    "percent_zero": "0%",
    "slot_blue": "blue",
    "slot_green": "green",
    "ab_for_user": "ab:50",
    "async_feature": true,
    "async_off_feature": false,
    "functional_on": true,
    "functional_off": false
  }
}"#;

static INIT: Once = Once::new();

/// Configure the ambient parser once for the whole binary. `Once` makes this safe under the default
/// parallel test runner, since every test shares the same read-only configuration.
fn ensure_configured() {
    INIT.call_once(|| {
        let path = common::temp_config("attribute", CONFIG);
        let parser = ToggleParserBuilder::new()
            .with_base_path(path)
            .with_percentage_rollout()
            .with_blue_green(
                Some("blue".to_string()),
                vec!["blue".to_string(), "green".to_string()],
            )
            .with_context_strategies()
            .with_context_accessor(Arc::new(common::TestContext))
            .build()
            .expect("build ambient parser");
        toggle_parser_provider::configure(Arc::new(parser));
    });
}

// --- gated functions: the attribute itself is what gates execution ---

#[toggle] // derived key == fn name
fn always_on_feature() -> i32 {
    42
}

#[toggle] // derived key == fn name; config value is false
fn send_welcome_email() -> i32 {
    42
}

#[toggle(key = "explicit_keyed")] // explicit key overrides the derived one
fn some_renamed_function() -> i32 {
    7
}

#[toggle]
fn percent_full() -> i32 {
    1
}

#[toggle]
fn percent_zero() -> i32 {
    1
}

#[toggle]
fn slot_blue() -> i32 {
    1
}

#[toggle]
fn slot_green() -> i32 {
    1
}

#[toggle(key = "ab_for_user")]
fn ab_gated() -> i32 {
    3
}

#[toggle_async]
async fn async_feature() -> i32 {
    5
}

#[toggle_async]
async fn async_off_feature() -> i32 {
    5
}

// --- tests ---

#[test]
fn derived_key_runs_when_on() {
    ensure_configured();
    assert_eq!(always_on_feature(), 42);
}

#[test]
fn derived_key_skipped_returns_default_when_off() {
    ensure_configured();
    assert_eq!(send_welcome_email(), 0); // Default::default() for i32
}

#[test]
fn explicit_key_overrides_derived_key() {
    ensure_configured();
    assert_eq!(some_renamed_function(), 7);
}

#[test]
fn percentage_boundaries_gate_deterministically() {
    ensure_configured();
    assert_eq!(percent_full(), 1);
    assert_eq!(percent_zero(), 0);
}

#[test]
fn blue_green_gates_against_current_slot() {
    ensure_configured();
    assert_eq!(slot_blue(), 1);
    assert_eq!(slot_green(), 0);
}

#[test]
fn async_toggle_awaitable_when_on_and_off() {
    ensure_configured();
    assert_eq!(common::block_on(async_feature()), 5);
    assert_eq!(common::block_on(async_off_feature()), 0);
}

#[test]
fn ab_bucket_is_stable_for_a_user() {
    ensure_configured();
    // alice's bucket for this toggle is fixed, so repeated calls agree.
    assert_eq!(ab_gated(), ab_gated());
}

#[test]
fn functional_api_runs_and_defaults() {
    ensure_configured();
    assert_eq!(ftrio::execute_if_toggle_on(|| 9, "functional_on"), 9);
    let off: i32 = ftrio::execute_if_toggle_on(|| 9, "functional_off");
    assert_eq!(off, 0);
    assert_eq!(
        ftrio::try_execute_if_toggle_on(|| 9, "functional_on"),
        Ok(9)
    );
}
