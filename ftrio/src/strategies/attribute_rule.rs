//! Attribute rules: `attribute:plan equals premium`.

use super::{ToggleDecisionStrategy, ToggleEvaluationContext};
use crate::error::ToggleError;

const ATTRIBUTE_PREFIX: &str = "attribute:";

/// Resolves `attribute:<name> <operator> <value>` rules against the context's attributes.
///
/// Supported operators: `equals`, `notEquals`, `startsWith`, `endsWith`, `contains`, `in`, `notIn`
/// (the last two take a comma-separated list). All comparison is case-insensitive. When the
/// attribute is absent from the context, the negative operators (`notEquals`, `notIn`) hold and the
/// rest do not, the logically consistent reading of "the attribute is not X".
#[derive(Debug, Default, Clone, Copy)]
pub struct AttributeRuleStrategy;

impl ToggleDecisionStrategy for AttributeRuleStrategy {
    fn can_handle(&self, raw_value: &str) -> bool {
        raw_value
            .trim_start()
            .to_ascii_lowercase()
            .starts_with(ATTRIBUTE_PREFIX)
    }

    fn should_execute(
        &self,
        raw_value: &str,
        context: &ToggleEvaluationContext,
    ) -> Result<bool, ToggleError> {
        let body = raw_value.trim_start()[ATTRIBUTE_PREFIX.len()..].trim();
        let tokens: Vec<&str> = body.split_whitespace().collect();
        if tokens.len() < 2 {
            return Err(ToggleError::ParsedOutOfRange {
                raw_value: raw_value.to_string(),
            });
        }
        let attribute_name = tokens[0];
        let operator = tokens[1].to_ascii_lowercase();
        let expected_value = tokens[2..].join(" ");

        let operator_is_known = matches!(
            operator.as_str(),
            "equals" | "notequals" | "startswith" | "endswith" | "contains" | "in" | "notin"
        );
        if !operator_is_known {
            return Err(ToggleError::ParsedOutOfRange {
                raw_value: raw_value.to_string(),
            });
        }

        let decision = match context.attribute(attribute_name) {
            None => matches!(operator.as_str(), "notequals" | "notin"),
            Some(actual_value) => evaluate_operator(&operator, &actual_value, &expected_value),
        };
        Ok(decision)
    }
}

/// Apply a (known) operator to the present attribute value.
fn evaluate_operator(operator: &str, actual: &str, expected: &str) -> bool {
    let actual_lower = actual.to_ascii_lowercase();
    let expected_lower = expected.to_ascii_lowercase();
    match operator {
        "equals" => actual_lower == expected_lower,
        "notequals" => actual_lower != expected_lower,
        "startswith" => actual_lower.starts_with(&expected_lower),
        "endswith" => actual_lower.ends_with(&expected_lower),
        "contains" => actual_lower.contains(&expected_lower),
        "in" => list_contains(expected, actual),
        "notin" => !list_contains(expected, actual),
        _ => false,
    }
}

/// Whether the comma-separated `list` contains `value` (case-insensitively).
fn list_contains(list: &str, value: &str) -> bool {
    list.split(',')
        .map(str::trim)
        .any(|entry| entry.eq_ignore_ascii_case(value))
}
