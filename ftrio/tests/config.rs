//! Boolean + config behaviour, including the offline-safe default.

mod common;

use common::temp_config;
use ftrio::{AppSettingsToggleParser, StrategyToggleParser, ToggleError, ToggleParser};

const MIXED_CONFIG: &str = r#"{
  "Toggles": {
    "flag_true": true,
    "flag_false": false,
    "flag_one": 1,
    "flag_zero": 0,
    "flag_upper": "TRUE",
    "flag_junk": "ASDF"
  }
}"#;

#[test]
fn booleans_and_numbers_resolve_case_insensitively() {
    let path = temp_config("bool_mixed", MIXED_CONFIG);
    let parser = AppSettingsToggleParser::new(path);

    // JSON booleans, JSON numbers (1/0), and a mixed-case string all coerce correctly.
    assert_eq!(parser.get_toggle_status("flag_true"), Ok(true));
    assert_eq!(parser.get_toggle_status("flag_false"), Ok(false));
    assert_eq!(parser.get_toggle_status("flag_one"), Ok(true));
    assert_eq!(parser.get_toggle_status("flag_zero"), Ok(false));
    assert_eq!(parser.get_toggle_status("flag_upper"), Ok(true));
}

#[test]
fn unparseable_value_is_parsed_out_of_range() {
    let path = temp_config("bool_junk", MIXED_CONFIG);
    let parser = AppSettingsToggleParser::new(path);

    assert_eq!(
        parser.get_toggle_status("flag_junk"),
        Err(ToggleError::ParsedOutOfRange {
            raw_value: "ASDF".to_string()
        })
    );
}

#[test]
fn absent_key_does_not_exist() {
    let path = temp_config("bool_absent", MIXED_CONFIG);
    let parser = AppSettingsToggleParser::new(path);

    assert_eq!(
        parser.get_toggle_status("no_such_toggle"),
        Err(ToggleError::DoesNotExist {
            toggle_key: "no_such_toggle".to_string()
        })
    );
}

#[test]
fn no_config_file_defaults_every_toggle_to_on() {
    // No appsettings.json on disk at all → the offline-safe default is on, for any key.
    let parser =
        AppSettingsToggleParser::new("this_directory_does_not_exist_ftrio/appsettings.json");
    assert_eq!(parser.get_toggle_status("anything"), Ok(true));
    assert_eq!(parser.get_toggle_status("another"), Ok(true));

    // The same holds through the full strategy parser.
    let strategy_parser = StrategyToggleParser::from_app_settings(
        "this_directory_does_not_exist_ftrio/appsettings.json",
    );
    assert_eq!(strategy_parser.get_toggle_status("anything"), Ok(true));
}
