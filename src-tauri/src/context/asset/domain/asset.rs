use anyhow::Result;
use async_trait::async_trait;
use iso_currency::Currency;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::str::FromStr;
use uuid::Uuid;

use super::category::AssetCategory;

/// Represents the classification of an asset.

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Default,
    Clone,
    Type,
    PartialEq,
    Eq,
    strum_macros::Display,
    strum_macros::EnumString,
)]
pub enum AssetClass {
    /// Real estate properties or REITs.
    RealEstate,
    /// Fiat currency or highly liquid equivalents.
    #[default]
    Cash,
    /// Individual company equities.
    Stocks,
    /// Fixed income securities.
    Bonds,
    /// Exchange Traded Funds.
    ETF,
    /// Managed investment funds.
    MutualFunds,
    /// Cryptocurrencies or other blockchain-based assets.
    DigitalAsset,
}

impl AssetClass {
    /// Returns the default risk level for this asset class (R3).
    pub fn default_risk(&self) -> u8 {
        match self {
            AssetClass::Cash => 1,
            AssetClass::Bonds => 2,
            AssetClass::RealEstate => 2,
            AssetClass::MutualFunds => 3,
            AssetClass::ETF => 3,
            AssetClass::Stocks => 4,
            AssetClass::DigitalAsset => 5,
        }
    }
}

/// A financial instrument or resource held by a user.
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
pub struct Asset {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Asset classification.
    pub class: AssetClass,
    /// Category link.
    pub category: AssetCategory,
    /// ISO 4217 currency code.
    pub currency: String,
    /// Risk score from 1 to 5.
    pub risk_level: u8,
    /// Identifier like ticker or ISIN.
    pub reference: String,
    /// Whether the asset is archived (soft-archived, reversible).
    pub is_archived: bool,
}

impl Asset {
    /// Creates a new Asset.
    pub fn new(
        name: String,
        class: AssetClass,
        category: AssetCategory,
        currency: String,
        risk_level: u8,
        reference: String,
    ) -> Result<Self> {
        Self::validate(&name, risk_level, &currency, &reference)?;

        let reference = reference.trim().to_uppercase();

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            name,
            class,
            category,
            currency,
            risk_level,
            reference,
            is_archived: false,
        })
    }

    /// Reconstructs an Asset with a known ID (used for updates).
    #[allow(clippy::too_many_arguments)]
    pub fn with_id(
        asset_id: String,
        name: String,
        class: AssetClass,
        category: AssetCategory,
        currency: String,
        risk_level: u8,
        reference: String,
        is_archived: bool,
    ) -> Result<Self> {
        Self::validate(&name, risk_level, &currency, &reference)?;

        let reference = reference.trim().to_uppercase();

        Ok(Self {
            id: asset_id,
            name,
            class,
            category,
            currency,
            risk_level,
            reference,
            is_archived,
        })
    }

    fn validate(name: &str, risk_level: u8, currency: &str, reference: &str) -> Result<()> {
        if name.trim().is_empty() {
            anyhow::bail!("Asset name cannot be empty");
        }
        if reference.trim().is_empty() {
            anyhow::bail!("Asset reference cannot be empty");
        }
        if !(1..=5).contains(&risk_level) {
            anyhow::bail!(
                "Risk level must be between 1 and 5 (received: {})",
                risk_level
            );
        }
        if Currency::from_str(currency).is_err() {
            anyhow::bail!("Invalid currency code: {}", currency);
        }
        Ok(())
    }

    /// Restores an Asset from storage (no validation — already validated at write time).
    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        asset_id: String,
        name: String,
        class: AssetClass,
        category: AssetCategory,
        currency: String,
        risk_level: u8,
        reference: String,
        is_archived: bool,
    ) -> Self {
        Self {
            id: asset_id,
            name,
            class,
            category,
            currency,
            risk_level,
            reference,
            is_archived,
        }
    }
}

/// Interface for asset persistence.
#[async_trait]
pub trait AssetRepository: Send + Sync {
    /// Fetches all active (non-archived) assets.
    async fn get_all(&self) -> Result<Vec<Asset>>;
    /// Fetches all assets including archived ones.
    async fn get_all_including_archived(&self) -> Result<Vec<Asset>>;
    /// Fetches an asset by its ID.
    async fn get_by_id(&self, id: &str) -> Result<Option<Asset>>;
    /// Persists a new asset.
    async fn create(&self, asset: Asset) -> Result<Asset>;
    /// Updates an existing asset.
    async fn update(&self, asset: Asset) -> Result<Asset>;
    /// Soft-deletes an asset.
    async fn delete(&self, id: &str) -> Result<()>;
    /// Archives an asset (reversible).
    async fn archive(&self, id: &str) -> Result<()>;
    /// Unarchives an asset.
    async fn unarchive(&self, id: &str) -> Result<()>;
}
