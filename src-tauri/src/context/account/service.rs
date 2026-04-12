use super::domain::{Account, AccountRepository, UpdateFrequency};
use crate::core::{logger::BACKEND, Event, SideEffectEventBus};
use anyhow::Result;
use std::sync::Arc;
use tracing::info;

/// Orchestrates business logic for accounts.
pub struct AccountService {
    account_repo: Box<dyn AccountRepository>,
    event_bus: Option<Arc<SideEffectEventBus>>,
}

impl AccountService {
    /// Creates a new AccountService.
    pub fn new(account_repo: Box<dyn AccountRepository>) -> Self {
        Self {
            account_repo,
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
        if self
            .account_repo
            .find_by_name(&account.name)
            .await?
            .is_some()
        {
            anyhow::bail!("An account with this name already exists");
        }
        info!(target: BACKEND, account_id = %account.id, name = %account.name, "creating account");
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
        let account = Account::with_id(id, name, update_frequency)?;
        if let Some(existing) = self.account_repo.find_by_name(&account.name).await? {
            if existing.id != account.id {
                anyhow::bail!("An account with this name already exists");
            }
        }
        info!(target: BACKEND, account_id = %account.id, name = %account.name, "updating account");
        let updated = self.account_repo.update(account).await?;

        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AccountUpdated);
        }

        Ok(updated)
    }

    /// Permanently deletes an account and cascades to its holdings (R5).
    /// R6 (transaction cascade) is pending the Transaction feature.
    pub async fn delete(&self, id: &str) -> Result<()> {
        info!(target: BACKEND, account_id = %id, "deleting account");
        self.account_repo.delete(id).await?;

        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AccountUpdated);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Integration tests: use concrete SQLite repos against an in-memory DB to catch
    // real constraint violations (e.g. UNIQUE ON LOWER(name)) that mocks would miss.
    use crate::context::account::SqliteAccountRepository;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_service() -> AccountService {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test pool");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");
        AccountService::new(Box::new(SqliteAccountRepository::new(pool)))
    }

    // R3 — duplicate name (case-insensitive) is rejected at creation
    #[tokio::test]
    async fn create_rejects_duplicate_name_case_insensitive() {
        let svc = setup_service().await;
        svc.create("Alpha".to_string(), UpdateFrequency::ManualMonth)
            .await
            .unwrap();
        let err = svc
            .create("alpha".to_string(), UpdateFrequency::ManualMonth)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("already exists"), "got: {err}");
    }

    // R3 — updating an account to a name used by another account is rejected
    #[tokio::test]
    async fn update_rejects_name_collision_with_other_account() {
        let svc = setup_service().await;
        svc.create("Alpha".to_string(), UpdateFrequency::ManualMonth)
            .await
            .unwrap();
        let beta = svc
            .create("Beta".to_string(), UpdateFrequency::ManualMonth)
            .await
            .unwrap();
        let err = svc
            .update(beta.id, "ALPHA".to_string(), UpdateFrequency::ManualMonth)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("already exists"), "got: {err}");
    }

    // R3 — updating an account with its own name (same case) must succeed
    #[tokio::test]
    async fn update_allows_same_name_on_same_account() {
        let svc = setup_service().await;
        let account = svc
            .create("Alpha".to_string(), UpdateFrequency::ManualMonth)
            .await
            .unwrap();
        let result = svc
            .update(account.id, "Alpha".to_string(), UpdateFrequency::ManualDay)
            .await;
        assert!(result.is_ok());
    }
}
