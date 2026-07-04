//! The six decision strategies, the behavioural heart of FtrIO.
//!
//! A raw configuration value is routed through an ordered chain; the first strategy whose
//! [`ToggleDecisionStrategy::can_handle`] returns `true` decides the outcome. [`BooleanStrategy`] is
//! always appended last as the fallback, so a plain `true`/`false`/`1`/`0` always resolves even if
//! no richer grammar matched.

mod ab_test;
mod attribute_rule;
mod blue_green;
mod boolean;
mod percentage;
mod user_targeting;

pub use ab_test::{compute_bucket, AbTestStrategy};
pub use attribute_rule::AttributeRuleStrategy;
pub use blue_green::BlueGreenStrategy;
pub use boolean::BooleanStrategy;
pub use percentage::PercentageRolloutStrategy;
pub use user_targeting::UserTargetingStrategy;

use crate::context::FtrIoContextAccessor;
use crate::error::ToggleError;

/// The context a strategy evaluates against: the toggle's own key (needed for stable A/B bucketing)
/// and the ambient user/attribute context (needed for targeting).
#[derive(Clone, Copy)]
pub struct ToggleEvaluationContext<'a> {
    /// The toggle key being resolved. A/B bucketing hashes this so a user's bucket is stable per
    /// toggle.
    pub toggle_key: &'a str,
    /// The ambient context, or `None` when the parser was configured without one.
    pub context_accessor: Option<&'a dyn FtrIoContextAccessor>,
}

impl<'a> ToggleEvaluationContext<'a> {
    /// Convenience accessor for the current user id.
    pub fn current_user_id(&self) -> Option<String> {
        self.context_accessor
            .and_then(|accessor| accessor.get_user_id())
    }

    /// Convenience accessor for a named attribute.
    pub fn attribute(&self, attribute_name: &str) -> Option<String> {
        self.context_accessor
            .and_then(|accessor| accessor.get_attribute(attribute_name))
    }
}

/// A single decision strategy. Mirrors `IToggleDecisionStrategy`.
pub trait ToggleDecisionStrategy: Send + Sync {
    /// Whether this strategy recognises the raw value's grammar.
    fn can_handle(&self, raw_value: &str) -> bool;

    /// Render the on/off decision for a value this strategy has claimed via `can_handle`.
    fn should_execute(
        &self,
        raw_value: &str,
        context: &ToggleEvaluationContext,
    ) -> Result<bool, ToggleError>;
}

/// Parse a boolean toggle value, accepting `true`/`false`/`1`/`0` case-insensitively. Shared by the
/// boolean strategy and the parsers' `parse_bool_value_from_source`.
pub(crate) fn parse_boolean_value(raw_value: &str) -> Result<bool, ToggleError> {
    match raw_value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => Err(ToggleError::ParsedOutOfRange {
            raw_value: raw_value.to_string(),
        }),
    }
}

/// Whether a raw value is a recognised boolean literal.
pub(crate) fn is_boolean_value(raw_value: &str) -> bool {
    matches!(
        raw_value.trim().to_ascii_lowercase().as_str(),
        "true" | "false" | "1" | "0"
    )
}
