//! Value sources: the `ToggleValueProvider` trait and its implementations.
//!
//! The environment-variable and composite sources are always available. The HTTP and Azure
//! providers live behind cargo features, mirroring the separate .NET provider projects
//! (`Providers.Http`, `Providers.AzureAppConfig`); the core stays lean, and a consumer opts in with
//! `features = ["http"]` or `["azure"]`.

mod value_provider;
pub use value_provider::{
    CompositeToggleParser, EnvironmentVariableToggleParser, ToggleValueProvider,
};

#[cfg(feature = "http")]
mod http;
#[cfg(feature = "http")]
pub use http::HttpToggleParser;

#[cfg(feature = "azure")]
mod azure;
#[cfg(feature = "azure")]
pub use azure::AzureAppConfigToggleParser;
