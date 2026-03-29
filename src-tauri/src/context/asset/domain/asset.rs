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
}

impl Asset {
    /// Creates a new Asset.
    pub fn new(
        name: String,
        class: AssetClass,
        category: AssetCategory,
        currency: String,
        risk_level: u8,
        reference: Option<String>,
    ) -> Result<Self> {
        Self::validate(&name, risk_level, &currency)?;

        let reference = match reference {
            Some(r) if !r.trim().is_empty() => r.trim().to_uppercase(),
            _ => Self::generate_internal_reference(&class),
        };

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            name,
            class,
            category,
            currency,
            risk_level,
            reference,
        })
    }

    /// Updates an existing Asset.
    pub fn update_from(
        asset_id: String,
        name: String,
        class: AssetClass,
        category: AssetCategory,
        currency: String,
        risk_level: u8,
        reference: Option<String>,
    ) -> Result<Self> {
        Self::validate(&name, risk_level, &currency)?;

        let reference = match reference {
            Some(r) if !r.trim().is_empty() => r.trim().to_uppercase(),
            _ => Self::generate_internal_reference(&class),
        };

        Ok(Self {
            id: asset_id,
            name,
            class,
            category,
            currency,
            risk_level,
            reference,
        })
    }

    fn validate(name: &str, risk_level: u8, currency: &str) -> Result<()> {
        if name.trim().is_empty() {
            anyhow::bail!("Asset name cannot be empty");
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

    fn generate_internal_reference(class: &AssetClass) -> String {
        let reference: String = class
            .to_string()
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect();

        let short_id = &uuid::Uuid::new_v4().to_string()[..4];
        format!("INT-{}-{}", reference.trim_matches('-'), short_id).to_uppercase()
    }

    /// Creates a new Asset from storage.
    pub fn from_storage(
        asset_id: String,
        name: String,
        class: AssetClass,
        category: AssetCategory,
        currency: String,
        risk_level: u8,
        reference: String,
    ) -> Self {
        Self {
            id: asset_id,
            name,
            class,
            category,
            currency,
            risk_level,
            reference,
        }
    }
}

/// Interface for asset persistence.
#[async_trait]
pub trait AssetRepository: Send + Sync {
    /// Fetches all active assets.
    async fn get_all(&self) -> Result<Vec<Asset>>;
    /// Fetches an asset by its ID.
    async fn get_by_id(&self, id: &str) -> Result<Option<Asset>>;
    /// Persists a new asset.
    async fn create(&self, asset: Asset) -> Result<Asset>;
    /// Updates an existing asset.
    async fn update(&self, asset: Asset) -> Result<Asset>;
    /// Soft-deletes an asset.
    async fn delete(&self, id: &str) -> Result<()>;
}
