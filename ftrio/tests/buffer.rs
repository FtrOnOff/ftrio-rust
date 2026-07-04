//! The buffer stages writes, flushes atomically, honours last-write-wins, preserves other sections,
//! creates `Toggles` when absent, and performs a final flush on drop.

mod common;

use std::time::Duration;

use common::temp_config;
use ftrio::{ToggleBuffer, ToggleProviderBuffer};
use serde_json::Value;

fn read_json(path: &std::path::Path) -> Value {
    let contents = std::fs::read_to_string(path).expect("read config");
    serde_json::from_str(&contents).expect("parse config")
}

#[test]
fn flush_is_last_write_wins_and_preserves_other_sections() {
    let path = temp_config(
        "buffer_flush",
        r#"{ "FtrIO": { "Environment": "Prod" }, "Toggles": { "existing": true } }"#,
    );

    let buffer = ToggleProviderBuffer::new(path.clone(), Duration::from_millis(25));
    buffer.stage_toggle("new_flag", true);
    buffer.stage_toggle("new_flag", false); // last write wins before the flush
    buffer.flush().expect("flush");

    let json = read_json(&path);
    assert_eq!(json["Toggles"]["new_flag"], Value::Bool(false));
    assert_eq!(json["Toggles"]["existing"], Value::Bool(true)); // preserved
    assert_eq!(
        json["FtrIO"]["Environment"],
        Value::String("Prod".to_string())
    ); // other section
}

#[test]
fn final_flush_on_drop_creates_toggles_section() {
    let path = temp_config("buffer_drop", r#"{ "FtrIO": { "Environment": "Dev" } }"#);

    {
        // A long interval so the background thread never flushes; the write must come from Drop.
        let buffer = ToggleProviderBuffer::new(path.clone(), Duration::from_secs(3600));
        buffer.stage_toggle("drop_flag", true);
    } // dropped here → final flush

    let json = read_json(&path);
    assert_eq!(json["Toggles"]["drop_flag"], Value::Bool(true)); // Toggles created
    assert_eq!(
        json["FtrIO"]["Environment"],
        Value::String("Dev".to_string())
    ); // preserved
}
