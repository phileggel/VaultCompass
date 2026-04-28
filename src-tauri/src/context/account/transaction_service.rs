use crate::context::account::domain::{Transaction, TransactionRepository};
use crate::core::{logger::BACKEND, Event, SideEffectEventBus};
use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::info;

/// Orchestrates business logic for transactions (B4 — publishes TransactionUpdated, TRX-037).
pub struct TransactionService {
    repo: Box<dyn TransactionRepository>,
    event_bus: Option<Arc<SideEffectEventBus>>,
}

impl TransactionService {
    /// Creates a new TransactionService.
    pub fn new(repo: Box<dyn TransactionRepository>) -> Self {
        Self {
            repo,
            event_bus: None,
        }
    }

    /// Attaches an event bus for side-effect notifications.
    pub fn with_event_bus(mut self, bus: Arc<SideEffectEventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Retrieves a transaction by ID.
    pub async fn get_by_id(&self, id: &str) -> Result<Option<Transaction>> {
        self.repo.get_by_id(id).await
    }

    /// Returns distinct asset IDs that have transactions for the given account (TXL-013).
    pub async fn get_asset_ids_for_account(&self, account_id: &str) -> Result<Vec<String>> {
        self.repo.get_asset_ids_for_account(account_id).await
    }

    /// Returns sum of realized_pnl grouped by asset_id for Sell transactions in the account (SEL-038).
    pub async fn get_realized_pnl_by_account(
        &self,
        account_id: &str,
    ) -> Result<Vec<(String, i64)>> {
        self.repo.get_realized_pnl_by_account(account_id).await
    }

    /// Retrieves all transactions for an account/asset pair in chronological order (TRX-036).
    pub async fn get_by_account_asset(
        &self,
        account_id: &str,
        asset_id: &str,
    ) -> Result<Vec<Transaction>> {
        self.repo.get_by_account_asset(account_id, asset_id).await
    }

    /// Persists a new transaction and publishes TransactionUpdated (TRX-037).
    pub async fn create(&self, tx: Transaction) -> Result<Transaction> {
        info!(target: BACKEND, transaction_id = %tx.id, account_id = %tx.account_id, asset_id = %tx.asset_id, "creating transaction");
        let created = self
            .repo
            .create(tx)
            .await
            .context("TransactionService::create failed")?;
        self.publish();
        Ok(created)
    }

    /// Updates an existing transaction and publishes TransactionUpdated (TRX-037).
    pub async fn update(&self, tx: Transaction) -> Result<Transaction> {
        info!(target: BACKEND, transaction_id = %tx.id, "updating transaction");
        let updated = self
            .repo
            .update(tx)
            .await
            .context("TransactionService::update failed")?;
        self.publish();
        Ok(updated)
    }

    /// Deletes a transaction and publishes TransactionUpdated (TRX-037).
    pub async fn delete(&self, id: &str) -> Result<()> {
        info!(target: BACKEND, transaction_id = %id, "deleting transaction");
        self.repo
            .delete(id)
            .await
            .context("TransactionService::delete failed")?;
        self.publish();
        Ok(())
    }

    /// Publishes TransactionUpdated without performing any write.
    /// Called by the use case after an atomic DB commit (TRX-027 + B8).
    pub fn notify_transaction_updated(&self) {
        self.publish();
    }

    fn publish(&self) {
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::TransactionUpdated);
        }
    }
}
