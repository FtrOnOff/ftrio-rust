//! Percentage rollout: `50%`.

use rand::Rng;

use super::{ToggleDecisionStrategy, ToggleEvaluationContext};
use crate::error::ToggleError;

/// Resolves values like `50%`. The `0%` and `100%` boundaries are deterministic (always off / always
/// on); values in between are a probabilistic per-call roll, matching the .NET `Random`-based path.
#[derive(Debug, Default, Clone, Copy)]
pub struct PercentageRolloutStrategy;

/// Extract the integer percentage from a `NN%` value, if it is well-formed.
fn parse_percentage(raw_value: &str) -> Option<u32> {
    let trimmed = raw_value.trim();
    let digits = trimmed.strip_suffix('%')?;
    digits.trim().parse::<u32>().ok()
}

impl ToggleDecisionStrategy for PercentageRolloutStrategy {
    fn can_handle(&self, raw_value: &str) -> bool {
        parse_percentage(raw_value).is_some()
    }

    fn should_execute(
        &self,
        raw_value: &str,
        _context: &ToggleEvaluationContext,
    ) -> Result<bool, ToggleError> {
        let percentage =
            parse_percentage(raw_value).ok_or_else(|| ToggleError::ParsedOutOfRange {
                raw_value: raw_value.to_string(),
            })?;

        if percentage == 0 {
            return Ok(false);
        }
        if percentage >= 100 {
            return Ok(true);
        }
        let roll = rand::thread_rng().gen_range(0..100);
        Ok(roll < percentage)
    }
}
