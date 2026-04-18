use crate::context::account::{AccountRepository, Holding, HoldingRepository};
use crate::context::asset::AssetRepository;
use crate::context::transaction::{Transaction, TransactionService, TransactionType};
use crate::core::logger::BACKEND;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tracing::info;

/// DTO for creating or updating a transaction.
/// `total_amount` is intentionally absent — the backend computes it from the other
/// fields (TRX-026) so the frontend never sends a derived value over the wire.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateTransactionDTO {
    /// Account where the transaction occurs.
    pub account_id: String,
    /// Financial asset involved.
    pub asset_id: String,
    /// Transaction date (YYYY-MM-DD).
    pub date: String,
    /// Quantity in micro-units.
    pub quantity: i64,
    /// Unit price in asset currency (micro-units).
    pub unit_price: i64,
    /// Exchange rate asset→account currency (micro-units).
    pub exchange_rate: i64,
    /// Fees in account currency (micro-units).
    pub fees: i64,
    /// Optional user note.
    pub note: Option<String>,
}

/// Orchestrates transaction creation, update, and deletion across
/// `transaction/`, `account/`, and `asset/` bounded contexts (B6, B10).
///
/// Atomicity (TRX-027): all DB writes within each operation are wrapped in a
/// single sqlx transaction via the pool. Event publication is delegated to
/// `TransactionService.notify_transaction_updated()` after commit (B8).
pub struct RecordTransactionUseCase {
    pool: Arc<Pool<Sqlite>>,
    transaction_service: Arc<TransactionService>,
    holding_repo: Arc<dyn HoldingRepository>,
    asset_repo: Arc<dyn AssetRepository>,
    account_repo: Arc<dyn AccountRepository>,
}

impl RecordTransactionUseCase {
    /// Creates a new RecordTransactionUseCase.
    pub fn new(
        pool: Arc<Pool<Sqlite>>,
        transaction_service: Arc<TransactionService>,
        holding_repo: Arc<dyn HoldingRepository>,
        asset_repo: Arc<dyn AssetRepository>,
        account_repo: Arc<dyn AccountRepository>,
    ) -> Self {
        Self {
            pool,
            transaction_service,
            holding_repo,
            asset_repo,
            account_repo,
        }
    }

    /// Creates a new purchase transaction and updates the Holding atomically (TRX-027).
    pub async fn create_transaction(&self, dto: CreateTransactionDTO) -> Result<Transaction> {
        // TRX-020 — validate asset and account exist
        let asset = self
            .asset_repo
            .get_by_id(&dto.asset_id)
            .await?
            .with_context(|| format!("Asset {} not found", dto.asset_id))?;

        self.account_repo
            .get_by_id(&dto.account_id)
            .await?
            .with_context(|| format!("Account {} not found", dto.account_id))?;

        // TRX-028 — check archived status
        let needs_unarchive = asset.is_archived;

        // Compute total_amount server-side (TRX-026) — not trusted from the client.
        let total_amount =
            Self::compute_total(dto.quantity, dto.unit_price, dto.exchange_rate, dto.fees);

        // Build + validate the transaction entity (TRX-020)
        let tx = Transaction::new(
            dto.account_id.clone(),
            dto.asset_id.clone(),
            TransactionType::Purchase,
            dto.date,
            dto.quantity,
            dto.unit_price,
            dto.exchange_rate,
            dto.fees,
            total_amount,
            dto.note,
        )?;

        // Compute new holding state from existing transactions + this new one
        let existing = self
            .transaction_service
            .get_by_account_asset(&dto.account_id, &dto.asset_id)
            .await?;
        let holding = self
            .compute_holding_after_insert(&dto.account_id, &dto.asset_id, &existing, &tx)
            .await?;

        // Atomic DB writes (TRX-027)
        let tx_type_str = tx.transaction_type.to_string();
        let mut db_tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin DB transaction")?;

        if needs_unarchive {
            info!(target: BACKEND, asset_id = %dto.asset_id, "auto-unarchiving asset (TRX-028)");
            sqlx::query!(
                r#"UPDATE assets SET is_archived = FALSE WHERE id = ?"#,
                dto.asset_id
            )
            .execute(&mut *db_tx)
            .await
            .context("Failed to unarchive asset")?;
        }

        sqlx::query!(
            r#"INSERT INTO transactions (id, account_id, asset_id, transaction_type, date, quantity, unit_price, exchange_rate, fees, total_amount, note)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            tx.id,
            tx.account_id,
            tx.asset_id,
            tx_type_str,
            tx.date,
            tx.quantity,
            tx.unit_price,
            tx.exchange_rate,
            tx.fees,
            tx.total_amount,
            tx.note
        )
        .execute(&mut *db_tx)
        .await
        .context("Failed to insert transaction")?;

        self.upsert_holding_in_tx(&mut db_tx, &holding).await?;

        db_tx
            .commit()
            .await
            .context("Failed to commit DB transaction")?;

        // B8 — service publishes TransactionUpdated after commit
        self.transaction_service.notify_transaction_updated();

        Ok(tx)
    }

