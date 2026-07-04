//! The strategy-routing parser: the resolution engine that renders the full value grammar.
//!
//! Mirrors `StrategyToggleParser`. It reads a raw value from a [`ToggleValueProvider`] source and
//! routes it through an ordered strategy chain. Resolution order is the FtrIO contract:
//! 1. No config on disk at all → `true` (offline-safe default).
//! 2. A per-user override wins unconditionally.
//! 3. Otherwise the first strategy whose `can_handle` matches decides; [`BooleanStrategy`] is always
//!    appended last as the fallback.

use std::path::Path;
use std::sync::Arc;

use super::overrides::OverrideResolver;
use super::toggle_parser::{read_blue_green_settings, AppSettingsToggleParser, ToggleParser};
use crate::context::FtrIoContextAccessor;
use crate::error::ToggleError;
use crate::providers::ToggleValueProvider;
use crate::strategies::{
    parse_boolean_value, AbTestStrategy, AttributeRuleStrategy, BlueGreenStrategy, BooleanStrategy,
    PercentageRolloutStrategy, ToggleDecisionStrategy, ToggleEvaluationContext,
    UserTargetingStrategy,
};

/// Routes raw toggle values through an ordered strategy chain, with overrides winning first.
pub struct StrategyToggleParser {
    source: Box<dyn ToggleValueProvider>,
    strategies: Vec<Box<dyn ToggleDecisionStrategy>>,
    context_accessor: Option<Arc<dyn FtrIoContextAccessor>>,
    override_resolver: OverrideResolver,
}

impl StrategyToggleParser {
    /// Construct from a source, a chain of strategies, and an optional context accessor.
    ///
    /// [`BooleanStrategy`] is appended to the chain here so it is *always* the last fallback,
    /// regardless of how the chain was assembled, the single guarantee the FtrIO contract makes
    /// about the strategy order.
    pub fn new(
        source: Box<dyn ToggleValueProvider>,
        mut strategies: Vec<Box<dyn ToggleDecisionStrategy>>,
        context_accessor: Option<Arc<dyn FtrIoContextAccessor>>,
    ) -> Self {
        strategies.push(Box::new(BooleanStrategy));
        let override_resolver = OverrideResolver::new(context_accessor.clone());
        StrategyToggleParser {
            source,
            strategies,
            context_accessor,
            override_resolver,
        }
    }

    /// Build the conventional default parser over an `appsettings.json` file: the full strategy
    /// chain (percentage, blue-green from config, user targeting, attribute rules, A/B) with no
    /// context accessor. This is what the ambient provider hands out when nothing was configured.
    pub fn from_app_settings(base_path: impl AsRef<Path>) -> Self {
        let base_path = base_path.as_ref();
        let (current_slot, known_slots) = read_blue_green_settings(base_path);
        let source = Box::new(AppSettingsToggleParser::new(base_path.to_path_buf()));
        let strategies: Vec<Box<dyn ToggleDecisionStrategy>> = vec![
            Box::new(PercentageRolloutStrategy),
            Box::new(BlueGreenStrategy::new(current_slot, known_slots)),
            Box::new(UserTargetingStrategy),
            Box::new(AttributeRuleStrategy),
            Box::new(AbTestStrategy),
        ];
        StrategyToggleParser::new(source, strategies, None)
    }
}

impl ToggleParser for StrategyToggleParser {
    fn get_toggle_status(&self, toggle_key: &str) -> Result<bool, ToggleError> {
        if !self.source.config_present() {
            return Ok(true); // offline-safe default
        }

        if let Some(overridden) = self
            .override_resolver
            .resolve(self.source.as_ref(), toggle_key)
        {
            return Ok(overridden);
        }

        let raw_value = match self.source.get_raw_value(toggle_key)? {
            Some(value) => value,
            None => {
                return Err(ToggleError::DoesNotExist {
                    toggle_key: toggle_key.to_string(),
                })
            }
        };

        let context = ToggleEvaluationContext {
            toggle_key,
            context_accessor: self
                .context_accessor
                .as_ref()
                .map(|accessor| accessor.as_ref()),
        };

        for strategy in &self.strategies {
            if strategy.can_handle(&raw_value) {
                return strategy.should_execute(&raw_value, &context);
            }
        }

        Err(ToggleError::ParsedOutOfRange {
            raw_value: raw_value.clone(),
        })
    }

    fn parse_bool_value_from_source(&self, toggle_key: &str) -> Result<bool, ToggleError> {
        if !self.source.config_present() {
            return Ok(true);
        }
        match self.source.get_raw_value(toggle_key)? {
            Some(raw_value) => parse_boolean_value(&raw_value),
            None => Err(ToggleError::DoesNotExist {
                toggle_key: toggle_key.to_string(),
            }),
        }
    }

    fn get_override(&self, toggle_key: &str) -> Option<bool> {
        self.override_resolver
            .resolve(self.source.as_ref(), toggle_key)
    }
}
