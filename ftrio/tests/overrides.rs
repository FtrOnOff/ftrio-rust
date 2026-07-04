//! Per-user overrides win unconditionally, before any strategy, but only with a user context.

mod common;

use std::sync::Arc;

use common::{temp_config, MapProvider, TestContext};
use ftrio::{AppSettingsToggleParser, StrategyToggleParser, ToggleParser};

#[test]
fn override_wins_before_the_strategy_chain() {
    // The strategy value says off, but a per-user override for the current user says on.
    let source = MapProvider::new()
        .with_value("feature", "false")
        .with_override("feature", "alice", true);
    let parser = StrategyToggleParser::new(Box::new(source), vec![], Some(Arc::new(TestContext)));

    assert_eq!(parser.get_toggle_status("feature"), Ok(true));
}

#[test]
fn override_is_ignored_without_user_context() {
    let source = MapProvider::new()
        .with_value("feature", "false")
        .with_override("feature", "alice", true);
    // No context accessor → no user to key the override by → fall through to the strategy value.
    let parser = StrategyToggleParser::new(Box::new(source), vec![], None);

    assert_eq!(parser.get_toggle_status("feature"), Ok(false));
}

const OVERRIDE_CONFIG: &str = r#"{
  "Toggles": {
    "f_bool": "false",
    "f_num": "false",
    "f_str": "false",
    "f_zero": "true"
  },
  "TogglesOverrides": {
    "f_bool": { "alice": true },
    "f_num": { "alice": 1 },
    "f_str": { "alice": "true" },
    "f_zero": { "alice": 0 }
  }
}"#;

#[test]
fn override_accepts_bool_number_and_string_forms() {
    let path = temp_config("overrides", OVERRIDE_CONFIG);
    let parser = AppSettingsToggleParser::new(path).with_context_accessor(Arc::new(TestContext));

    assert_eq!(parser.get_toggle_status("f_bool"), Ok(true)); // JSON true
    assert_eq!(parser.get_toggle_status("f_num"), Ok(true)); // JSON 1
    assert_eq!(parser.get_toggle_status("f_str"), Ok(true)); // "true"
    assert_eq!(parser.get_toggle_status("f_zero"), Ok(false)); // JSON 0
}
