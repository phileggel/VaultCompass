//! Account Details use case: cross-context read orchestrating account + asset data (ADR-003).

mod api;
/// Typed error enum for the AccountDetails use case.
pub mod error;
mod orchestrator;

pub use api::*;
pub use error::AccountDetailsError;
pub use orchestrator::*;
