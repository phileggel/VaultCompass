//! Domain events published across bounded contexts.

use serde::Serialize;

/// All possible side-effect events that can be published across the application.
/// Each variant represents a specific business event that features may need to react to.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, specta::Type, tauri_specta::Event)]
#[serde(tag = "type")]
pub enum Event {
    /// Health check event for testing/monitoring
    Health,
    /// An asset was created, updated, or deleted
    AssetUpdated,
    /// An account was created, updated, or deleted
    AccountUpdated,
    /// A category was created, updated, or deleted
    CategoryUpdated,
    /// A transaction was created, updated, or deleted (position data changed)
    TransactionUpdated,
    /// A market price was recorded or updated for an asset (MKT-026)
    AssetPriceUpdated,
    /// A price-fetch task finished: `ok` assets were updated, `skipped` were not
    /// (no data or fetch failure). Carries counts so the frontend can summarize
    /// the outcome (MKT-119). Distinct from the per-asset `AssetPriceUpdated`.
    AssetPriceFetchCompleted {
        /// Count of assets whose price was successfully updated.
        ok: u32,
        /// Count of assets skipped — no data, or a fetch/upsert failure.
        skipped: u32,
    },
    /// A currency rate was recorded, updated, or deleted (FXR-026/052/053/074).
    CurrencyRateUpdated,
}
