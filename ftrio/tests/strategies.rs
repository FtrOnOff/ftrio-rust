//! Strategy behaviour: percentage boundaries, blue-green, user targeting, attribute operators, and
//! the always-last boolean fallback.

mod common;

use common::{EmptyContext, MapProvider, TestContext};
use ftrio::strategies::{
    AttributeRuleStrategy, BlueGreenStrategy, PercentageRolloutStrategy, ToggleDecisionStrategy,
    ToggleEvaluationContext, UserTargetingStrategy,
};
use ftrio::{FtrIoContextAccessor, StrategyToggleParser, ToggleError, ToggleParser};

fn context_with<'a>(
    accessor: &'a dyn FtrIoContextAccessor,
    key: &'a str,
) -> ToggleEvaluationContext<'a> {
    ToggleEvaluationContext {
        toggle_key: key,
        context_accessor: Some(accessor),
    }
}

#[test]
fn percentage_boundaries_are_deterministic() {
    let strategy = PercentageRolloutStrategy;
    let accessor = EmptyContext;
    let context = context_with(&accessor, "feature");

    assert!(strategy.can_handle("50%"));
    assert!(!strategy.can_handle("not-a-percentage"));
    assert_eq!(strategy.should_execute("0%", &context), Ok(false));
    assert_eq!(strategy.should_execute("100%", &context), Ok(true));
}

#[test]
fn blue_green_resolves_against_current_slot() {
    let strategy = BlueGreenStrategy::new(Some("blue".to_string()), vec![]);
    let accessor = EmptyContext;
    let context = context_with(&accessor, "feature");

    assert!(strategy.can_handle("blue"));
    assert!(strategy.can_handle("GREEN")); // known slot, case-insensitive
    assert!(!strategy.can_handle("purple"));
    assert_eq!(strategy.should_execute("blue", &context), Ok(true));
    assert_eq!(strategy.should_execute("green", &context), Ok(false));
}

#[test]
fn user_targeting_matches_case_insensitively() {
    let strategy = UserTargetingStrategy;
    let accessor = TestContext; // user "alice"
    let context = context_with(&accessor, "feature");

    assert!(strategy.can_handle("users:alice,bob"));
    assert_eq!(
        strategy.should_execute("users:ALICE,bob", &context),
        Ok(true)
    );
    assert_eq!(
        strategy.should_execute("users:carol,dave", &context),
        Ok(false)
    );

    // No user context → off.
    let empty = EmptyContext;
    let empty_context = context_with(&empty, "feature");
    assert_eq!(
        strategy.should_execute("users:alice", &empty_context),
        Ok(false)
    );
}

#[test]
fn attribute_rules_cover_every_operator() {
    let strategy = AttributeRuleStrategy;
    let accessor = TestContext; // plan = premium
    let context = context_with(&accessor, "feature");

    let cases: &[(&str, bool)] = &[
        ("attribute:plan equals premium", true),
        ("attribute:plan notEquals premium", false),
        ("attribute:plan notEquals basic", true),
        ("attribute:plan startsWith prem", true),
        ("attribute:plan endsWith ium", true),
        ("attribute:plan contains emi", true),
        ("attribute:plan in premium,gold", true),
        ("attribute:plan notIn premium,gold", false),
        ("attribute:plan in gold,silver", false),
    ];
    for (rule, expected) in cases {
        assert_eq!(
            strategy.should_execute(rule, &context),
            Ok(*expected),
            "rule `{rule}` should resolve to {expected}"
        );
    }
}

#[test]
fn boolean_strategy_is_always_the_last_fallback() {
    // A parser assembled with no strategies still resolves plain booleans, because BooleanStrategy
    // is always appended last — and a value no strategy can handle surfaces ParsedOutOfRange.
    let source = MapProvider::new()
        .with_value("plain_true", "true")
        .with_value("plain_zero", "0")
        .with_value("garbage", "ASDF");
    let parser = StrategyToggleParser::new(Box::new(source), vec![], None);

    assert_eq!(parser.get_toggle_status("plain_true"), Ok(true));
    assert_eq!(parser.get_toggle_status("plain_zero"), Ok(false));
    assert_eq!(
        parser.get_toggle_status("garbage"),
        Err(ToggleError::ParsedOutOfRange {
            raw_value: "ASDF".to_string()
        })
    );
}
