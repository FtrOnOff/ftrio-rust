//! The always-last fallback strategy: a plain boolean literal.

use super::{
    is_boolean_value, parse_boolean_value, ToggleDecisionStrategy, ToggleEvaluationContext,
};
use crate::error::ToggleError;

/// Resolves `true`/`false`/`1`/`0` (case-insensitive). Appended last to every chain so a boolean
/// value always resolves even when no richer grammar claimed it.
#[derive(Debug, Default, Clone, Copy)]
pub struct BooleanStrategy;

impl ToggleDecisionStrategy for BooleanStrategy {
    fn can_handle(&self, raw_value: &str) -> bool {
        is_boolean_value(raw_value)
    }

    fn should_execute(
        &self,
        raw_value: &str,
        _context: &ToggleEvaluationContext,
    ) -> Result<bool, ToggleError> {
        parse_boolean_value(raw_value)
    }
}
