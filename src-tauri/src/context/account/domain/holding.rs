use super::error::HoldingDomainError;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

/// Current state of a financial position: an asset held within an account (ADR-002).
/// All financial fields are stored as i64 micro-units (ADR-001).
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
pub struct Holding {
    /// Unique identifier.
    pub id: String,
    /// The account holding the asset.
    pub account_id: String,
    /// The financial asset held.
    pub asset_id: String,
    /// Current number of units held (micro-units: value × 10^6).
    pub quantity: i64,
    /// Volume-weighted average purchase price in account currency (micro-units).
    pub average_price: i64,
    /// Cumulative realized P&L from all sell transactions (micro-units, ACD-045).
    pub total_realized_pnl: i64,
    /// ISO date of the most recent sell transaction, if any (ACD-043).
    pub last_sold_date: Option<String>,
}

impl Holding {
    /// Creates a new Holding with a generated ID.
    pub fn new(
        account_id: String,
        asset_id: String,
        quantity: i64,
        average_price: i64,
        total_realized_pnl: i64,
        last_sold_date: Option<String>,
    ) -> Result<Self> {
        Self::validate(quantity, average_price)?;
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            account_id,
            asset_id,
            quantity,
            average_price,
            total_realized_pnl,
            last_sold_date,
        })
    }

    /// Creates a Holding with a provided ID (used by use case for upsert).
    pub fn with_id(
        id: String,
        account_id: String,
        asset_id: String,
        quantity: i64,
        average_price: i64,
        total_realized_pnl: i64,
        last_sold_date: Option<String>,
    ) -> Result<Self> {
        Self::validate(quantity, average_price)?;
        Ok(Self {
            id,
            account_id,
            asset_id,
            quantity,
            average_price,
            total_realized_pnl,
            last_sold_date,
        })
    }

    /// Reconstructs a Holding from storage without validation.
    pub fn restore(
        id: String,
        account_id: String,
        asset_id: String,
        quantity: i64,
        average_price: i64,
        total_realized_pnl: i64,
        last_sold_date: Option<String>,
    ) -> Self {
        Self {
            id,
            account_id,
            asset_id,
            quantity,
            average_price,
            total_realized_pnl,
            last_sold_date,
        }
    }

    fn validate(quantity: i64, average_price: i64) -> Result<()> {
        if quantity < 0 {
            return Err(HoldingDomainError::NegativeQuantity.into());
        }
        if average_price < 0 {
            return Err(HoldingDomainError::NegativeAveragePrice.into());
        }
        Ok(())
    }
}

/// Interface for holding persistence.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait HoldingRepository: Send + Sync {
    /// Fetches all holdings for a given account.
    async fn get_by_account(&self, account_id: &str) -> Result<Vec<Holding>>;
    /// Fetches a holding by account and asset.
    async fn get_by_account_asset(
        &self,
        account_id: &str,
        asset_id: &str,
    ) -> Result<Option<Holding>>;
    /// Creates or updates a holding atomically (INSERT OR REPLACE).
    async fn upsert(&self, holding: Holding) -> Result<Holding>;
    /// Deletes a holding by ID.
    async fn delete(&self, id: &str) -> Result<()>;
    /// Deletes a holding by account and asset (used when no transactions remain, TRX-034).
    async fn delete_by_account_asset(&self, account_id: &str, asset_id: &str) -> Result<()>;
    /// Returns true if any holding for the given asset has quantity > 0 (OQ-6).
    async fn has_active_holdings_for_asset(&self, asset_id: &str) -> Result<bool>;
}
