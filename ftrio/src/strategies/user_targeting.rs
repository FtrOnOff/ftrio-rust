//! User targeting: `users:alice,bob`.

use super::{ToggleDecisionStrategy, ToggleEvaluationContext};
use crate::error::ToggleError;

const USERS_PREFIX: &str = "users:";

/// Resolves `users:...` values by matching the current user id (case-insensitively) against the
/// listed ids. With no user context the toggle is off.
#[derive(Debug, Default, Clone, Copy)]
pub struct UserTargetingStrategy;

impl ToggleDecisionStrategy for UserTargetingStrategy {
    fn can_handle(&self, raw_value: &str) -> bool {
        raw_value
            .trim_start()
            .to_ascii_lowercase()
            .starts_with(USERS_PREFIX)
    }

    fn should_execute(
        &self,
        raw_value: &str,
        context: &ToggleEvaluationContext,
    ) -> Result<bool, ToggleError> {
        let trimmed = raw_value.trim_start();
        let list = &trimmed[USERS_PREFIX.len()..];
        let current_user_id = match context.current_user_id() {
            Some(user_id) => user_id,
            None => return Ok(false),
        };
        let is_targeted = list
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .any(|entry| entry.eq_ignore_ascii_case(&current_user_id));
        Ok(is_targeted)
    }
}
