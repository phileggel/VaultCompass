use crate::context::account::{AccountRepository, Holding, HoldingRepository};
use crate::context::asset::AssetRepository;
use crate::context::transaction::{Transaction, TransactionService, TransactionType};
use crate::core::logger::BACKEND;
use crate::use_cases::record_transaction::error::RecordTransactionError;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
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
    /// Transaction type: "Purchase" or "Sell".
    pub transaction_type: String,
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

    /// Creates a new transaction (Purchase or Sell) and updates the Holding atomically (TRX-027, SEL-028).
    pub async fn create_transaction(&self, dto: CreateTransactionDTO) -> Result<Transaction> {
        let tx_type = dto
            .transaction_type
            .parse::<TransactionType>()
            .map_err(|_| RecordTransactionError::InvalidType)?;

        // TRX-020 / SEL-020 — validate asset and account exist
        let asset = self
            .asset_repo
            .get_by_id(&dto.asset_id)
            .await?
            .ok_or(RecordTransactionError::AssetNotFound)?;

        self.account_repo
            .get_by_id(&dto.account_id)
            .await?
            .ok_or(RecordTransactionError::AccountNotFound)?;

        match tx_type {
            TransactionType::Purchase => self.create_purchase(dto, asset.is_archived).await,
            TransactionType::Sell => {
                // SEL-037 — reject sell on archived asset
                if asset.is_archived {
                    return Err(RecordTransactionError::ArchivedAssetSell.into());
                }
                self.create_sell(dto).await
            }
        }
    }

    async fn create_purchase(
        &self,
        dto: CreateTransactionDTO,
        needs_unarchive: bool,
    ) -> Result<Transaction> {
        let total_amount =
            Self::compute_total(dto.quantity, dto.unit_price, dto.exchange_rate, dto.fees);

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
            None,
        )?;

        let existing = self
            .transaction_service
            .get_by_account_asset(&dto.account_id, &dto.asset_id)
            .await?;

        let all: Vec<&Transaction> = existing.iter().chain(std::iter::once(&tx)).collect();
        let (holding, _) = Self::recalculate_holding(
            &dto.account_id,
            &dto.asset_id,
            &all,
            self.holding_repo.as_ref(),
        )
        .await?;

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
            r#"INSERT INTO transactions (id, account_id, asset_id, transaction_type, date, quantity, unit_price, exchange_rate, fees, total_amount, note, realized_pnl, created_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            tx.id, tx.account_id, tx.asset_id, tx_type_str,
            tx.date, tx.quantity, tx.unit_price, tx.exchange_rate,
            tx.fees, tx.total_amount, tx.note, tx.realized_pnl, tx.created_at
        )
        .execute(&mut *db_tx)
        .await
        .context("Failed to insert transaction")?;

        self.upsert_holding_in_tx(&mut db_tx, &holding).await?;

        db_tx
            .commit()
            .await
            .context("Failed to commit DB transaction")?;
        self.transaction_service.notify_transaction_updated();
        Ok(tx)
    }

    async fn create_sell(&self, dto: CreateTransactionDTO) -> Result<Transaction> {
        let existing = self
            .transaction_service
            .get_by_account_asset(&dto.account_id, &dto.asset_id)
            .await?;

        // SEL-012 — closed position guard.
        // Note: recalculate_holding runs before pool.begin(). SQLite WAL serialises all writers
        // so there is no concurrent-write risk in this single-user desktop app.
        let existing_refs: Vec<&Transaction> = existing.iter().collect();
        let (holding_before, _) = Self::recalculate_holding(
            &dto.account_id,
            &dto.asset_id,
            &existing_refs,
            self.holding_repo.as_ref(),
        )
        .await?;
        if holding_before.quantity == 0 {
            return Err(RecordTransactionError::ClosedPosition.into());
        }

        // SEL-021 — oversell guard
        if dto.quantity > holding_before.quantity {
            return Err(RecordTransactionError::Oversell {
                available: holding_before.quantity,
                requested: dto.quantity,
            }
            .into());
        }

        // SEL-023 — sell total formula: floor(floor(qty * price / MICRO) * rate / MICRO) - fees
        let total_amount =
            Self::compute_sell_total(dto.quantity, dto.unit_price, dto.exchange_rate, dto.fees);

        let tx = Transaction::new(
            dto.account_id.clone(),
            dto.asset_id.clone(),
            TransactionType::Sell,
            dto.date,
            dto.quantity,
            dto.unit_price,
            dto.exchange_rate,
            dto.fees,
            total_amount,
            dto.note,
            None, // realized_pnl computed in recalculate_holding
        )?;

        let all: Vec<&Transaction> = existing.iter().chain(std::iter::once(&tx)).collect();
        let (holding, pnl_map) = Self::recalculate_holding(
            &dto.account_id,
            &dto.asset_id,
            &all,
            self.holding_repo.as_ref(),
        )
        .await?;

        // Attach computed realized_pnl to the new sell transaction
        let realized_pnl = pnl_map.get(&tx.id).copied();
        let tx = Transaction::restore(
            tx.id,
            tx.account_id,
            tx.asset_id,
            tx.transaction_type,
            tx.date,
            tx.quantity,
            tx.unit_price,
            tx.exchange_rate,
            tx.fees,
            tx.total_amount,
            tx.note,
            realized_pnl,
            tx.created_at,
        );

        let tx_type_str = tx.transaction_type.to_string();
        let mut db_tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin DB transaction")?;

        sqlx::query!(
            r#"INSERT INTO transactions (id, account_id, asset_id, transaction_type, date, quantity, unit_price, exchange_rate, fees, total_amount, note, realized_pnl, created_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            tx.id, tx.account_id, tx.asset_id, tx_type_str,
            tx.date, tx.quantity, tx.unit_price, tx.exchange_rate,
            tx.fees, tx.total_amount, tx.note, tx.realized_pnl, tx.created_at
        )
        .execute(&mut *db_tx)
        .await
        .context("Failed to insert sell transaction")?;

        self.upsert_holding_in_tx(&mut db_tx, &holding).await?;

        db_tx
            .commit()
            .await
            .context("Failed to commit DB transaction")?;
        self.transaction_service.notify_transaction_updated();
        Ok(tx)
    }

    /// Updates an existing transaction and recalculates affected Holdings atomically (TRX-031, SEL-031).
    pub async fn update_transaction(
        &self,
        id: String,
        dto: CreateTransactionDTO,
    ) -> Result<Transaction> {
        let existing_tx = self
            .transaction_service
            .get_by_id(&id)
            .await?
            .ok_or(RecordTransactionError::TransactionNotFound)?;

        // SEL-035 — transaction_type immutability
        let existing_type_str = existing_tx.transaction_type.to_string();
        if existing_type_str != dto.transaction_type {
            return Err(RecordTransactionError::TypeImmutable.into());
        }

        let tx_type = existing_tx.transaction_type;
        let old_account_id = existing_tx.account_id.clone();
        let old_asset_id = existing_tx.asset_id.clone();

        // SEL-020 / TRX-033 — validate new asset and account exist
        let asset = self
            .asset_repo
            .get_by_id(&dto.asset_id)
            .await?
            .ok_or(RecordTransactionError::AssetNotFound)?;

        self.account_repo
            .get_by_id(&dto.account_id)
            .await?
            .ok_or(RecordTransactionError::AccountNotFound)?;

        // SEL-037 / TRX-033 — archived asset guard enforced on update
        if asset.is_archived && tx_type == TransactionType::Sell {
            return Err(RecordTransactionError::ArchivedAssetSell.into());
        }
        let needs_unarchive = asset.is_archived && tx_type == TransactionType::Purchase;

        let total_amount = match tx_type {
            TransactionType::Purchase => {
                Self::compute_total(dto.quantity, dto.unit_price, dto.exchange_rate, dto.fees)
            }
            TransactionType::Sell => {
                Self::compute_sell_total(dto.quantity, dto.unit_price, dto.exchange_rate, dto.fees)
            }
        };

        let updated_tx = Transaction::with_id(
            id,
            dto.account_id.clone(),
            dto.asset_id.clone(),
            tx_type,
            dto.date,
            dto.quantity,
            dto.unit_price,
            dto.exchange_rate,
            dto.fees,
            total_amount,
            dto.note,
            existing_tx.realized_pnl,
            existing_tx.created_at.clone(),
        )?;

        // SEL-031 / TRX-031 — full recalculation for new pair
        let all_for_new_pair = self
            .transaction_service
            .get_by_account_asset(&dto.account_id, &dto.asset_id)
            .await?;
        let new_pair_txs: Vec<&Transaction> = all_for_new_pair
            .iter()
            .filter(|t| t.id != updated_tx.id)
            .chain(std::iter::once(&updated_tx))
            .collect();

        let (new_holding, new_pnl_map) = Self::recalculate_holding(
            &dto.account_id,
            &dto.asset_id,
            &new_pair_txs,
            self.holding_repo.as_ref(),
        )
        .await?;

        // Reattach computed realized_pnl to the updated tx if it's a Sell
        let updated_tx = if tx_type == TransactionType::Sell {
            let realized_pnl = new_pnl_map.get(&updated_tx.id).copied();
            Transaction::restore(
                updated_tx.id,
                updated_tx.account_id,
                updated_tx.asset_id,
                updated_tx.transaction_type,
                updated_tx.date,
                updated_tx.quantity,
                updated_tx.unit_price,
                updated_tx.exchange_rate,
                updated_tx.fees,
                updated_tx.total_amount,
                updated_tx.note,
                realized_pnl,
                updated_tx.created_at,
            )
        } else {
            updated_tx
        };

        // Compute holding for old pair if (account, asset) changed
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
                Self::recalculate_holding(
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
            r#"UPDATE transactions SET account_id = ?, asset_id = ?, transaction_type = ?, date = ?, quantity = ?, unit_price = ?, exchange_rate = ?, fees = ?, total_amount = ?, note = ?, realized_pnl = ? WHERE id = ?"#,
            updated_tx.account_id, updated_tx.asset_id, tx_type_str,
            updated_tx.date, updated_tx.quantity, updated_tx.unit_price,
            updated_tx.exchange_rate, updated_tx.fees, updated_tx.total_amount,
            updated_tx.note, updated_tx.realized_pnl, updated_tx.id
        )
        .execute(&mut *db_tx)
        .await
        .context("Failed to update transaction")?;

        // SEL-031 — update realized_pnl for all sells in the recalculated pair
        for (tx_id, pnl) in &new_pnl_map {
            if tx_id != &updated_tx.id {
                sqlx::query!(
                    r#"UPDATE transactions SET realized_pnl = ? WHERE id = ?"#,
                    pnl,
                    tx_id
                )
                .execute(&mut *db_tx)
                .await
                .with_context(|| format!("Failed to update realized_pnl for {}", tx_id))?;
            }
        }

        self.upsert_holding_in_tx(&mut db_tx, &new_holding).await?;

        if let Some((old_acc, old_ast, remaining_count, (old_holding, _))) = old_holding_opt {
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

    /// Deletes a transaction and recalculates (or removes) the associated Holding (TRX-034, SEL-033).
    pub async fn delete_transaction(&self, id: &str) -> Result<()> {
        let existing_tx = self
            .transaction_service
            .get_by_id(id)
            .await?
            .ok_or(RecordTransactionError::TransactionNotFound)?;

        let account_id = existing_tx.account_id.clone();
        let asset_id = existing_tx.asset_id.clone();

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
            sqlx::query!(
                r#"DELETE FROM holdings WHERE account_id = ? AND asset_id = ?"#,
                account_id,
                asset_id
            )
            .execute(&mut *db_tx)
            .await
            .context("Failed to delete orphan holding")?;
        } else {
            // SEL-033 — full recalculation updates realized_pnl on remaining sells
            let (updated_holding, pnl_map) = Self::recalculate_holding(
                &account_id,
                &asset_id,
                &remaining,
                self.holding_repo.as_ref(),
            )
            .await?;

            for (tx_id, pnl) in &pnl_map {
                sqlx::query!(
                    r#"UPDATE transactions SET realized_pnl = ? WHERE id = ?"#,
                    pnl,
                    tx_id
                )
                .execute(&mut *db_tx)
                .await
                .with_context(|| format!("Failed to update realized_pnl for {}", tx_id))?;
            }

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

    /// Computes total_amount for a Purchase (TRX-026).
    /// Formula: floor(floor(qty × price / MICRO) × rate / MICRO) + fees
    fn compute_total(quantity: i64, unit_price: i64, exchange_rate: i64, fees: i64) -> i64 {
        const MICRO: i128 = 1_000_000;
        let qty = quantity as i128;
        let price = unit_price as i128;
        let rate = exchange_rate as i128;
        ((qty * price / MICRO) * rate / MICRO) as i64 + fees
    }

    /// Computes total_amount for a Sell (SEL-023).
    /// Formula: floor(floor(qty × price / MICRO) × rate / MICRO) - fees
    fn compute_sell_total(quantity: i64, unit_price: i64, exchange_rate: i64, fees: i64) -> i64 {
        const MICRO: i128 = 1_000_000;
        let qty = quantity as i128;
        let price = unit_price as i128;
        let rate = exchange_rate as i128;
        ((qty * price / MICRO) * rate / MICRO) as i64 - fees
    }

    /// Computes realized P&L for a sell (SEL-024).
    /// realized_pnl = total_sell_amount - floor(vwap_before_sell × sold_quantity / MICRO)
    fn compute_realized_pnl(
        total_sell_amount: i64,
        vwap_before_sell: i64,
        sold_quantity: i64,
    ) -> i64 {
        const MICRO: i128 = 1_000_000;
        let cost_basis = (vwap_before_sell as i128 * sold_quantity as i128 / MICRO) as i64;
        total_sell_amount - cost_basis
    }

    /// Full chronological recalculation of Holding state and realized P&L for all transactions
    /// in the given slice (TRX-030, SEL-024, SEL-025, SEL-026, SEL-027, SEL-032).
    ///
    /// Returns: `(updated_holding, sell_tx_id -> realized_pnl)`.
    /// Errors if any Sell would exceed the running quantity (SEL-032).
    async fn recalculate_holding(
        account_id: &str,
        asset_id: &str,
        transactions: &[&Transaction],
        holding_repo: &dyn HoldingRepository,
    ) -> Result<(Holding, HashMap<String, i64>)> {
        const MICRO: i128 = 1_000_000;

        let mut total_quantity: i128 = 0;
        let mut vwap_numerator: i128 = 0;
        let mut last_vwap: i64 = 0;
        let mut pnl_map: HashMap<String, i64> = HashMap::new();
        let mut total_realized_pnl: i64 = 0;
        let mut last_sold_date: Option<String> = None;

        for t in transactions {
            match t.transaction_type {
                TransactionType::Purchase => {
                    let qty = t.quantity as i128;
                    total_quantity += qty;
                    // VWAP uses total_amount (TRX-030); scale to MICRO² before dividing by qty.
                    vwap_numerator += t.total_amount as i128 * MICRO;
                }
                TransactionType::Sell => {
                    // SEL-032 — cascading oversell check
                    if t.quantity as i128 > total_quantity {
                        return Err(RecordTransactionError::CascadingOversell.into());
                    }
                    let vwap_before: i64 = if total_quantity > 0 {
                        (vwap_numerator / total_quantity) as i64
                    } else {
                        0
                    };
                    last_vwap = vwap_before;
                    // SEL-024 — realized P&L
                    let pnl = Self::compute_realized_pnl(t.total_amount, vwap_before, t.quantity);
                    pnl_map.insert(t.id.clone(), pnl);
                    total_realized_pnl += pnl;
                    // ACD-043 — track latest sell date (ISO strings sort lexicographically)
                    if last_sold_date.as_deref() < Some(t.date.as_str()) {
                        last_sold_date = Some(t.date.clone());
                    }
                    // SEL-025 — decrease quantity; SEL-027 — VWAP numerator scales with qty
                    let qty = t.quantity as i128;
                    vwap_numerator -= vwap_before as i128 * qty;
                    total_quantity -= qty;
                }
            }
        }

        // SEL-026 / TRX-040 — when qty reaches zero, preserve last known VWAP
        let average_price: i64 = if total_quantity > 0 {
            (vwap_numerator / total_quantity) as i64
        } else {
            last_vwap
        };
        // SEL-026 — retain holding at qty=0
        let quantity = total_quantity as i64;

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
                total_realized_pnl,
                last_sold_date,
            )?,
            None => Holding::new(
                account_id.to_string(),
                asset_id.to_string(),
                quantity,
                average_price,
                total_realized_pnl,
                last_sold_date,
            )?,
        };

        Ok((holding, pnl_map))
    }

    #[cfg(test)]
    pub(crate) fn compute_sell_total_pub(
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        fees: i64,
    ) -> i64 {
        Self::compute_sell_total(quantity, unit_price, exchange_rate, fees)
    }

    #[cfg(test)]
    pub(crate) fn compute_realized_pnl_pub(
        total_sell_amount: i64,
        vwap_before_sell: i64,
        sold_quantity: i64,
    ) -> i64 {
        Self::compute_realized_pnl(total_sell_amount, vwap_before_sell, sold_quantity)
    }

    /// Executes an upsert holding query within an active sqlx transaction.
    async fn upsert_holding_in_tx(
        &self,
        db_tx: &mut sqlx::Transaction<'_, Sqlite>,
        holding: &Holding,
    ) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO holdings (id, account_id, asset_id, quantity, average_price, total_realized_pnl, last_sold_date)
               VALUES (?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(account_id, asset_id) DO UPDATE SET
                   quantity = excluded.quantity,
                   average_price = excluded.average_price,
                   total_realized_pnl = excluded.total_realized_pnl,
                   last_sold_date = excluded.last_sold_date"#,
            holding.id,
            holding.account_id,
            holding.asset_id,
            holding.quantity,
            holding.average_price,
            holding.total_realized_pnl,
            holding.last_sold_date
        )
        .execute(&mut **db_tx)
        .await
        .context("Failed to upsert holding")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::account::{
        AccountService, SqliteAccountRepository, SqliteHoldingRepository, UpdateFrequency,
    };
    use crate::context::asset::{
        AssetClass, CreateAssetDTO, SqliteAssetCategoryRepository, SqliteAssetRepository,
        SYSTEM_CATEGORY_ID,
    };
    use crate::context::transaction::SqliteTransactionRepository;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn make_pool() -> sqlx::Pool<sqlx::Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test pool");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");
        pool
    }

    async fn setup_uc(pool: &sqlx::Pool<sqlx::Sqlite>) -> RecordTransactionUseCase {
        let account_repo = Arc::new(SqliteAccountRepository::new(pool.clone()));
        let holding_repo = Arc::new(SqliteHoldingRepository::new(pool.clone()));
        let asset_repo = Arc::new(SqliteAssetRepository::new(pool.clone()));
        let tx_service = Arc::new(crate::context::transaction::TransactionService::new(
            Box::new(SqliteTransactionRepository::new(pool.clone())),
        ));
        RecordTransactionUseCase::new(
            Arc::new(pool.clone()),
            tx_service,
            holding_repo,
            asset_repo,
            account_repo,
        )
    }

    async fn create_account(pool: &sqlx::Pool<sqlx::Sqlite>) -> String {
        AccountService::new(
            Box::new(SqliteAccountRepository::new(pool.clone())),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
        )
        .create(
            "Test Account".to_string(),
            "EUR".to_string(),
            UpdateFrequency::ManualMonth,
        )
        .await
        .unwrap()
        .id
    }

    async fn create_asset(pool: &sqlx::Pool<sqlx::Sqlite>) -> String {
        crate::context::asset::AssetService::new(
            Box::new(SqliteAssetRepository::new(pool.clone())),
            Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
            Box::new(crate::context::asset::SqliteAssetPriceRepository::new(
                pool.clone(),
            )),
        )
        .create_asset(CreateAssetDTO {
            name: "AAPL".to_string(),
            reference: "AAPL".to_string(),
            class: AssetClass::Stocks,
            currency: "USD".to_string(),
            risk_level: 3,
            category_id: SYSTEM_CATEGORY_ID.to_string(),
        })
        .await
        .unwrap()
        .id
    }

    fn buy_dto(account_id: &str, asset_id: &str, qty: i64) -> CreateTransactionDTO {
        let micro = 1_000_000i64;
        CreateTransactionDTO {
            account_id: account_id.to_string(),
            asset_id: asset_id.to_string(),
            transaction_type: "Purchase".to_string(),
            date: "2024-01-01".to_string(),
            quantity: qty,
            unit_price: 100 * micro,
            exchange_rate: micro,
            fees: 0,
            note: None,
        }
    }

    fn sell_dto(account_id: &str, asset_id: &str, qty: i64) -> CreateTransactionDTO {
        let micro = 1_000_000i64;
        CreateTransactionDTO {
            account_id: account_id.to_string(),
            asset_id: asset_id.to_string(),
            transaction_type: "Sell".to_string(),
            date: "2024-06-01".to_string(),
            quantity: qty,
            unit_price: 120 * micro,
            exchange_rate: micro,
            fees: 0,
            note: None,
        }
    }

    // SEL-012 — selling when holding quantity is 0 is rejected
    #[tokio::test]
    async fn sell_rejected_when_no_holding() {
        let pool = make_pool().await;
        let uc = setup_uc(&pool).await;
        let account_id = create_account(&pool).await;
        let asset_id = create_asset(&pool).await;

        let err = uc
            .create_transaction(sell_dto(&account_id, &asset_id, 1_000_000))
            .await
            .unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<RecordTransactionError>(),
                Some(RecordTransactionError::ClosedPosition)
            ),
            "got: {err}"
        );
    }

    // SEL-026 — when full position is sold, holding is retained at quantity=0 with last VWAP preserved
    #[tokio::test]
    async fn full_sell_retains_holding_at_zero_with_last_vwap() {
        let pool = make_pool().await;
        let uc = setup_uc(&pool).await;
        let account_id = create_account(&pool).await;
        let asset_id = create_asset(&pool).await;
        let micro = 1_000_000i64;

        uc.create_transaction(buy_dto(&account_id, &asset_id, 2 * micro))
            .await
            .unwrap();
        uc.create_transaction(sell_dto(&account_id, &asset_id, 2 * micro))
            .await
            .unwrap();

        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        let holdings = holding_repo.get_by_account(&account_id).await.unwrap();
        let h = holdings.iter().find(|h| h.asset_id == asset_id).unwrap();
        assert_eq!(h.quantity, 0, "holding should be retained at qty=0");
        assert_eq!(h.average_price, 100 * micro, "VWAP should be preserved");
    }

    // SEL-032 — editing a purchase so it creates an oversell on a subsequent sell is rejected
    #[tokio::test]
    async fn edit_purchase_rejected_when_causes_oversell() {
        let pool = make_pool().await;
        let uc = setup_uc(&pool).await;
        let account_id = create_account(&pool).await;
        let asset_id = create_asset(&pool).await;
        let micro = 1_000_000i64;

        let buy = uc
            .create_transaction(buy_dto(&account_id, &asset_id, 3 * micro))
            .await
            .unwrap();
        uc.create_transaction(sell_dto(&account_id, &asset_id, 2 * micro))
            .await
            .unwrap();

        // Edit the buy down to 1 unit — now the sell of 2 would exceed the holding
        let mut reduced_buy = buy_dto(&account_id, &asset_id, micro);
        reduced_buy.transaction_type = "Purchase".to_string();
        let err = uc
            .update_transaction(buy.id, reduced_buy)
            .await
            .unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<RecordTransactionError>(),
                Some(RecordTransactionError::CascadingOversell)
            ),
            "got: {err}"
        );
    }

    // SEL-035 — changing transaction_type on update is rejected
    #[tokio::test]
    async fn update_rejects_transaction_type_change() {
        let pool = make_pool().await;
        let uc = setup_uc(&pool).await;
        let account_id = create_account(&pool).await;
        let asset_id = create_asset(&pool).await;
        let micro = 1_000_000i64;

        let buy = uc
            .create_transaction(buy_dto(&account_id, &asset_id, micro))
            .await
            .unwrap();

        let mut sell_edit = sell_dto(&account_id, &asset_id, micro);
        sell_edit.transaction_type = "Sell".to_string();
        let err = uc.update_transaction(buy.id, sell_edit).await.unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<RecordTransactionError>(),
                Some(RecordTransactionError::TypeImmutable)
            ),
            "got: {err}"
        );
    }

    // SEL-023 — sell total = floor(floor(qty × price / MICRO) × rate / MICRO) - fees
    #[test]
    fn compute_sell_total_subtracts_fees() {
        // 2 units @ 50.00, rate=1.0, fees=5.00 → 95.00
        let result = RecordTransactionUseCase::compute_sell_total_pub(
            2_000_000, 50_000_000, 1_000_000, 5_000_000,
        );
        assert_eq!(result, 95_000_000);
    }

    #[test]
    fn compute_sell_total_applies_exchange_rate() {
        // 1 unit @ 100.00, rate=1.5, fees=0 → 150.00
        let result =
            RecordTransactionUseCase::compute_sell_total_pub(1_000_000, 100_000_000, 1_500_000, 0);
        assert_eq!(result, 150_000_000);
    }

    // SEL-024 — realized_pnl = total_sell - floor(vwap × qty / MICRO)
    #[test]
    fn compute_realized_pnl_profit() {
        // Sell 1 unit for 95.00; cost basis VWAP=80.00 → P&L = +15.00
        let result =
            RecordTransactionUseCase::compute_realized_pnl_pub(95_000_000, 80_000_000, 1_000_000);
        assert_eq!(result, 15_000_000);
    }

    #[test]
    fn compute_realized_pnl_loss() {
        // Sell 1 unit for 60.00; VWAP=80.00 → P&L = -20.00
        let result =
            RecordTransactionUseCase::compute_realized_pnl_pub(60_000_000, 80_000_000, 1_000_000);
        assert_eq!(result, -20_000_000);
    }

    #[test]
    fn compute_realized_pnl_zero() {
        // Sell at exactly VWAP → P&L = 0
        let result =
            RecordTransactionUseCase::compute_realized_pnl_pub(50_000_000, 50_000_000, 1_000_000);
        assert_eq!(result, 0);
    }
}
