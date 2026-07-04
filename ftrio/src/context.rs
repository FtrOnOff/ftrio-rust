//! The ambient request/user context the context-aware strategies read from.
//!
//! Mirrors `IFtrIOContextAccessor`. Acronyms are one word per `clippy::upper_case_acronyms`, so the
//! trait is `FtrIoContextAccessor`, not `FtrIOContextAccessor`. User targeting, attribute rules, and
//! A/B bucketing all consult it; a parser configured without one simply behaves context-free (user
//! lists never match, overrides are ignored, A/B falls back to a probabilistic roll).

/// Supplies the current user identity and arbitrary attributes for strategy evaluation.
///
/// It is `Send + Sync` because the ambient parser is shared across threads.
pub trait FtrIoContextAccessor: Send + Sync {
    /// The current user's id, or `None` when there is no user context (e.g. a background job).
    fn get_user_id(&self) -> Option<String>;

    /// An arbitrary attribute of the current context (e.g. `plan`, `country`), used by
    /// attribute-rule targeting. `None` when the attribute is not set.
    fn get_attribute(&self, attribute_name: &str) -> Option<String>;
}
