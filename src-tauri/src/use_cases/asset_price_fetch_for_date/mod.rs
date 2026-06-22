//! Date-scoped price fetch use case: fetches each fetchable holding's close at (or
//! carried back to) a user-picked date and stores it keyed to that date. Fully
//! separate from the latest-price auto-fetch (`asset_price_fetch`) so neither path
//! disturbs the other (ADR-017).

/// Tauri command handler for the per-account, per-date fetch.
pub mod api;
/// Use-case-specific failure codes + the wire-facing error composite.
pub mod error;
/// Orchestrator with the synchronous `fetch_for_account_on_date` method.
pub mod orchestrator;
#[cfg(test)]
mod serde_check;

pub use api::*;
pub use error::{FetchAccountAssetPricesForDateError, FetchPriceForDateTask};
pub use orchestrator::{AssetPriceFetchForDateUseCase, FetchForDateOutcome};
