use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

/// A historical price point for an asset.
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
pub struct AssetPrice {
    /// Unique identifier for the price record.
    pub id: String,
    /// ID of the linked asset.
    pub asset_id: String,
    /// Valuation at the specific date.
    pub price: f64,
    /// ISO 8601 formatted date (YYYY-MM-DD).
    pub date: String,
}

impl AssetPrice {
    /// Creates a new AssetPrice.
    pub fn new(asset_id: String, price: f64, date: String) -> Result<Self> {
        Self::validate(price, &date)?;
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            asset_id,
            price,
            date,
        })
    }

    fn validate(price: f64, date: &str) -> Result<()> {
        if price <= 0.0 {
            anyhow::bail!("Price must be greater than zero");
        }
        // Basic date format validation (YYYY-MM-DD)
        if date.len() != 10 || !matches!(date.chars().nth(4), Some('-')) {
            anyhow::bail!("Invalid date format. Expected YYYY-MM-DD");
        }
        Ok(())
    }

    /// Creates a new AssetPrice from storage.
    pub fn from_storage(id: String, asset_id: String, price: f64, date: String) -> Self {
        Self {
            id,
            asset_id,
            price,
            date,
        }
    }
}

/// Interface for price data persistence.
#[async_trait]
pub trait PriceRepository: Send + Sync {
    /// Fetches all prices for an asset.
    async fn get_by_asset(&self, asset_id: &str) -> Result<Vec<AssetPrice>>;
    /// Fetches prices within a specific date range.
    async fn get_by_asset_and_date_range(
        &self,
        asset_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<AssetPrice>>;
    /// Persists a new price point.
    async fn create(&self, price: AssetPrice) -> Result<AssetPrice>;
}
