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
    /// Creates a new Account. Trims the name before validation and storage (R1).
    pub fn new(name: String, update_frequency: UpdateFrequency) -> Result<Self> {
        let name = name.trim().to_string();
        if name.is_empty() {
            anyhow::bail!("Account name cannot be empty");
        }
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            name,
            update_frequency,
        })
    }

    /// Updates an existing Account. Trims and validates identically to new() (R1, R2).
    pub fn with_id(id: String, name: String, update_frequency: UpdateFrequency) -> Result<Self> {
        let name = name.trim().to_string();
        if name.is_empty() {
            anyhow::bail!("Account name cannot be empty");
        }
        Ok(Self {
            id,
            name,
            update_frequency,
        })
    }

    /// Reconstructs an Account from storage without validation.
    pub fn restore(id: String, name: String, update_frequency: UpdateFrequency) -> Self {
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
    /// Fetches all accounts.
    async fn get_all(&self) -> Result<Vec<Account>>;
    /// Fetches an account by its ID.
    async fn get_by_id(&self, id: &str) -> Result<Option<Account>>;
    /// Finds an account by name (case-insensitive, R3).
    async fn find_by_name(&self, name: &str) -> Result<Option<Account>>;
    /// Persists a new account.
    async fn create(&self, account: Account) -> Result<Account>;
    /// Updates an existing account.
    async fn update(&self, account: Account) -> Result<Account>;
    /// Permanently deletes an account and cascades to its holdings (R5).
    async fn delete(&self, id: &str) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // R1 — trim at creation
    #[test]
    fn new_trims_leading_trailing_spaces() {
        let account =
            Account::new("  My Account  ".to_string(), UpdateFrequency::ManualMonth).unwrap();
        assert_eq!(account.name, "My Account");
    }

    // R1, R2 — spaces-only name is invalid after trim
    #[test]
    fn new_rejects_whitespace_only_name() {
        let result = Account::new("   ".to_string(), UpdateFrequency::ManualMonth);
        assert!(result.is_err());
    }

    // R1, R2 — with_id trims and validates
    #[test]
    fn with_id_trims_name() {
        let account = Account::with_id(
            "some-id".to_string(),
            "  Trimmed  ".to_string(),
            UpdateFrequency::ManualDay,
        )
        .unwrap();
        assert_eq!(account.name, "Trimmed");
    }

    // R1, R2 — with_id rejects empty name after trim
    #[test]
    fn with_id_rejects_empty_name_after_trim() {
        let result = Account::with_id(
            "some-id".to_string(),
            "  ".to_string(),
            UpdateFrequency::ManualMonth,
        );
        assert!(result.is_err());
    }
}
