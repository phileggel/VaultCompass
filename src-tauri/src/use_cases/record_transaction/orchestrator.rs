use crate::context::account::{AccountService, Transaction, TransactionType};
use crate::context::asset::AssetService;
use crate::core::logger::BACKEND;
use crate::use_cases::record_transaction::error::RecordTransactionError;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;
use tracing::warn;

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
    /// MKT-054 — when true and unit_price > 0, also upserts AssetPrice(asset_id, date, unit_price)
    /// as a best-effort separate write after the transaction is saved (MKT-055/056).
    /// Existing same-date price is silently overwritten (MKT-058). Skipped when unit_price = 0 (MKT-061).
    pub record_price: bool,
}

/// Thin orchestrator for transaction create/update/delete.
/// Cross-BC concerns (asset validation, auto_record_price, unarchive) are handled here;
/// holding recalculation and atomicity are delegated to AccountService (B21, B4).
pub struct RecordTransactionUseCase {
    account_service: Arc<AccountService>,
    asset_service: Arc<AssetService>,
}

impl RecordTransactionUseCase {
    /// Creates a new RecordTransactionUseCase.
    pub fn new(account_service: Arc<AccountService>, asset_service: Arc<AssetService>) -> Self {
        Self {
            account_service,
            asset_service,
        }
    }

    /// Creates a new Purchase or Sell transaction and updates the Holding (TRX-027, SEL-028).
    pub async fn create_transaction(&self, dto: CreateTransactionDTO) -> Result<Transaction> {
        let tx_type = dto
            .transaction_type
            .parse::<TransactionType>()
            .map_err(|_| RecordTransactionError::InvalidType)?;

        let asset = self
            .asset_service
            .get_asset_by_id(&dto.asset_id)
            .await?
            .ok_or(RecordTransactionError::AssetNotFound)?;

        self.account_service
            .get_by_id(&dto.account_id)
            .await?
            .ok_or(RecordTransactionError::AccountNotFound)?;

        match tx_type {
            TransactionType::Purchase => {
                let tx = self
                    .account_service
                    .buy_holding(
                        &dto.account_id,
                        dto.asset_id.clone(),
                        dto.date,
                        dto.quantity,
                        dto.unit_price,
                        dto.exchange_rate,
                        dto.fees,
                        dto.note,
                    )
                    .await?;
                if asset.is_archived {
                    // TRX-028 — best-effort unarchive after the transaction is saved
                    if let Err(e) = self.asset_service.unarchive_asset(&dto.asset_id).await {
                        warn!(target: BACKEND, asset_id = %dto.asset_id, err = %e, "failed to auto-unarchive asset (TRX-028)");
                    }
                }
                self.maybe_record_price(dto.record_price, &tx).await;
                Ok(tx)
            }
            TransactionType::Sell => {
                // SEL-037 — reject sell on archived asset
                if asset.is_archived {
                    return Err(RecordTransactionError::ArchivedAssetSell.into());
                }
                let tx = self
                    .account_service
                    .sell_holding(
                        &dto.account_id,
                        dto.asset_id.clone(),
                        dto.date,
                        dto.quantity,
                        dto.unit_price,
                        dto.exchange_rate,
                        dto.fees,
                        dto.note,
                    )
                    .await?;
                self.maybe_record_price(dto.record_price, &tx).await;
                Ok(tx)
            }
        }
    }

    /// Updates an existing transaction and recalculates the affected Holding (TRX-031, SEL-031).
    pub async fn update_transaction(
        &self,
        id: String,
        dto: CreateTransactionDTO,
    ) -> Result<Transaction> {
        let existing = self
            .account_service
            .get_transaction_by_id(&id)
            .await?
            .ok_or(RecordTransactionError::TransactionNotFound)?;

        // SEL-035 — transaction_type immutability
        if existing.transaction_type.to_string() != dto.transaction_type {
            return Err(RecordTransactionError::TypeImmutable.into());
        }

        let asset = self
            .asset_service
            .get_asset_by_id(&existing.asset_id)
            .await?
            .ok_or(RecordTransactionError::AssetNotFound)?;

        if asset.is_archived && existing.transaction_type == TransactionType::Sell {
            return Err(RecordTransactionError::ArchivedAssetSell.into());
        }
        let needs_unarchive =
            asset.is_archived && existing.transaction_type == TransactionType::Purchase;

        // account_id and asset_id are immutable — use the existing tx's values
        let tx = self
            .account_service
            .correct_transaction(
                &existing.account_id,
                &id,
                dto.date,
                dto.quantity,
                dto.unit_price,
                dto.exchange_rate,
                dto.fees,
                dto.note,
            )
            .await?;

        if needs_unarchive {
            if let Err(e) = self.asset_service.unarchive_asset(&existing.asset_id).await {
                warn!(target: BACKEND, asset_id = %existing.asset_id, err = %e, "failed to auto-unarchive asset on update");
            }
        }
        self.maybe_record_price(dto.record_price, &tx).await;
        Ok(tx)
    }

    /// Deletes a transaction and recalculates (or removes) the associated Holding (TRX-034).
    pub async fn delete_transaction(&self, id: &str) -> Result<()> {
        let existing = self
            .account_service
            .get_transaction_by_id(id)
            .await?
            .ok_or(RecordTransactionError::TransactionNotFound)?;
        self.account_service
            .cancel_transaction(&existing.account_id, id)
            .await
    }

    /// Returns all transactions for an account/asset pair.
    pub async fn get_transactions(
        &self,
        account_id: &str,
        asset_id: &str,
    ) -> Result<Vec<Transaction>> {
        self.account_service
            .get_transactions(account_id, asset_id)
            .await
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    /// MKT-055/061 — best-effort price recording after transaction save.
    /// Logs on error but never fails the parent operation.
    async fn maybe_record_price(&self, record_price: bool, tx: &Transaction) {
        if !record_price || tx.unit_price <= 0 {
            return;
        }
        let price_f64 = tx.unit_price as f64 / 1_000_000.0;
        if let Err(e) = self
            .asset_service
            .record_price(&tx.asset_id, &tx.date, price_f64)
            .await
        {
            warn!(
                target: BACKEND,
                asset_id = %tx.asset_id,
                date = %tx.date,
                err = %e,
                "failed to auto-record asset price (MKT-055)"
            );
        }
    }
}
