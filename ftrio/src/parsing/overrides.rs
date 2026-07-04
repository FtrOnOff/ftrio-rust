//! Per-user override resolution.
//!
//! Mirrors `OverrideResolver`. Overrides win **unconditionally**, before any strategy runs, but
//! only when there is a user context to key them by. With no context accessor (or no current user),
//! overrides are silently ignored and evaluation falls through to the strategy chain.

use std::sync::Arc;

use crate::context::FtrIoContextAccessor;
use crate::providers::ToggleValueProvider;

/// Resolves the per-user override for a toggle by combining the ambient user identity with the
/// source's override table.
pub struct OverrideResolver {
    context_accessor: Option<Arc<dyn FtrIoContextAccessor>>,
}

impl OverrideResolver {
    /// Construct from the (optional) context accessor.
    pub fn new(context_accessor: Option<Arc<dyn FtrIoContextAccessor>>) -> Self {
        OverrideResolver { context_accessor }
    }

    /// The override decision for `toggle_key`, or `None` when there is no user context or no override
    /// entry for that user.
    pub fn resolve(&self, source: &dyn ToggleValueProvider, toggle_key: &str) -> Option<bool> {
        let user_id = self.context_accessor.as_ref()?.get_user_id()?;
        source.get_override(toggle_key, &user_id)
    }
}
