use super::error::{AccountDomainError, AccountOperationError};
use super::holding::Holding;
use super::transaction::{Transaction, TransactionType};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use iso_currency::Currency;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::str::FromStr;
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

/// A single change produced by an aggregate operation, applied atomically by the repository.
#[derive(Debug, Clone)]
pub enum AccountChange {
    /// A new transaction was created.
    TransactionInserted(Transaction),
    /// An existing transaction's fields were updated.
    TransactionUpdated(Transaction),
    /// A transaction was permanently removed.
    TransactionDeleted(String),
    /// A holding was created or updated (upsert).
    HoldingUpserted(Holding),
    /// A holding was removed (no transactions remain for the pair).
    HoldingDeleted {
        /// Account the holding belonged to.
        account_id: String,
        /// Asset the holding represented.
        asset_id: String,
    },
}

/// Represents a financial account — the Aggregate Root of the Account bounded context.
/// Owns all holdings and transactions for this account.
///
/// The `holdings`, `transactions`, and `pending_changes` fields are populated only
/// when the aggregate is loaded for mutation via `AccountRepository::get_with_holdings_and_transactions`.
/// They are excluded from Tauri serialization and TypeScript bindings.
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
pub struct Account {
    /// Unique identifier (uuid).
    pub id: String,
    /// User defined name.
    pub name: String,
    /// ISO 4217 currency code for this account (TRX-021).
    pub currency: String,
    /// How often this account is updated.
    pub update_frequency: UpdateFrequency,
    /// Holdings owned by this account. Populated only in aggregate load — excluded from bindings.
    #[serde(skip)]
    #[specta(skip)]
    pub holdings: Vec<Holding>,
    /// Transactions owned by this account. Populated only in aggregate load — excluded from bindings.
    #[serde(skip)]
    #[specta(skip)]
    pub transactions: Vec<Transaction>,
    /// Pending changes to persist atomically. Drained by `AccountRepository::save` on success.
    #[serde(skip)]
    #[specta(skip)]
    pub(crate) pending_changes: Vec<AccountChange>,
}

