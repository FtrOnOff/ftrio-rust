//! Blue-green deployment slots: `blue` / `green`.

use super::{ToggleDecisionStrategy, ToggleEvaluationContext};
use crate::error::ToggleError;

/// Resolves a slot value (`blue`, `green`, or any configured slot) against the currently active
/// slot. The known slots and the current slot come from `FtrIO:BlueGreen:*`, so the strategy holds
/// them directly (its `can_handle` needs to recognise slot names, and `can_handle` has no context).
#[derive(Debug, Clone)]
pub struct BlueGreenStrategy {
    current_slot: Option<String>,
    known_slots: Vec<String>,
}

impl Default for BlueGreenStrategy {
    fn default() -> Self {
        BlueGreenStrategy {
            current_slot: None,
            known_slots: vec!["blue".to_string(), "green".to_string()],
        }
    }
}

impl BlueGreenStrategy {
    /// Construct from the configured current slot and the list of known slots. An empty
    /// `known_slots` falls back to the conventional `blue`/`green` pair.
    pub fn new(current_slot: Option<String>, known_slots: Vec<String>) -> Self {
        let known_slots = if known_slots.is_empty() {
            vec!["blue".to_string(), "green".to_string()]
        } else {
            known_slots
        };
        BlueGreenStrategy {
            current_slot,
            known_slots,
        }
    }
}

impl ToggleDecisionStrategy for BlueGreenStrategy {
    fn can_handle(&self, raw_value: &str) -> bool {
        let value = raw_value.trim();
        self.known_slots
            .iter()
            .any(|slot| slot.eq_ignore_ascii_case(value))
    }

    fn should_execute(
        &self,
        raw_value: &str,
        _context: &ToggleEvaluationContext,
    ) -> Result<bool, ToggleError> {
        match &self.current_slot {
            Some(current) => Ok(current.eq_ignore_ascii_case(raw_value.trim())),
            None => Ok(false),
        }
    }
}
