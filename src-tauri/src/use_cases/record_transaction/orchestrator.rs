use crate::context::account::{AccountRepository, Holding, HoldingRepository};
use crate::context::asset::{AssetRepository, AssetService};
use crate::context::transaction::{Transaction, TransactionService, TransactionType};
use crate::core::logger::BACKEND;
use crate::use_cases::record_transaction::error::RecordTransactionError;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

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
    /// MKT-054 — when true and unit_price > 0, the orchestrator also upserts
    /// AssetPrice(asset_id, date, unit_price) inside the same DB tx (MKT-055/056)
    /// and publishes AssetPriceUpdated after commit (MKT-057). Existing same-date
    /// price is silently overwritten (MKT-058). Skipped when unit_price = 0 (MKT-061).
    pub record_price: bool,
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
    asset_service: Arc<AssetService>,
}

impl RecordTransactionUseCase {
    /// Creates a new RecordTransactionUseCase.
    pub fn new(
        pool: Arc<Pool<Sqlite>>,
        transaction_service: Arc<TransactionService>,
        holding_repo: Arc<dyn HoldingRepository>,
        asset_repo: Arc<dyn AssetRepository>,
        account_repo: Arc<dyn AccountRepository>,
        asset_service: Arc<AssetService>,
    ) -> Self {
        Self {
            pool,
            transaction_service,
            holding_repo,
            asset_repo,
            account_repo,
            asset_service,
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

        // MKT-055/061 — auto-record AssetPrice inside the same DB transaction
        let price_written = Self::auto_record_price(&mut db_tx, &tx, dto.record_price).await?;

        db_tx
            .commit()
            .await
            .context("Failed to commit DB transaction")?;
        self.transaction_service.notify_transaction_updated();
        self.maybe_notify_price_updated(price_written);
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

        // MKT-055/061 — auto-record AssetPrice inside the same DB transaction
        let price_written = Self::auto_record_price(&mut db_tx, &tx, dto.record_price).await?;

        db_tx
            .commit()
            .await
            .context("Failed to commit DB transaction")?;
        self.transaction_service.notify_transaction_updated();
        self.maybe_notify_price_updated(price_written);
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

        // MKT-055/059/061 — auto-record AssetPrice at the (possibly new) tx.date and tx.unit_price
        let price_written =
            Self::auto_record_price(&mut db_tx, &updated_tx, dto.record_price).await?;

        db_tx
            .commit()
            .await
            .context("Failed to commit DB transaction")?;
        self.transaction_service.notify_transaction_updated();
        self.maybe_notify_price_updated(price_written);
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

    /// MKT-055/058/061 — upserts AssetPrice(asset_id, date, unit_price) inside the open
    /// DB transaction when the user opted in. Returns true if a price was written so the
    /// caller can publish AssetPriceUpdated after commit (MKT-057). Skipped silently when
    /// `record_price = false` (MKT-054) or `tx.unit_price = 0` (MKT-061, gifted assets).
    /// Conflicts on `(asset_id, date)` are silently overwritten via ON CONFLICT.
    async fn auto_record_price(
        db_tx: &mut sqlx::Transaction<'_, Sqlite>,
        tx: &Transaction,
        record_price: bool,
    ) -> Result<bool> {
        // Domain guarantees unit_price >= 0; the `<= 0` guard collapses to MKT-061 (== 0).
        if !record_price || tx.unit_price <= 0 {
            return Ok(false);
        }
        sqlx::query!(
            r#"INSERT INTO asset_prices (asset_id, date, price) VALUES (?, ?, ?)
               ON CONFLICT(asset_id, date) DO UPDATE SET price = excluded.price"#,
            tx.asset_id,
            tx.date,
            tx.unit_price,
        )
        .execute(&mut **db_tx)
        .await
        .context("Failed to upsert asset price (MKT-055)")?;
        debug!(
            target: BACKEND,
            asset_id = %tx.asset_id,
            date = %tx.date,
            "Auto-recorded asset price from transaction (MKT-055)"
        );
        Ok(true)
    }

    /// MKT-057 — publishes AssetPriceUpdated through the asset bounded context after a
    /// successful commit. No-op when no price was written (record_price = false or skipped
    /// by MKT-061). Centralises the post-commit notification so each call site stays a
    /// single line.
    fn maybe_notify_price_updated(&self, price_written: bool) {
        if price_written {
            self.asset_service.notify_asset_price_updated();
        }
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
        let asset_service = Arc::new(crate::context::asset::AssetService::new(
            Box::new(SqliteAssetRepository::new(pool.clone())),
            Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
            Box::new(crate::context::asset::SqliteAssetPriceRepository::new(
                pool.clone(),
            )),
        ));
        RecordTransactionUseCase::new(
            Arc::new(pool.clone()),
            tx_service,
            holding_repo,
            asset_repo,
            account_repo,
            asset_service,
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
            record_price: false,
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
            record_price: false,
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

    // SEL-037 — creating a sell on an archived asset is rejected
    #[tokio::test]
    async fn create_sell_rejected_when_asset_archived() {
        let pool = make_pool().await;
        let uc = setup_uc(&pool).await;
        let account_id = create_account(&pool).await;
        let asset_id = create_asset(&pool).await;
        let micro = 1_000_000i64;

        uc.create_transaction(buy_dto(&account_id, &asset_id, 2 * micro))
            .await
            .unwrap();

        sqlx::query("UPDATE assets SET is_archived = TRUE WHERE id = ?")
            .bind(&asset_id)
            .execute(&pool)
            .await
            .unwrap();

        let err = uc
            .create_transaction(sell_dto(&account_id, &asset_id, micro))
            .await
            .unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<RecordTransactionError>(),
                Some(RecordTransactionError::ArchivedAssetSell)
            ),
            "got: {err}"
        );
    }

    // SEL-037 / TRX-033 — editing a sell on an archived asset is rejected
    #[tokio::test]
    async fn update_sell_rejected_when_asset_archived() {
        let pool = make_pool().await;
        let uc = setup_uc(&pool).await;
        let account_id = create_account(&pool).await;
        let asset_id = create_asset(&pool).await;
        let micro = 1_000_000i64;

        uc.create_transaction(buy_dto(&account_id, &asset_id, 2 * micro))
            .await
            .unwrap();
        let sell = uc
            .create_transaction(sell_dto(&account_id, &asset_id, micro))
            .await
            .unwrap();

        sqlx::query("UPDATE assets SET is_archived = TRUE WHERE id = ?")
            .bind(&asset_id)
            .execute(&pool)
            .await
            .unwrap();

        let err = uc
            .update_transaction(sell.id, sell_dto(&account_id, &asset_id, micro))
            .await
            .unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<RecordTransactionError>(),
                Some(RecordTransactionError::ArchivedAssetSell)
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

    // --- MKT-055 / MKT-054 / MKT-058 / MKT-059 / MKT-060 / MKT-061 ---

    // MKT-055 — create_purchase with record_price=true writes an AssetPrice row
    #[tokio::test]
    async fn create_purchase_with_record_price_writes_asset_price() {
        let pool = make_pool().await;
        let uc = setup_uc(&pool).await;
        let account_id = create_account(&pool).await;
        let asset_id = create_asset(&pool).await;
        let micro = 1_000_000i64;

        let dto = CreateTransactionDTO {
            record_price: true,
            ..buy_dto(&account_id, &asset_id, micro)
        };
        let expected_price = dto.unit_price;
        let expected_date = dto.date.clone();

        uc.create_transaction(dto).await.unwrap();

        let rows = sqlx::query!(
            "SELECT asset_id, date, price FROM asset_prices WHERE asset_id = ?",
            asset_id
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 1, "expected exactly one AssetPrice row");
        assert_eq!(rows[0].asset_id, asset_id);
        assert_eq!(rows[0].date, expected_date);
        assert_eq!(rows[0].price, expected_price);
    }

    // MKT-055 — create_sell with record_price=true writes an AssetPrice row at the sell date/price
    #[tokio::test]
    async fn create_sell_with_record_price_writes_asset_price() {
        let pool = make_pool().await;
        let uc = setup_uc(&pool).await;
        let account_id = create_account(&pool).await;
        let asset_id = create_asset(&pool).await;
        let micro = 1_000_000i64;

        // Establish a holding first (record_price: false — no noise in asset_prices)
        uc.create_transaction(buy_dto(&account_id, &asset_id, 2 * micro))
            .await
            .unwrap();

        let sell = CreateTransactionDTO {
            record_price: true,
            ..sell_dto(&account_id, &asset_id, micro)
        };
        let expected_price = sell.unit_price;
        let expected_date = sell.date.clone();

        uc.create_transaction(sell).await.unwrap();

        let rows = sqlx::query!(
            "SELECT asset_id, date, price FROM asset_prices WHERE asset_id = ?",
            asset_id
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(
            rows.len(),
            1,
            "expected exactly one AssetPrice row from the sell"
        );
        assert_eq!(rows[0].asset_id, asset_id);
        assert_eq!(rows[0].date, expected_date);
        assert_eq!(rows[0].price, expected_price);
    }

    // MKT-055 — update_transaction with record_price=true writes an AssetPrice row
    #[tokio::test]
    async fn update_transaction_with_record_price_writes_asset_price() {
        let pool = make_pool().await;
        let uc = setup_uc(&pool).await;
        let account_id = create_account(&pool).await;
        let asset_id = create_asset(&pool).await;
        let micro = 1_000_000i64;

        // Create purchase with record_price: false — no price written yet
        let tx = uc
            .create_transaction(buy_dto(&account_id, &asset_id, micro))
            .await
            .unwrap();

        // Update: same transaction, but now record_price: true and different unit_price
        let new_unit_price = 150 * micro;
        let update_dto = CreateTransactionDTO {
            record_price: true,
            unit_price: new_unit_price,
            ..buy_dto(&account_id, &asset_id, micro)
        };
        let expected_date = update_dto.date.clone();

        uc.update_transaction(tx.id, update_dto).await.unwrap();

        let rows = sqlx::query!(
            "SELECT asset_id, date, price FROM asset_prices WHERE asset_id = ?",
            asset_id
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(
            rows.len(),
            1,
            "expected exactly one AssetPrice row after update"
        );
        assert_eq!(rows[0].asset_id, asset_id);
        assert_eq!(rows[0].date, expected_date);
        assert_eq!(rows[0].price, new_unit_price);
    }

    // MKT-054 — record_price=false does NOT write any AssetPrice row
    #[tokio::test]
    async fn record_price_false_does_not_write_asset_price() {
        let pool = make_pool().await;
        let uc = setup_uc(&pool).await;
        let account_id = create_account(&pool).await;
        let asset_id = create_asset(&pool).await;
        let micro = 1_000_000i64;

        // record_price: false (the default in buy_dto)
        uc.create_transaction(buy_dto(&account_id, &asset_id, micro))
            .await
            .unwrap();

        let rows = sqlx::query!(
            "SELECT asset_id FROM asset_prices WHERE asset_id = ?",
            asset_id
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert!(
            rows.is_empty(),
            "expected no AssetPrice rows when record_price is false"
        );
    }

    // MKT-058 — same-date collision is silently overwritten; exactly one row remains
    #[tokio::test]
    async fn same_date_collision_overwrites_silently() {
        let pool = make_pool().await;
        let uc = setup_uc(&pool).await;
        let account_id = create_account(&pool).await;
        let asset_id = create_asset(&pool).await;
        let micro = 1_000_000i64;

        // Pre-insert an AssetPrice row at the same (asset_id, date) with an old price
        let old_price = 50 * micro;
        sqlx::query!(
            "INSERT INTO asset_prices (asset_id, date, price) VALUES (?, ?, ?)",
            asset_id,
            "2024-01-01",
            old_price
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create a transaction on the same date with record_price: true and a different price
        let new_unit_price = 100 * micro;
        let dto = CreateTransactionDTO {
            record_price: true,
            unit_price: new_unit_price,
            ..buy_dto(&account_id, &asset_id, micro)
        };
        // buy_dto uses date "2024-01-01" which collides with the pre-inserted row
        uc.create_transaction(dto).await.unwrap();

        let rows = sqlx::query!(
            "SELECT price FROM asset_prices WHERE asset_id = ? AND date = ?",
            asset_id,
            "2024-01-01"
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(
            rows.len(),
            1,
            "expected exactly one row after collision (no duplicate)"
        );
        assert_eq!(
            rows[0].price, new_unit_price,
            "old price should be silently overwritten with new unit_price"
        );
    }

    // MKT-061 — zero unit_price skips the AssetPrice write; transaction still succeeds
    #[tokio::test]
    async fn zero_unit_price_skips_auto_record() {
        let pool = make_pool().await;
        let uc = setup_uc(&pool).await;
        let account_id = create_account(&pool).await;
        let asset_id = create_asset(&pool).await;
        let micro = 1_000_000i64;

        // unit_price=0 is allowed only when fees > 0 (TRX-020 requires total_amount > 0).
        // This matches MKT-061's gifted/inherited asset scenario where a recording fee exists.
        let dto = CreateTransactionDTO {
            record_price: true,
            unit_price: 0,
            fees: micro,
            ..buy_dto(&account_id, &asset_id, micro)
        };

        // Transaction must succeed normally
        let result = uc.create_transaction(dto).await;
        assert!(
            result.is_ok(),
            "transaction with unit_price=0 should succeed: {:?}",
            result
        );

        // No AssetPrice row should have been written
        let rows = sqlx::query!(
            "SELECT asset_id FROM asset_prices WHERE asset_id = ?",
            asset_id
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert!(
            rows.is_empty(),
            "expected no AssetPrice row when unit_price is 0 (MKT-061 skip)"
        );
    }

    // MKT-059 — editing to a new date leaves the old price row intact and creates a new one
    #[tokio::test]
    async fn edit_to_new_date_preserves_old_price_and_creates_new() {
        let pool = make_pool().await;
        let uc = setup_uc(&pool).await;
        let account_id = create_account(&pool).await;
        let asset_id = create_asset(&pool).await;
        let micro = 1_000_000i64;

        // Create purchase at "2024-01-01" with record_price: true (original price = 100 * micro)
        let original_unit_price = 100 * micro;
        let tx = uc
            .create_transaction(CreateTransactionDTO {
                record_price: true,
                unit_price: original_unit_price,
                ..buy_dto(&account_id, &asset_id, micro)
            })
            .await
            .unwrap();

        // Update: move date to "2024-06-01", new unit_price, record_price: true
        let new_unit_price = 200 * micro;
        let update_dto = CreateTransactionDTO {
            record_price: true,
            date: "2024-06-01".to_string(),
            unit_price: new_unit_price,
            ..buy_dto(&account_id, &asset_id, micro)
        };

        uc.update_transaction(tx.id, update_dto).await.unwrap();

        // Row at original date must still exist with original price
        let old_rows = sqlx::query!(
            "SELECT price FROM asset_prices WHERE asset_id = ? AND date = ?",
            asset_id,
            "2024-01-01"
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(
            old_rows.len(),
            1,
            "original AssetPrice row at 2024-01-01 should be untouched after date change"
        );
        assert_eq!(
            old_rows[0].price, original_unit_price,
            "price at original date should be unchanged"
        );

        // Row at new date must also exist with the new price
        let new_rows = sqlx::query!(
            "SELECT price FROM asset_prices WHERE asset_id = ? AND date = ?",
            asset_id,
            "2024-06-01"
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(
            new_rows.len(),
            1,
            "new AssetPrice row at 2024-06-01 should have been created"
        );
        assert_eq!(
            new_rows[0].price, new_unit_price,
            "price at new date should equal the updated unit_price"
        );
    }

    // MKT-060 — deleting a transaction does NOT remove the AssetPrice row written by it
    #[tokio::test]
    async fn delete_does_not_cascade_to_asset_price() {
        let pool = make_pool().await;
        let uc = setup_uc(&pool).await;
        let account_id = create_account(&pool).await;
        let asset_id = create_asset(&pool).await;
        let micro = 1_000_000i64;

        let dto = CreateTransactionDTO {
            record_price: true,
            ..buy_dto(&account_id, &asset_id, micro)
        };
        let expected_date = dto.date.clone();
        let expected_price = dto.unit_price;

        let tx = uc.create_transaction(dto).await.unwrap();

        // Confirm the price row exists before deletion
        let before = sqlx::query!(
            "SELECT price FROM asset_prices WHERE asset_id = ? AND date = ?",
            asset_id,
            expected_date
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(before.len(), 1, "price row should exist before delete");

        // Delete the transaction
        uc.delete_transaction(&tx.id).await.unwrap();

        // Price row must still be present after deletion
        let after = sqlx::query!(
            "SELECT price FROM asset_prices WHERE asset_id = ? AND date = ?",
            asset_id,
            expected_date
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(
            after.len(),
            1,
            "AssetPrice row must survive transaction deletion (MKT-060)"
        );
        assert_eq!(
            after[0].price, expected_price,
            "price value must be unchanged after transaction deletion"
        );
    }

    // MKT-056 — atomicity: full rollback when any step inside the DB transaction fails.
    // MKT-062 — auto-record failure surfaces through the existing add/update_transaction
    // error contract; same rollback semantics as MKT-056. Both rules share the same path
    // (the price upsert is a step inside the orchestrator's open DB transaction), so a
    // single fault-injection test will cover both once the seam exists.
    // TODO(MKT-056, MKT-062): atomicity test deferred — requires fault injection at the
    // DB layer (e.g. a mock repository that errors after the price write but before
    // commit). The current integration-test style (real SQLite pool) cannot force a
    // mid-transaction failure without infrastructure-level seams not yet present in this
    // codebase.
}