impl Account {
    /// Creates a new Account. Trims the name before validation and storage (R1).
    pub fn new(name: String, currency: String, update_frequency: UpdateFrequency) -> Result<Self> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(AccountDomainError::NameEmpty.into());
        }
        Self::validate_currency(&currency)?;
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            name,
            currency,
            update_frequency,
            holdings: Vec::new(),
            transactions: Vec::new(),
            pending_changes: Vec::new(),
        })
    }

    /// Updates an existing Account. Trims and validates identically to new() (R1, R2).
    pub fn with_id(
        id: String,
        name: String,
        currency: String,
        update_frequency: UpdateFrequency,
    ) -> Result<Self> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(AccountDomainError::NameEmpty.into());
        }
        Self::validate_currency(&currency)?;
        Ok(Self {
            id,
            name,
            currency,
            update_frequency,
            holdings: Vec::new(),
            transactions: Vec::new(),
            pending_changes: Vec::new(),
        })
    }

    /// Reconstructs a thin Account from storage without validation (CRUD load — no aggregate data).
    pub fn restore(
        id: String,
        name: String,
        currency: String,
        update_frequency: UpdateFrequency,
    ) -> Self {
        Self {
            id,
            name,
            currency,
            update_frequency,
            holdings: Vec::new(),
            transactions: Vec::new(),
            pending_changes: Vec::new(),
        }
    }

    /// Reconstructs an Account with its full aggregate state from storage.
    /// Used exclusively by `AccountRepository::get_with_holdings_and_transactions`.
    pub fn restore_with_positions(
        id: String,
        name: String,
        currency: String,
        update_frequency: UpdateFrequency,
        holdings: Vec<Holding>,
        transactions: Vec<Transaction>,
    ) -> Self {
        Self {
            id,
            name,
            currency,
            update_frequency,
            holdings,
            transactions,
            pending_changes: Vec::new(),
        }
    }

    /// Returns the pending changes accumulated by aggregate operations since last save.
    pub fn pending_changes(&self) -> &[AccountChange] {
        &self.pending_changes
    }

    // -------------------------------------------------------------------------
    // Aggregate Root methods (B28 — domain/business vocabulary)
    // -------------------------------------------------------------------------

    /// Records a purchase of an asset into this account (TRX-020, TRX-026).
    ///
    /// Creates a Transaction internally, then upserts the Holding with the updated
    /// VWAP and quantity. Enqueues the changes for atomic persistence.
    #[allow(clippy::too_many_arguments)]
    pub fn buy_holding(
        &mut self,
        asset_id: String,
        date: String,
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        fees: i64,
        note: Option<String>,
    ) -> Result<&Transaction> {
        let total_amount = Self::compute_purchase_total(quantity, unit_price, exchange_rate, fees);
        let tx = Transaction::new(
            self.id.clone(),
            asset_id.clone(),
            TransactionType::Purchase,
            date,
            quantity,
            unit_price,
            exchange_rate,
            fees,
            total_amount,
            note,
            None,
        )?;
        self.transactions.push(tx);
        let tx_ref = self
            .transactions
            .last()
            .ok_or_else(|| anyhow!("BUG: tx list empty after push in account {}", self.id))?;

        let pair_txs: Vec<&Transaction> = self
            .transactions
            .iter()
            .filter(|t| t.asset_id == asset_id)
            .collect();
        let (holding, _) = self.recalculate_holding(&asset_id, &pair_txs)?;

        self.pending_changes
            .push(AccountChange::TransactionInserted(tx_ref.clone()));
        self.pending_changes
            .push(AccountChange::HoldingUpserted(holding.clone()));
        self.upsert_holding_in_memory(holding);

        self.transactions
            .last()
            .ok_or_else(|| anyhow!("BUG: tx list empty after push in account {}", self.id))
    }

    /// Records a sale of an asset from this account (SEL-012, SEL-021, SEL-023, SEL-024).
    ///
    /// Validates the position is open and the quantity is available, creates a Transaction,
    /// updates the Holding with the recalculated VWAP and realized P&L.
    #[allow(clippy::too_many_arguments)]
    pub fn sell_holding(
        &mut self,
        asset_id: String,
        date: String,
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        fees: i64,
        note: Option<String>,
    ) -> Result<&Transaction> {
        // SEL-012 — closed position guard
        let current_qty = self.holding_quantity(&asset_id);
        if current_qty == 0 {
            return Err(AccountOperationError::ClosedPosition.into());
        }
        // SEL-021 — oversell guard
        if quantity > current_qty {
            return Err(AccountOperationError::Oversell {
                available: current_qty,
                requested: quantity,
            }
            .into());
        }

        let total_amount = Self::compute_sell_total(quantity, unit_price, exchange_rate, fees);
        let tx = Transaction::new(
            self.id.clone(),
            asset_id.clone(),
            TransactionType::Sell,
            date,
            quantity,
            unit_price,
            exchange_rate,
            fees,
            total_amount,
            note,
            None, // realized_pnl computed below
        )?;
        self.transactions.push(tx);

        let pair_txs: Vec<&Transaction> = self
            .transactions
            .iter()
            .filter(|t| t.asset_id == asset_id)
            .collect();
        let (holding, pnl_map) = self.recalculate_holding(&asset_id, &pair_txs)?;

        // Attach computed realized_pnl to the new sell transaction
        let tx_ref = self
            .transactions
            .last_mut()
            .ok_or_else(|| anyhow!("BUG: tx list empty after push in account {}", self.id))?;
        let realized_pnl = pnl_map.get(&tx_ref.id).copied();
        tx_ref.realized_pnl = realized_pnl;
        let tx_snapshot = tx_ref.clone();

        self.pending_changes
            .push(AccountChange::TransactionInserted(tx_snapshot));
        self.pending_changes
            .push(AccountChange::HoldingUpserted(holding.clone()));
        self.upsert_holding_in_memory(holding);

        self.transactions
            .last()
            .ok_or_else(|| anyhow!("BUG: tx list empty after push in account {}", self.id))
    }

    /// Corrects the fields of an existing transaction and recalculates the affected holding
    /// (TRX-031, SEL-031, SEL-032).
    ///
    /// The transaction type is immutable — `correct_transaction` preserves it.
    /// Performs a cascading oversell check after recalculation.
    #[allow(clippy::too_many_arguments)]
    pub fn correct_transaction(
        &mut self,
        tx_id: &str,
        date: String,
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        fees: i64,
        note: Option<String>,
    ) -> Result<&Transaction> {
        let existing = self
            .transactions
            .iter()
            .find(|t| t.id == tx_id)
            .ok_or(AccountOperationError::TransactionNotFound)?;

        let tx_type = existing.transaction_type;
        let asset_id = existing.asset_id.clone();
        let created_at = existing.created_at.clone();

        let total_amount = match tx_type {
            TransactionType::Purchase => {
                Self::compute_purchase_total(quantity, unit_price, exchange_rate, fees)
            }
            TransactionType::Sell => {
                Self::compute_sell_total(quantity, unit_price, exchange_rate, fees)
            }
        };

        let updated_tx = Transaction::with_id(
            tx_id.to_string(),
            self.id.clone(),
            asset_id.clone(),
            tx_type,
            date,
            quantity,
            unit_price,
            exchange_rate,
            fees,
            total_amount,
            note,
            None, // realized_pnl recomputed below
            created_at,
        )?;

        // Replace the transaction in-memory
        if let Some(slot) = self.transactions.iter_mut().find(|t| t.id == tx_id) {
            *slot = updated_tx;
        } else {
            return Err(AccountOperationError::TransactionNotFound.into());
        }

        // Full recalculation for the (account, asset) pair — SEL-032 cascading check inside
        let pair_txs: Vec<&Transaction> = self
            .transactions
            .iter()
            .filter(|t| t.asset_id == asset_id)
            .collect();
        let (holding, pnl_map) = self.recalculate_holding(&asset_id, &pair_txs)?;

        // Attach updated realized_pnl to all sells in the pair (excluding the corrected tx itself,
        // which is handled unconditionally below to cover the Purchase case too)
        for tx in self
            .transactions
            .iter_mut()
            .filter(|t| t.asset_id == asset_id && t.id != tx_id)
        {
            if tx.transaction_type == TransactionType::Sell {
                tx.realized_pnl = pnl_map.get(&tx.id).copied();
                self.pending_changes
                    .push(AccountChange::TransactionUpdated(tx.clone()));
            }
        }
        // The corrected transaction itself — always record so the repository gets the latest state
        let corrected = self
            .transactions
            .iter()
            .find(|t| t.id == tx_id)
            .ok_or_else(|| {
                anyhow!(
                    "BUG: tx {} missing after update in account {}",
                    tx_id,
                    self.id
                )
            })?;
        // Ensure the corrected tx is always recorded (re-push to overwrite any earlier entry;
        // repository applies changes in order so the last write wins)
        self.pending_changes
            .push(AccountChange::TransactionUpdated(corrected.clone()));

        self.pending_changes
            .push(AccountChange::HoldingUpserted(holding.clone()));
        self.upsert_holding_in_memory(holding);

        self.transactions
            .iter()
            .find(|t| t.id == tx_id)
            .ok_or_else(|| {
                anyhow!(
                    "BUG: tx {} missing after update in account {}",
                    tx_id,
                    self.id
                )
            })
    }

    /// Deletes an existing transaction and recalculates (or removes) the associated holding
    /// (TRX-034, SEL-033, SEL-026).
    pub fn cancel_transaction(&mut self, tx_id: &str) -> Result<()> {
        let asset_id = self
            .transactions
            .iter()
            .find(|t| t.id == tx_id)
            .ok_or(AccountOperationError::TransactionNotFound)?
            .asset_id
            .clone();
        let pos = self
            .transactions
            .iter()
            .position(|t| t.id == tx_id)
            .ok_or(AccountOperationError::TransactionNotFound)?;
        self.transactions.remove(pos);
        self.pending_changes
            .push(AccountChange::TransactionDeleted(tx_id.to_string()));

        let remaining: Vec<&Transaction> = self
            .transactions
            .iter()
            .filter(|t| t.asset_id == asset_id)
            .collect();

        if remaining.is_empty() {
            // Remove the holding — no transactions left for this pair
            self.holdings
                .retain(|h| !(h.account_id == self.id && h.asset_id == asset_id));
            self.pending_changes.push(AccountChange::HoldingDeleted {
                account_id: self.id.clone(),
                asset_id,
            });
        } else {
            // SEL-033 — full recalculation updates realized_pnl on remaining sells
            let (holding, pnl_map) = self.recalculate_holding(&asset_id, &remaining)?;
            for tx in self
                .transactions
                .iter_mut()
                .filter(|t| t.asset_id == asset_id && t.transaction_type == TransactionType::Sell)
            {
                tx.realized_pnl = pnl_map.get(&tx.id).copied();
                self.pending_changes
                    .push(AccountChange::TransactionUpdated(tx.clone()));
            }
            self.pending_changes
                .push(AccountChange::HoldingUpserted(holding.clone()));
            self.upsert_holding_in_memory(holding);
        }

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    /// Returns the current quantity for a (account, asset) pair, or 0 if no holding exists.
    fn holding_quantity(&self, asset_id: &str) -> i64 {
        self.holdings
            .iter()
            .find(|h| h.asset_id == asset_id)
            .map(|h| h.quantity)
            .unwrap_or(0)
    }

    /// Upserts a holding in the in-memory list.
    fn upsert_holding_in_memory(&mut self, holding: Holding) {
        if let Some(existing) = self
            .holdings
            .iter_mut()
            .find(|h| h.asset_id == holding.asset_id)
        {
            *existing = holding;
        } else {
            self.holdings.push(holding);
        }
    }

    /// Full chronological recalculation of Holding state and realized P&L for the given
    /// transaction slice (TRX-030, SEL-024, SEL-025, SEL-026, SEL-027, SEL-032).
    ///
    /// Returns `(updated_holding, sell_tx_id → realized_pnl)`.
    /// Returns `AccountOperationError::CascadingOversell` if any sell exceeds running qty.
    fn recalculate_holding(
        &self,
        asset_id: &str,
        transactions: &[&Transaction],
    ) -> Result<(Holding, std::collections::HashMap<String, i64>)> {
        use std::collections::HashMap;
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
                    vwap_numerator += t.total_amount as i128 * MICRO;
                }
                TransactionType::Sell => {
                    if t.quantity as i128 > total_quantity {
                        return Err(AccountOperationError::CascadingOversell.into());
                    }
                    let vwap_before: i64 = if total_quantity > 0 {
                        (vwap_numerator / total_quantity) as i64
                    } else {
                        0
                    };
                    last_vwap = vwap_before;
                    let pnl = Self::compute_realized_pnl(t.total_amount, vwap_before, t.quantity);
                    pnl_map.insert(t.id.clone(), pnl);
                    total_realized_pnl += pnl;
                    if last_sold_date.as_deref() < Some(t.date.as_str()) {
                        last_sold_date = Some(t.date.clone());
                    }
                    let qty = t.quantity as i128;
                    vwap_numerator -= vwap_before as i128 * qty;
                    total_quantity -= qty;
                }
            }
        }

        // SEL-026 / TRX-040 — retain holding at qty=0, preserve last VWAP
        let average_price: i64 = if total_quantity > 0 {
            (vwap_numerator / total_quantity) as i64
        } else {
            last_vwap
        };
        let quantity = total_quantity as i64;

        let holding = match self.holdings.iter().find(|h| h.asset_id == asset_id) {
            Some(existing) => Holding::with_id(
                existing.id.clone(),
                self.id.clone(),
                asset_id.to_string(),
                quantity,
                average_price,
                total_realized_pnl,
                last_sold_date,
            )?,
            None => Holding::new(
                self.id.clone(),
                asset_id.to_string(),
                quantity,
                average_price,
                total_realized_pnl,
                last_sold_date,
            )?,
        };

        Ok((holding, pnl_map))
    }

    /// Computes total_amount for a Purchase (TRX-026).
    /// Formula: floor(floor(qty × price / MICRO) × rate / MICRO) + fees
    fn compute_purchase_total(
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        fees: i64,
    ) -> i64 {
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

    fn validate_currency(currency: &str) -> Result<()> {
        if Currency::from_str(currency).is_err() {
            return Err(AccountDomainError::InvalidCurrency(currency.to_string()).into());
        }
        Ok(())
    }
}

