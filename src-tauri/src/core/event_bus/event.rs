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
    /// the outcome (MKT-119), plus the per-asset unpriced list so it can offer
    /// manual entry (MKT-170). Distinct from the per-asset `AssetPriceUpdated`.
    AssetPriceFetchCompleted {
        /// Count of assets whose price was successfully updated.
        ok: u32,
        /// Count of assets skipped — no data, or a fetch/upsert failure.
        skipped: u32,
        /// The skipped assets, one entry each (MKT-170/171); `len() == skipped`.
        unpriced: Vec<UnpricedAsset>,
    },
    /// A currency rate was recorded, updated, or deleted (FXR-026/052/053/074).
    CurrencyRateUpdated,
}

/// One asset a price-fetch task could not price (MKT-170/171), carried in the
/// `AssetPriceFetchCompleted` payload so the frontend can list it for manual entry.
/// `last_price` / `last_price_date` describe the asset's most recently recorded
/// price and are absent when the asset has never had a price recorded.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, specta::Type)]
pub struct UnpricedAsset {
    /// The asset whose price could not be updated.
    pub asset_id: String,
    /// The asset's display name.
    pub name: String,
    /// The asset's ticker / free-form reference.
    pub reference: String,
    /// The asset's ISIN, when it has one.
    pub isin: Option<String>,
    /// ISO 4217 currency code the asset's prices are denominated in.
    pub currency: String,
    /// Most recently recorded price in the asset's native currency, i64 micros
    /// (ADR-001); absent when the asset has never had a price recorded.
    pub last_price: Option<i64>,
    /// ISO 8601 date of `last_price`; absent when there is no recorded price.
    pub last_price_date: Option<String>,
}
