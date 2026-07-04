//! The explicit functional API, the `FeatureToggle<T>` analogue.
//!
//! `execute_if_toggle_on(closure, key)` runs the closure only when the toggle is on, otherwise
//! returns `T::default()`. The `try_*` forms surface the `Result` for callers who want to handle a
//! misconfiguration instead of panicking.
//!
//! **Honest divergence:** a Rust closure has no name, so the ".NET derives the key from the method
//! name" branch has no analogue here, the functional API *requires* an explicit key. Name
//! derivation lives entirely in the `#[toggle]` macro, where the function name is available at
//! expansion time. Recorded in `PORTING_NOTES.md`.

use std::future::Future;

use crate::error::ToggleError;
use crate::toggle_parser_provider;

/// Run `closure` when the toggle is on; otherwise return `T::default()`. Panics on a
/// misconfiguration, mirroring the `#[toggle]` macro's behaviour.
pub fn execute_if_toggle_on<T, F>(closure: F, toggle_key: &str) -> T
where
    T: Default,
    F: FnOnce() -> T,
{
    match try_execute_if_toggle_on(closure, toggle_key) {
        Ok(value) => value,
        Err(error) => panic!("FtrIO: toggle '{toggle_key}' could not be resolved: {error}"),
    }
}

/// `Result`-returning form of [`execute_if_toggle_on`].
pub fn try_execute_if_toggle_on<T, F>(closure: F, toggle_key: &str) -> Result<T, ToggleError>
where
    T: Default,
    F: FnOnce() -> T,
{
    if toggle_parser_provider::instance().get_toggle_status(toggle_key)? {
        Ok(closure())
    } else {
        Ok(T::default())
    }
}

/// Async form: the gating check runs synchronously *before* the future is produced, so a
/// misconfiguration surfaces at the call site, not as a faulted future.
pub async fn execute_if_toggle_on_async<T, F, Fut>(closure: F, toggle_key: &str) -> T
where
    T: Default,
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    match try_execute_if_toggle_on_async(closure, toggle_key).await {
        Ok(value) => value,
        Err(error) => panic!("FtrIO: toggle '{toggle_key}' could not be resolved: {error}"),
    }
}

/// `Result`-returning form of [`execute_if_toggle_on_async`]. The toggle status is checked
/// synchronously before awaiting the closure's future.
pub async fn try_execute_if_toggle_on_async<T, F, Fut>(
    closure: F,
    toggle_key: &str,
) -> Result<T, ToggleError>
where
    T: Default,
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    if toggle_parser_provider::instance().get_toggle_status(toggle_key)? {
        Ok(closure().await)
    } else {
        Ok(T::default())
    }
}