/// Interface for account persistence.
#[cfg_attr(test, mockall::automock)]
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
    /// Loads the full aggregate: account + all holdings + all transactions (ordered by date, created_at).
    async fn get_with_holdings_and_transactions(&self, id: &str) -> Result<Option<Account>>;
    /// Atomically applies all pending changes accumulated by aggregate operations.
    /// Clears `pending_changes` on the aggregate after a successful commit.
    async fn save(&self, account: &mut Account) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn micro(v: i64) -> i64 {
        v * 1_000_000
    }

    fn base_account() -> Account {
        Account::restore_with_positions(
            "acc-1".to_string(),
            "Test".to_string(),
            "EUR".to_string(),
            UpdateFrequency::ManualMonth,
            Vec::new(),
            Vec::new(),
        )
    }

    // R1 — trim at creation
    #[test]
    fn new_trims_leading_trailing_spaces() {
        let account = Account::new(
            "  My Account  ".to_string(),
            "EUR".to_string(),
            UpdateFrequency::ManualMonth,
        )
        .unwrap();
        assert_eq!(account.name, "My Account");
    }

    // R1, R2 — spaces-only name is invalid after trim
    #[test]
    fn new_rejects_whitespace_only_name() {
        let result = Account::new(
            "   ".to_string(),
            "EUR".to_string(),
            UpdateFrequency::ManualMonth,
        );
        assert!(result.is_err());
    }

    // currency — invalid ISO code rejected
    #[test]
    fn new_rejects_invalid_currency() {
        let result = Account::new(
            "My Account".to_string(),
            "INVALID".to_string(),
            UpdateFrequency::ManualMonth,
        );
        assert!(result.is_err());
    }

    // R1, R2 — with_id trims and validates
    #[test]
    fn with_id_trims_name() {
        let account = Account::with_id(
            "some-id".to_string(),
            "  Trimmed  ".to_string(),
            "USD".to_string(),
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
            "EUR".to_string(),
            UpdateFrequency::ManualMonth,
        );
        assert!(result.is_err());
    }

    // TRX-026 / TRX-030 — buy_holding updates VWAP correctly (2 purchases)
    #[test]
    fn buy_holding_updates_vwap() {
        let mut acc = base_account();
        // Buy 2 units @ 100.00 → total = 200.00
        acc.buy_holding(
            "asset-1".to_string(),
            "2024-01-01".to_string(),
            micro(2),
            micro(100),
            micro(1),
            0,
            None,
        )
        .unwrap();
        // Buy 2 units @ 200.00 → total = 400.00; VWAP = 600/4 = 150.00
        acc.buy_holding(
            "asset-1".to_string(),
            "2024-02-01".to_string(),
            micro(2),
            micro(200),
            micro(1),
            0,
            None,
        )
        .unwrap();

        let h = acc
            .holdings
            .iter()
            .find(|h| h.asset_id == "asset-1")
            .unwrap();
        assert_eq!(h.quantity, micro(4));
        assert_eq!(h.average_price, micro(150));
    }

    // SEL-012 — sell_holding on a zero-qty position is rejected
    #[test]
    fn sell_holding_rejects_closed_position() {
        let mut acc = base_account();
        let err = acc
            .sell_holding(
                "asset-1".to_string(),
                "2024-01-01".to_string(),
                micro(1),
                micro(100),
                micro(1),
                0,
                None,
            )
            .unwrap_err();
        assert!(
            err.downcast_ref::<AccountOperationError>()
                .map(|e| matches!(e, AccountOperationError::ClosedPosition))
                .unwrap_or(false),
            "expected ClosedPosition, got: {err}"
        );
    }

    // SEL-021 — sell_holding rejects quantity exceeding available
    #[test]
    fn sell_holding_rejects_oversell() {
        let mut acc = base_account();
        acc.buy_holding(
            "asset-1".to_string(),
            "2024-01-01".to_string(),
            micro(1),
            micro(100),
            micro(1),
            0,
            None,
        )
        .unwrap();
        let err = acc
            .sell_holding(
                "asset-1".to_string(),
                "2024-06-01".to_string(),
                micro(2),
                micro(100),
                micro(1),
                0,
                None,
            )
            .unwrap_err();
        assert!(
            err.downcast_ref::<AccountOperationError>()
                .map(|e| matches!(e, AccountOperationError::Oversell { .. }))
                .unwrap_or(false),
            "expected Oversell, got: {err}"
        );
    }

    // SEL-024 — sell_holding computes P&L: sell 1 unit @ 150 after buying @ 100 → P&L = +50
    #[test]
    fn sell_holding_computes_realized_pnl() {
        let mut acc = base_account();
        acc.buy_holding(
            "asset-1".to_string(),
            "2024-01-01".to_string(),
            micro(1),
            micro(100),
            micro(1),
            0,
            None,
        )
        .unwrap();
        let tx = acc
            .sell_holding(
                "asset-1".to_string(),
                "2024-06-01".to_string(),
                micro(1),
                micro(150),
                micro(1),
                0,
                None,
            )
            .unwrap();
        assert_eq!(tx.realized_pnl, Some(micro(50)));
    }

    // TRX-031 — correct_transaction recalculates holding
    #[test]
    fn correct_transaction_recalculates_holding() {
        let mut acc = base_account();
        let tx = acc
            .buy_holding(
                "asset-1".to_string(),
                "2024-01-01".to_string(),
                micro(2),
                micro(100),
                micro(1),
                0,
                None,
            )
            .unwrap()
            .clone();

        // Correct: change unit_price to 200 → total = 400, VWAP = 200
        acc.correct_transaction(
            &tx.id,
            "2024-01-01".to_string(),
            micro(2),
            micro(200),
            micro(1),
            0,
            None,
        )
        .unwrap();

        let h = acc
            .holdings
            .iter()
            .find(|h| h.asset_id == "asset-1")
            .unwrap();
        assert_eq!(h.average_price, micro(200));
    }

    // TRX-034 — cancel_transaction removes holding when it was the last transaction
    #[test]
    fn cancel_transaction_removes_holding_when_last() {
        let mut acc = base_account();
        let tx = acc
            .buy_holding(
                "asset-1".to_string(),
                "2024-01-01".to_string(),
                micro(1),
                micro(100),
                micro(1),
                0,
                None,
            )
            .unwrap()
            .clone();

        acc.cancel_transaction(&tx.id).unwrap();

        assert!(
            acc.holdings.iter().all(|h| h.asset_id != "asset-1"),
            "holding should be removed"
        );
        assert!(acc.transactions.is_empty(), "transaction should be removed");
    }

    // SEL-026 — cancel_transaction retains holding at qty=0 when other transactions remain
    #[test]
    fn cancel_transaction_retains_holding_when_transactions_remain() {
        let mut acc = base_account();
        acc.buy_holding(
            "asset-1".to_string(),
            "2024-01-01".to_string(),
            micro(2),
            micro(100),
            micro(1),
            0,
            None,
        )
        .unwrap();
        let sell_tx = acc
            .sell_holding(
                "asset-1".to_string(),
                "2024-06-01".to_string(),
                micro(2),
                micro(150),
                micro(1),
                0,
                None,
            )
            .unwrap()
            .clone();

        // Cancel the sell → holding should remain at qty=2 with VWAP=100
        acc.cancel_transaction(&sell_tx.id).unwrap();

        let h = acc
            .holdings
            .iter()
            .find(|h| h.asset_id == "asset-1")
            .unwrap();
        assert_eq!(h.quantity, micro(2));
        assert_eq!(h.average_price, micro(100));
    }
}
