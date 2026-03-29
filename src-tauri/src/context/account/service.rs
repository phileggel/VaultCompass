use super::domain::{
    Account, AccountRepository, AssetAccount, AssetAccountRepository, UpdateFrequency,
};
use crate::core::{Event, SideEffectEventBus};
use anyhow::Result;
use std::sync::Arc;

/// Orchestrates business logic for accounts and asset-account mappings.
pub struct AccountService {
    account_repo: Box<dyn AccountRepository>,
    asset_account_repo: Box<dyn AssetAccountRepository>,
    event_bus: Option<Arc<SideEffectEventBus>>,
}

impl AccountService {
    /// Creates a new AccountService.
    pub fn new(
        account_repo: Box<dyn AccountRepository>,
        asset_account_repo: Box<dyn AssetAccountRepository>,
    ) -> Self {
        Self {
            account_repo,
            asset_account_repo,
            event_bus: None,
        }
    }

    /// Attaches an event bus for side-effect notifications.
    pub fn with_event_bus(mut self, bus: Arc<SideEffectEventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Retrieves all non-deleted accounts.
    pub async fn get_all(&self) -> Result<Vec<Account>> {
        self.account_repo.get_all().await
    }

    /// Retrieves an account by ID.
    pub async fn get_by_id(&self, id: &str) -> Result<Option<Account>> {
        self.account_repo.get_by_id(id).await
    }

    /// Creates a new account.
    pub async fn create(&self, name: String, update_frequency: UpdateFrequency) -> Result<Account> {
        let account = Account::new(name, update_frequency)?;
        let created = self.account_repo.create(account).await?;

        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AccountUpdated);
        }

        Ok(created)
    }

    /// Updates an existing account.
    pub async fn update(
        &self,
        id: String,
        name: String,
        update_frequency: UpdateFrequency,
    ) -> Result<Account> {
        let account = Account::from_storage(id, name, update_frequency);
        let updated = self.account_repo.update(account).await?;

        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AccountUpdated);
        }

        Ok(updated)
    }

    /// Soft-deletes an account.
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.account_repo.delete(id).await?;

        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AccountUpdated);
        }

        Ok(())
    }

    // --- AssetAccount Methods ---

    /// Gets all asset holdings for an account.
    pub async fn get_holdings(&self, account_id: &str) -> Result<Vec<AssetAccount>> {
        self.asset_account_repo.get_by_account(account_id).await
    }

    /// Updates or creates an asset holding in an account.
    pub async fn upsert_holding(
        &self,
        account_id: String,
        asset_id: String,
        average_price: f64,
        quantity: f64,
    ) -> Result<AssetAccount> {
        let aa = AssetAccount::new(account_id, asset_id, average_price, quantity)?;
        let upserted = self.asset_account_repo.upsert(aa).await?;

        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AccountUpdated);
        }

        Ok(upserted)
    }

    /// Removes an asset holding from an account.
    pub async fn remove_holding(&self, account_id: &str, asset_id: &str) -> Result<()> {
        self.asset_account_repo.remove(account_id, asset_id).await?;

        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AccountUpdated);
        }

        Ok(())
    }
}
