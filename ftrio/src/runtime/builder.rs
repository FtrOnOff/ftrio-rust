//! The fluent builder that assembles a [`StrategyToggleParser`].
//!
//! Mirrors `ToggleParserBuilder`. Rust has no method overloading, so the fluent builder plus
//! `Default` reproduces the same assembled strategy chains the .NET builder produced from its
//! overloaded factory methods. Strategies are added in call order; [`BooleanStrategy`] is appended
//! last by the parser itself.

use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use crate::context::FtrIoContextAccessor;
use crate::parsing::AppSettingsToggleParser;
use crate::parsing::StrategyToggleParser;
use crate::providers::ToggleValueProvider;
use crate::strategies::{
    AbTestStrategy, AttributeRuleStrategy, BlueGreenStrategy, PercentageRolloutStrategy,
    ToggleDecisionStrategy, UserTargetingStrategy,
};

/// Building a parser with overrides but no context accessor is invalid, the Rust analogue of the
/// .NET `InvalidOperationException`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToggleParserBuilderError {
    message: String,
}

impl fmt::Display for ToggleParserBuilderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl Error for ToggleParserBuilderError {}

/// Fluent builder for a [`StrategyToggleParser`].
#[derive(Default)]
pub struct ToggleParserBuilder {
    strategies: Vec<Box<dyn ToggleDecisionStrategy>>,
    context_accessor: Option<Arc<dyn FtrIoContextAccessor>>,
    source: Option<Box<dyn ToggleValueProvider>>,
    base_path: Option<PathBuf>,
    overrides_requested: bool,
}

impl ToggleParserBuilder {
    /// Start a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add the percentage rollout strategy.
    pub fn with_percentage_rollout(mut self) -> Self {
        self.strategies.push(Box::new(PercentageRolloutStrategy));
        self
    }

    /// Add the blue-green strategy configured with the current slot and known slots.
    pub fn with_blue_green(
        mut self,
        current_slot: Option<String>,
        known_slots: Vec<String>,
    ) -> Self {
        self.strategies
            .push(Box::new(BlueGreenStrategy::new(current_slot, known_slots)));
        self
    }

    /// Add the context-aware strategies: user targeting, attribute rules, and A/B testing.
    pub fn with_context_strategies(mut self) -> Self {
        self.strategies.push(Box::new(UserTargetingStrategy));
        self.strategies.push(Box::new(AttributeRuleStrategy));
        self.strategies.push(Box::new(AbTestStrategy));
        self
    }

    /// Add an arbitrary custom strategy.
    pub fn with_strategy(mut self, strategy: Box<dyn ToggleDecisionStrategy>) -> Self {
        self.strategies.push(strategy);
        self
    }

    /// Enable per-user overrides. Requires a context accessor; `build` fails otherwise.
    pub fn with_overrides(mut self) -> Self {
        self.overrides_requested = true;
        self
    }

    /// Set the context accessor (user id + attributes) used by overrides and context strategies.
    pub fn with_context_accessor(mut self, accessor: Arc<dyn FtrIoContextAccessor>) -> Self {
        self.context_accessor = Some(accessor);
        self
    }

    /// Set an explicit value source. Without one, the file source at the configured base path is
    /// used.
    pub fn with_provider(mut self, source: Box<dyn ToggleValueProvider>) -> Self {
        self.source = Some(source);
        self
    }

    /// Set the base path of the `appsettings.json` file source.
    pub fn with_base_path(mut self, base_path: impl Into<PathBuf>) -> Self {
        self.base_path = Some(base_path.into());
        self
    }

    /// Assemble the [`StrategyToggleParser`].
    ///
    /// Fails if overrides were requested without a context accessor, the
    /// `InvalidOperationException` analogue.
    pub fn build(self) -> Result<StrategyToggleParser, ToggleParserBuilderError> {
        if self.overrides_requested && self.context_accessor.is_none() {
            return Err(ToggleParserBuilderError {
                message: "with_overrides() requires a context accessor to key overrides by user"
                    .to_string(),
            });
        }

        let source = self.source.unwrap_or_else(|| {
            let base_path = self
                .base_path
                .clone()
                .unwrap_or_else(|| PathBuf::from("appsettings.json"));
            Box::new(AppSettingsToggleParser::new(base_path))
        });

        Ok(StrategyToggleParser::new(
            source,
            self.strategies,
            self.context_accessor,
        ))
    }
}
