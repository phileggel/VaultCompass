use crate::context::account::{AccountRepository, Holding, HoldingRepository};
use crate::context::asset::AssetRepository;
use crate::context::transaction::{Transaction, TransactionService, TransactionType};
use crate::core::logger::BACKEND;
use anyhow::{bail, Context, Result};
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
            .map_err(|_| anyhow::anyhow!("Unknown transaction_type: {}", dto.transaction_type))?;

        // TRX-020 / SEL-020 — validate asset and account exist
        let asset = self
            .asset_repo
            .get_by_id(&dto.asset_id)
            .await?
            .with_context(|| format!("Asset {} not found", dto.asset_id))?;

        self.account_repo
            .get_by_id(&dto.account_id)
            .await?
            .with_context(|| format!("Account {} not found", dto.account_id))?;

        match tx_type {
            TransactionType::Purchase => self.create_purchase(dto, asset.is_archived).await,
            TransactionType::Sell => {
                // SEL-037 — reject sell on archived asset
                if asset.is_archived {
                    bail!("Cannot sell an archived asset");
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
            bail!("No units available to sell");
        }

        // SEL-021 — oversell guard
        if dto.quantity > holding_before.quantity {
            bail!(
                "Quantity exceeds current holding ({} available)",
                holding_before.quantity
            );
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
            .with_context(|| format!("Transaction {} not found", id))?;

        // SEL-035 — transaction_type immutability
        let existing_type_str = existing_tx.transaction_type.to_string();
        if existing_type_str != dto.transaction_type {
            bail!(
                "Cannot change transaction type from {} to {}",
                existing_type_str,
                dto.transaction_type
            );
        }

        let tx_type = existing_tx.transaction_type;
        let old_account_id = existing_tx.account_id.clone();
        let old_asset_id = existing_tx.asset_id.clone();

        // SEL-020 / TRX-033 — validate new asset and account exist
        let asset = self
            .asset_repo
            .get_by_id(&dto.asset_id)
            .await?
            .with_context(|| format!("Asset {} not found", dto.asset_id))?;

        self.account_repo
            .get_by_id(&dto.account_id)
            .await?
            .with_context(|| format!("Account {} not found", dto.account_id))?;

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
            .with_context(|| format!("Transaction {} not found", id))?;

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
        let mut pnl_map: HashMap<String, i64> = HashMap::new();

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
                        bail!(
                            "Sell transaction {} would exceed available quantity at that point",
                            t.id
                        );
                    }
                    let vwap_before: i64 = if total_quantity > 0 {
                        (vwap_numerator / total_quantity) as i64
                    } else {
                        0
                    };
                    // SEL-024 — realized P&L
                    let pnl = Self::compute_realized_pnl(t.total_amount, vwap_before, t.quantity);
                    pnl_map.insert(t.id.clone(), pnl);
                    // SEL-025 — decrease quantity; SEL-027 — VWAP numerator scales with qty
                    let qty = t.quantity as i128;
                    vwap_numerator -= vwap_before as i128 * qty;
                    total_quantity -= qty;
                }
            }
        }

        let average_price: i64 = if total_quantity > 0 {
            (vwap_numerator / total_quantity) as i64
        } else {
            0
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
            )?,
            None => Holding::new(
                account_id.to_string(),
                asset_id.to_string(),
                quantity,
                average_price,
            )?,
        };

        Ok((holding, pnl_map))
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