    /// Updates an existing transaction and recalculates affected Holdings atomically (TRX-031).
    pub async fn update_transaction(
        &self,
        id: String,
        dto: CreateTransactionDTO,
    ) -> Result<Transaction> {
        // Fetch existing to capture old (account_id, asset_id) (TRX-032)
        let existing_tx = self
            .transaction_service
            .get_by_id(&id)
            .await?
            .with_context(|| format!("Transaction {} not found", id))?;

        let old_account_id = existing_tx.account_id.clone();
        let old_asset_id = existing_tx.asset_id.clone();

        // TRX-033 — validate new asset and account exist
        let asset = self
            .asset_repo
            .get_by_id(&dto.asset_id)
            .await?
            .with_context(|| format!("Asset {} not found", dto.asset_id))?;

        self.account_repo
            .get_by_id(&dto.account_id)
            .await?
            .with_context(|| format!("Account {} not found", dto.account_id))?;

        let needs_unarchive = asset.is_archived;

        // Compute total_amount server-side (TRX-026)
        let total_amount =
            Self::compute_total(dto.quantity, dto.unit_price, dto.exchange_rate, dto.fees);

        // Build updated transaction entity (TRX-033)
        let updated_tx = Transaction::with_id(
            id,
            dto.account_id.clone(),
            dto.asset_id.clone(),
            TransactionType::Purchase,
            dto.date,
            dto.quantity,
            dto.unit_price,
            dto.exchange_rate,
            dto.fees,
            total_amount,
            dto.note,
        )?;

        // Compute new holding for the updated (account, asset) pair (TRX-031, TRX-036)
        let all_for_new_pair = self
            .transaction_service
            .get_by_account_asset(&dto.account_id, &dto.asset_id)
            .await?;
        // Replace the old version of this transaction with the updated one for recalculation
        let new_pair_txs: Vec<&Transaction> = all_for_new_pair
            .iter()
            .filter(|t| t.id != updated_tx.id)
            .chain(std::iter::once(&updated_tx))
            .collect();
        let new_holding = Self::compute_vwap_holding(
            &dto.account_id,
            &dto.asset_id,
            &new_pair_txs,
            self.holding_repo.as_ref(),
        )
        .await?;

        // Compute holding for old pair if it changed
        let pair_changed = old_account_id != dto.account_id || old_asset_id != dto.asset_id;
        let old_holding_opt = if pair_changed {
            let all_for_old_pair = self
                .transaction_service
                .get_by_account_asset(&old_account_id, &old_asset_id)
                .await?;
            let old_pair_txs: Vec<&Transaction> = all_for_old_pair
                .iter()
                .filter(|t| t.id != updated_tx.id)
                .collect();
            Some((
                old_account_id.clone(),
                old_asset_id.clone(),
                old_pair_txs.len(),
                Self::compute_vwap_holding(
                    &old_account_id,
                    &old_asset_id,
                    &old_pair_txs,
                    self.holding_repo.as_ref(),
                )
                .await?,
            ))
        } else {
            None
        };

        let tx_type_str = updated_tx.transaction_type.to_string();
        let mut db_tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin DB transaction")?;

        if needs_unarchive {
            sqlx::query!(
                r#"UPDATE assets SET is_archived = FALSE WHERE id = ?"#,
                dto.asset_id
            )
            .execute(&mut *db_tx)
            .await
            .context("Failed to unarchive asset")?;
        }

        sqlx::query!(
            r#"UPDATE transactions SET account_id = ?, asset_id = ?, transaction_type = ?, date = ?, quantity = ?, unit_price = ?, exchange_rate = ?, fees = ?, total_amount = ?, note = ? WHERE id = ?"#,
            updated_tx.account_id,
            updated_tx.asset_id,
            tx_type_str,
            updated_tx.date,
            updated_tx.quantity,
            updated_tx.unit_price,
            updated_tx.exchange_rate,
            updated_tx.fees,
            updated_tx.total_amount,
            updated_tx.note,
            updated_tx.id
        )
        .execute(&mut *db_tx)
        .await
        .context("Failed to update transaction")?;

        self.upsert_holding_in_tx(&mut db_tx, &new_holding).await?;

        if let Some((old_acc, old_ast, remaining_count, old_holding)) = old_holding_opt {
            if remaining_count == 0 {
                sqlx::query!(
                    r#"DELETE FROM holdings WHERE account_id = ? AND asset_id = ?"#,
                    old_acc,
                    old_ast
                )
                .execute(&mut *db_tx)
                .await
                .context("Failed to remove orphan holding")?;
            } else {
                self.upsert_holding_in_tx(&mut db_tx, &old_holding).await?;
            }
        }

        db_tx
            .commit()
            .await
            .context("Failed to commit DB transaction")?;

        self.transaction_service.notify_transaction_updated();
        Ok(updated_tx)
    }

