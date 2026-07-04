//! The single idiomatic error type for the toggle read path.
//!
//! FtrIO (.NET) models failure as an exception hierarchy (`ToggleDoesNotExistException`,
//! `ToggleParsedOutOfRangeException`, `ToggleAttributeMissingException`). Rust models recoverable
//! failure as a value, so those three exceptions collapse into one enum of variants — the same
//! principle the Python port applied when it renamed `*Exception` to `*Error`. The *meaning* is
//! preserved; only the shape follows Rust.

use std::error::Error;
use std::fmt;

/// A toggle could not be read or resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToggleError {
    /// The toggle key is absent from the `Toggles` section — analogue of
    /// `ToggleDoesNotExistException`.
    DoesNotExist {
        /// The key that was looked up and not found.
        toggle_key: String,
    },
    /// A raw value could not be parsed to a decision by any strategy — analogue of
    /// `ToggleParsedOutOfRangeException`.
    ParsedOutOfRange {
        /// The raw configuration value that no strategy could handle.
        raw_value: String,
    },
    /// A method expected to carry the toggle attribute did not — analogue of
    /// `ToggleAttributeMissingException`.
    AttributeMissing {
        /// The name of the method missing its toggle attribute.
        method_name: String,
    },
}

impl fmt::Display for ToggleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToggleError::DoesNotExist { toggle_key } => write!(
                formatter,
                "toggle '{toggle_key}' does not exist in the Toggles section"
            ),
            ToggleError::ParsedOutOfRange { raw_value } => write!(
                formatter,
                "toggle value '{raw_value}' could not be parsed to a decision"
            ),
            ToggleError::AttributeMissing { method_name } => write!(
                formatter,
                "method '{method_name}' is not decorated with a toggle attribute"
            ),
        }
    }
}

impl Error for ToggleError {}
