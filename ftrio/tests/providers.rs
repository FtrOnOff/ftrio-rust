//! Value-source providers: the environment-variable source and the first-wins composite.

mod common;

use common::MapProvider;
use ftrio::{
    CompositeToggleParser, EnvironmentVariableToggleParser, StrategyToggleParser, ToggleError,
    ToggleParser, ToggleValueProvider,
};

#[test]
fn env_var_provider_reads_ftrio_prefixed_variables() {
    std::env::set_var("FTRIO__Toggles__EnvProviderFlag", "true");
    let provider = EnvironmentVariableToggleParser::new();

    assert_eq!(
        provider.get_raw_value("EnvProviderFlag").unwrap(),
        Some("true".to_string())
    );
    assert_eq!(provider.get_raw_value("NotSetFlag").unwrap(), None);

    std::env::remove_var("FTRIO__Toggles__EnvProviderFlag");
}

#[test]
fn composite_tries_sources_in_order_first_wins() {
    let first = MapProvider::new().with_value("shared", "true");
    let second = MapProvider::new()
        .with_value("shared", "false")
        .with_value("only_second", "false");
    let composite = CompositeToggleParser::new(vec![Box::new(first), Box::new(second)]);

    // First source wins for a shared key; a key only in the second still resolves; a missing key is
    // None.
    assert_eq!(
        composite.get_raw_value("shared").unwrap(),
        Some("true".to_string())
    );
    assert_eq!(
        composite.get_raw_value("only_second").unwrap(),
        Some("false".to_string())
    );
    assert_eq!(composite.get_raw_value("nowhere").unwrap(), None);
}

#[test]
fn composite_reraises_does_not_exist_only_when_all_miss() {
    let composite = CompositeToggleParser::new(vec![
        Box::new(MapProvider::new().with_value("a", "true")),
        Box::new(MapProvider::new().with_value("b", "false")),
    ]);
    let parser = StrategyToggleParser::new(Box::new(composite), vec![], None);

    assert_eq!(parser.get_toggle_status("a"), Ok(true));
    assert_eq!(parser.get_toggle_status("b"), Ok(false));
    assert_eq!(
        parser.get_toggle_status("missing_everywhere"),
        Err(ToggleError::DoesNotExist {
            toggle_key: "missing_everywhere".to_string()
        })
    );
}
