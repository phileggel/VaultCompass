use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Link between an Account and an Asset with holdings data.
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
pub struct AssetAccount {
    /// Linked account ID.
    pub account_id: String,
    /// Linked asset ID.
    pub asset_id: String,
    /// Average purchase price in the asset's currency.
    pub average_price: f64,
    /// Quantity of the asset held.
    pub quantity: f64,
}

impl AssetAccount {
    /// Creates a new AssetAccount link.
    pub fn new(
        account_id: String,
        asset_id: String,
        average_price: f64,
        quantity: f64,
    ) -> Result<Self> {
        if average_price < 0.0 {
            anyhow::bail!("Average price cannot be negative");
        }
        if quantity < 0.0 {
            anyhow::bail!("Quantity cannot be negative");
        }
        Ok(Self {
            account_id,
            asset_id,
            average_price,
            quantity,
        })
    }
}

/// Interface for asset-account mapping persistence.
#[async_trait]
pub trait AssetAccountRepository: Send + Sync {
    /// Fetches all assets linked to an account.
    async fn get_by_account(&self, account_id: &str) -> Result<Vec<AssetAccount>>;
    /// Links an asset to an account or updates existing holdings.
    async fn upsert(&self, asset_account: AssetAccount) -> Result<AssetAccount>;
    /// Removes an asset from an account.
    async fn remove(&self, account_id: &str, asset_id: &str) -> Result<()>;
}
