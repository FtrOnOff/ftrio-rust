//! The fluent builder assembles the same strategy chains as the .NET `ToggleParserBuilder`, and
//! rejects overrides without a context accessor (the `InvalidOperationException` analogue).

mod common;

use std::sync::Arc;

use common::{MapProvider, TestContext};
use ftrio::{ToggleParser, ToggleParserBuilder};

#[test]
fn builder_assembles_the_configured_strategy_chain() {
    let source = MapProvider::new()
        .with_value("full_rollout", "100%")
        .with_value("active_slot", "blue")
        .with_value("targeted", "users:alice");
    let parser = ToggleParserBuilder::new()
        .with_provider(Box::new(source))
        .with_percentage_rollout()
        .with_blue_green(
            Some("blue".to_string()),
            vec!["blue".to_string(), "green".to_string()],
        )
        .with_context_strategies()
        .with_context_accessor(Arc::new(TestContext))
        .build()
        .expect("builder should assemble");

    assert_eq!(parser.get_toggle_status("full_rollout"), Ok(true));
    assert_eq!(parser.get_toggle_status("active_slot"), Ok(true));
    assert_eq!(parser.get_toggle_status("targeted"), Ok(true));
}

#[test]
fn overrides_without_context_accessor_is_an_error() {
    let result = ToggleParserBuilder::new().with_overrides().build();
    assert!(result.is_err());
}

#[test]
fn overrides_with_context_accessor_builds() {
    let result = ToggleParserBuilder::new()
        .with_overrides()
        .with_context_accessor(Arc::new(TestContext))
        .build();
    assert!(result.is_ok());
}