    /// Deletes a transaction and recalculates (or removes) the associated Holding (TRX-034).
    pub async fn delete_transaction(&self, id: &str) -> Result<()> {
        let existing_tx = self
            .transaction_service
            .get_by_id(id)
            .await?
            .with_context(|| format!("Transaction {} not found", id))?;

        let account_id = existing_tx.account_id.clone();
        let asset_id = existing_tx.asset_id.clone();

        // Compute remaining transactions after deletion
        let all = self
            .transaction_service
            .get_by_account_asset(&account_id, &asset_id)
            .await?;
        let remaining: Vec<&Transaction> = all.iter().filter(|t| t.id != id).collect();

        let mut db_tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin DB transaction")?;

        sqlx::query!(r#"DELETE FROM transactions WHERE id = ?"#, id)
            .execute(&mut *db_tx)
            .await
            .context("Failed to delete transaction")?;

        if remaining.is_empty() {
            // TRX-034 — no transactions remain: remove the holding
            sqlx::query!(
                r#"DELETE FROM holdings WHERE account_id = ? AND asset_id = ?"#,
                account_id,
                asset_id
            )
            .execute(&mut *db_tx)
            .await
            .context("Failed to delete orphan holding")?;
        } else {
            let updated_holding = Self::compute_vwap_holding(
                &account_id,
                &asset_id,
                &remaining,
                self.holding_repo.as_ref(),
            )
            .await?;
            self.upsert_holding_in_tx(&mut db_tx, &updated_holding)
                .await?;
        }

        db_tx
            .commit()
            .await
            .context("Failed to commit DB transaction")?;

        self.transaction_service.notify_transaction_updated();
        Ok(())
    }

    /// Returns all transactions for an account/asset pair.
    pub async fn get_transactions(
        &self,
        account_id: &str,
        asset_id: &str,
    ) -> Result<Vec<Transaction>> {
        self.transaction_service
            .get_by_account_asset(account_id, asset_id)
            .await
    }

    // --- Private helpers ---

    /// Computes total_amount from the other transaction fields (TRX-026).
    /// Formula: floor(floor(qty × price / MICRO) × rate / MICRO) + fees
    /// Uses i128 intermediates to prevent overflow.
    fn compute_total(quantity: i64, unit_price: i64, exchange_rate: i64, fees: i64) -> i64 {
        const MICRO: i128 = 1_000_000;
        let qty = quantity as i128;
        let price = unit_price as i128;
        let rate = exchange_rate as i128;
        ((qty * price / MICRO) * rate / MICRO) as i64 + fees
    }

    /// Computes the new holding after inserting `new_tx` into the set of existing transactions.
    async fn compute_holding_after_insert(
        &self,
        account_id: &str,
        asset_id: &str,
        existing: &[Transaction],
        new_tx: &Transaction,
    ) -> Result<Holding> {
        let all: Vec<&Transaction> = existing.iter().chain(std::iter::once(new_tx)).collect();
        Self::compute_vwap_holding(account_id, asset_id, &all, self.holding_repo.as_ref()).await
    }

    /// Computes the VWAP-based Holding from a slice of transactions (TRX-030, TRX-036).
    /// Uses i128 intermediates to avoid overflow (plan note).
    async fn compute_vwap_holding(
        account_id: &str,
        asset_id: &str,
        transactions: &[&Transaction],
        holding_repo: &dyn HoldingRepository,
    ) -> Result<Holding> {
        const MICRO: i128 = 1_000_000;

        let mut total_quantity: i128 = 0;
        let mut vwap_numerator: i128 = 0;

        for t in transactions {
            if t.transaction_type == TransactionType::Purchase {
                let qty = t.quantity as i128;
                total_quantity += qty;
                // Use stored total_amount (TRX-026) so VWAP and displayed cost share the same value (TRX-030).
                // total_amount is MICRO; scale to MICRO² so dividing by total_quantity (MICRO) yields MICRO.
                vwap_numerator += t.total_amount as i128 * MICRO;
            }
        }

        let average_price: i64 = if total_quantity > 0 {
            (vwap_numerator / total_quantity) as i64
        } else {
            0
        };
        let quantity = total_quantity as i64;

        // Preserve existing holding ID or generate a new one
        let holding = match holding_repo
            .get_by_account_asset(account_id, asset_id)
            .await?
        {
            Some(existing) => Holding::with_id(
                existing.id,
                account_id.to_string(),
                asset_id.to_string(),
                quantity,
                average_price,
            )?,
            None => Holding::new(
                account_id.to_string(),
                asset_id.to_string(),
                quantity,
                average_price,
            )?,
        };

        Ok(holding)
    }

    /// Executes an upsert holding query within an active sqlx transaction.
    async fn upsert_holding_in_tx(
        &self,
        db_tx: &mut sqlx::Transaction<'_, Sqlite>,
        holding: &Holding,
    ) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO holdings (id, account_id, asset_id, quantity, average_price)
               VALUES (?, ?, ?, ?, ?)
               ON CONFLICT(account_id, asset_id) DO UPDATE SET
                   quantity = excluded.quantity,
                   average_price = excluded.average_price"#,
            holding.id,
            holding.account_id,
            holding.asset_id,
            holding.quantity,
            holding.average_price
        )
        .execute(&mut **db_tx)
        .await
        .context("Failed to upsert holding")?;

        Ok(())
    }
}
