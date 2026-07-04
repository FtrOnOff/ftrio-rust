//! How you wire FtrIO up and call it: the ambient parser instance, the fluent builder, the explicit
//! functional API, and the write-back buffer.

mod buffer;
mod builder;
mod functional;
pub mod parser_provider;

pub use buffer::{ToggleBuffer, ToggleProviderBuffer};
pub use builder::{ToggleParserBuilder, ToggleParserBuilderError};
pub use functional::{
    execute_if_toggle_on, execute_if_toggle_on_async, try_execute_if_toggle_on,
    try_execute_if_toggle_on_async,
};
