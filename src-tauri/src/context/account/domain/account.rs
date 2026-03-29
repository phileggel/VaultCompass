use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

/// Defines how often an account's data should be updated.
#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Type,
    PartialEq,
    Eq,
    Default,
    strum_macros::Display,
    strum_macros::EnumString,
)]
pub enum UpdateFrequency {
    /// Automatic updates (e.g. via API)
    Automatic,
    /// Manual update daily
    ManualDay,
    /// Manual update weekly
    ManualWeek,
    /// Manual update monthly
    #[default]
    ManualMonth,
    /// Manual update yearly
    ManualYear,
}

/// Represents a financial account.
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
pub struct Account {
    /// Unique identifier (uuid).
    pub id: String,
    /// User defined name.
    pub name: String,
    /// How often this account is updated.
    pub update_frequency: UpdateFrequency,
}

impl Account {
    /// Creates a new Account.
    pub fn new(name: String, update_frequency: UpdateFrequency) -> Result<Self> {
        if name.trim().is_empty() {
            anyhow::bail!("Account name cannot be empty");
        }
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            name,
            update_frequency,
        })
    }

    /// Reconstructs an Account from storage.
    pub fn from_storage(id: String, name: String, update_frequency: UpdateFrequency) -> Self {
        Self {
            id,
            name,
            update_frequency,
        }
    }
}

/// Interface for account persistence.
#[async_trait]
pub trait AccountRepository: Send + Sync {
    /// Fetches all non-deleted accounts.
    async fn get_all(&self) -> Result<Vec<Account>>;
    /// Fetches an account by its ID.
    async fn get_by_id(&self, id: &str) -> Result<Option<Account>>;
    /// Persists a new account.
    async fn create(&self, account: Account) -> Result<Account>;
    /// Updates an existing account.
    async fn update(&self, account: Account) -> Result<Account>;
    /// Soft-deletes an account.
    async fn delete(&self, id: &str) -> Result<()>;
}
