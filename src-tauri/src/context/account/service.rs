use super::domain::{
    Account, AccountRepository, FeeSchedule, FeeScheduleRepository, Holding, HoldingRepository,
    HoldingSnapshot, Transaction, TransactionRepository, UpdateFrequency,
};
use super::error::AccountError;
use crate::core::{logger::BACKEND, Event, SideEffectEventBus};
use crate::use_cases::holding_transaction::OpenHoldingError;
use chrono::NaiveDate;
use std::result::Result as StdResult;
use std::sync::Arc;
use tracing::info;

/// Orchestrates business logic for the Account bounded context.
pub struct AccountService {
    account_repo: Box<dyn AccountRepository>,
    holding_repo: Box<dyn HoldingRepository>,
    transaction_repo: Box<dyn TransactionRepository>,
    event_bus: Option<Arc<SideEffectEventBus>>,
    fee_schedule_repo: Option<Box<dyn FeeScheduleRepository>>,
}

impl AccountService {
    /// Creates a new AccountService.
    pub fn new(
        account_repo: Box<dyn AccountRepository>,
        holding_repo: Box<dyn HoldingRepository>,
        transaction_repo: Box<dyn TransactionRepository>,
    ) -> Self {
        Self {
            account_repo,
            holding_repo,
            transaction_repo,
            event_bus: None,
            fee_schedule_repo: None,
        }
    }

    /// Attaches an event bus for side-effect notifications.
    pub fn with_event_bus(mut self, bus: Arc<SideEffectEventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Attaches the fee-schedule repository (FEE-030) — required for the
    /// `*_fee_schedule` methods; absent in constructions that never touch them.
    pub fn with_fee_schedule_repo(mut self, repo: Box<dyn FeeScheduleRepository>) -> Self {
        self.fee_schedule_repo = Some(repo);
        self
    }

    /// Returns the wired fee-schedule repository or a `DatabaseError` if absent
    /// (a wiring bug — the repo must be attached via `with_fee_schedule_repo`).
    fn fee_schedule_repo(&self) -> StdResult<&dyn FeeScheduleRepository, AccountError> {
        self.fee_schedule_repo.as_deref().ok_or_else(|| {
            tracing::error!(target: BACKEND, "fee_schedule_repo not wired on AccountService");
            AccountError::DatabaseError
        })
    }

    // -------------------------------------------------------------------------
    // Account CRUD
    // -------------------------------------------------------------------------

    /// Retrieves all non-deleted accounts.
    pub async fn get_all(&self) -> StdResult<Vec<Account>, AccountError> {
        self.account_repo.get_all().await.map_err(|e| {
            tracing::error!(target: BACKEND, err = ?e, "get_all: repository failure");
            AccountError::DatabaseError
        })
    }

    /// Retrieves an account by ID.
    pub async fn get_by_id(&self, id: &str) -> StdResult<Option<Account>, AccountError> {
        self.account_repo.get_by_id(id).await.map_err(|e| {
            tracing::error!(target: BACKEND, account_id = %id, err = ?e, "get_by_id: repository failure");
            AccountError::DatabaseError
        })
    }

    /// Creates a new account.
    pub async fn create(
        &self,
        name: String,
        bank_name: String,
        currency: String,
        update_frequency: UpdateFrequency,
        management_fees_enabled: bool,
    ) -> Result<Account, AccountError> {
        let account = Account::new(
            name,
            bank_name,
            currency,
            update_frequency,
            management_fees_enabled,
        )?;
        if find_account_by_name(&*self.account_repo, &account.name)
            .await?
            .is_some()
        {
            return Err(AccountError::NameAlreadyExists);
        }
        info!(target: BACKEND, account_id = %account.id, name = %account.name, "creating account");
        let created = self.account_repo.create(account).await.map_err(|e| {
            tracing::error!(target: BACKEND, err = ?e, "create: repository failure");
            AccountError::DatabaseError
        })?;
        self.emit_account_updated();
        Ok(created)
    }

    /// Seeds the account's 0-balance Cash Holding (CSH-012). Called by the
    /// account-creation use case after the Cash Asset has been ensured (FK).
    /// Idempotent — `Account::seed_cash_holding` is a no-op if one already exists.
    ///
    /// Uses inline load/save logic rather than the module-level
    /// `load_account`/`save_account` helpers.
    pub async fn seed_cash_holding(&self, account_id: &str) -> Result<(), AccountError> {
        let mut account = match self
            .account_repo
            .get_with_holdings_and_transactions(account_id)
            .await
        {
            Ok(Some(acc)) => acc,
            Ok(None) => {
                return Err(AccountError::AccountNotFound {
                    account_id: account_id.to_string(),
                });
            }
            Err(e) => {
                tracing::error!(target: BACKEND, account_id = %account_id, err = ?e, "seed_cash_holding: load failure");
                return Err(AccountError::DatabaseError);
            }
        };
        account.seed_cash_holding();
        self.account_repo.save(&mut account).await.map_err(|e| {
            tracing::error!(target: BACKEND, account_id = %account_id, err = ?e, "seed_cash_holding: save failure");
            AccountError::DatabaseError
        })?;
        self.emit_account_updated();
        Ok(())
    }

    /// Updates an existing account.
    pub async fn update(
        &self,
        id: String,
        name: String,
        bank_name: String,
        currency: String,
        update_frequency: UpdateFrequency,
        management_fees_enabled: bool,
    ) -> Result<Account, AccountError> {
        let account = Account::with_id(
            id,
            name,
            bank_name,
            currency,
            update_frequency,
            management_fees_enabled,
        )?;
        if let Some(existing) = find_account_by_name(&*self.account_repo, &account.name).await? {
            if existing.id != account.id {
                return Err(AccountError::NameAlreadyExists);
            }
        }
        info!(target: BACKEND, account_id = %account.id, name = %account.name, "updating account");
        let updated = self.account_repo.update(account).await.map_err(|e| {
            tracing::error!(target: BACKEND, err = ?e, "update: repository failure");
            AccountError::DatabaseError
        })?;
        self.emit_account_updated();
        Ok(updated)
    }

    /// Permanently deletes an account and cascades to its holdings (R5).
    pub async fn delete(&self, id: &str) -> StdResult<(), AccountError> {
        info!(target: BACKEND, account_id = %id, "deleting account");
        self.account_repo.delete(id).await.map_err(|e| {
            tracing::error!(target: BACKEND, account_id = %id, err = ?e, "delete: repository failure");
            AccountError::DatabaseError
        })?;
        self.emit_account_updated();
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Holding reads
    // -------------------------------------------------------------------------

    /// Retrieves all holdings for a given account (ACD-022, ADR-004).
    pub async fn get_holdings_for_account(
        &self,
        account_id: &str,
    ) -> StdResult<Vec<Holding>, AccountError> {
        self.holding_repo.get_by_account(account_id).await.map_err(|e| {
            tracing::error!(target: BACKEND, account_id = %account_id, err = ?e, "get_holdings_for_account: repository failure");
            AccountError::DatabaseError
        })
    }

    /// Retrieves a single holding by account/asset pair, or None (B19).
    pub async fn get_holding_by_account_asset(
        &self,
        account_id: &str,
        asset_id: &str,
    ) -> StdResult<Option<Holding>, AccountError> {
        self.holding_repo
            .get_by_account_asset(account_id, asset_id)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, account_id = %account_id, asset_id = %asset_id, err = ?e, "get_holding_by_account_asset: repository failure");
                AccountError::DatabaseError
            })
    }

    // -------------------------------------------------------------------------
    // Transaction reads
    // -------------------------------------------------------------------------

    /// Retrieves a transaction by ID.
    pub async fn get_transaction_by_id(
        &self,
        id: &str,
    ) -> StdResult<Option<Transaction>, AccountError> {
        self.transaction_repo.get_by_id(id).await.map_err(|e| {
            tracing::error!(target: BACKEND, transaction_id = %id, err = ?e, "get_transaction_by_id: repository failure");
            AccountError::DatabaseError
        })
    }

    /// Retrieves all transactions for an account/asset pair in chronological order (TRX-036).
    pub async fn get_transactions(
        &self,
        account_id: &str,
        asset_id: &str,
    ) -> StdResult<Vec<Transaction>, AccountError> {
        self.transaction_repo
            .get_by_account_asset(account_id, asset_id)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, account_id = %account_id, asset_id = %asset_id, err = ?e, "get_transactions: repository failure");
                AccountError::DatabaseError
            })
    }

    /// TDI-010 — Computes the (account, asset) holding's quantity and VWAP
    /// average cost as of `date` by replaying the pair's transactions dated on or
    /// before it. Unknown account/asset yields an empty snapshot (`0`/`0`); an
    /// unparseable `date` is rejected with `InvalidDate` (TDI-012).
    pub async fn holding_snapshot_as_of(
        &self,
        account_id: &str,
        asset_id: &str,
        date: &str,
    ) -> StdResult<HoldingSnapshot, AccountError> {
        NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| AccountError::InvalidDate)?;
        let transactions = self
            .transaction_repo
            .get_by_account_asset(account_id, asset_id)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, account_id = %account_id, asset_id = %asset_id, err = ?e, "holding_snapshot_as_of: repository failure");
                AccountError::DatabaseError
            })?;
        Ok(Account::holding_snapshot_as_of(
            &transactions,
            asset_id,
            date,
        ))
    }

    /// Retrieves every transaction for an account across all assets, ordered
    /// chronologically by `(date, created_at)` (PRF-021). Used by the
    /// account-performance use case to replay holdings and cash as of any past date.
    pub async fn get_all_transactions_for_account(
        &self,
        account_id: &str,
    ) -> StdResult<Vec<Transaction>, AccountError> {
        self.transaction_repo
            .get_all_for_account(account_id)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, account_id = %account_id, err = ?e, "get_all_transactions_for_account: repository failure");
                AccountError::DatabaseError
            })
    }

    /// Returns distinct asset IDs that have transactions for the given account (TXL-013).
    pub async fn get_asset_ids_for_account(
        &self,
        account_id: &str,
    ) -> StdResult<Vec<String>, AccountError> {
        self.transaction_repo
            .get_asset_ids_for_account(account_id)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, account_id = %account_id, err = ?e, "get_asset_ids_for_account: repository failure");
                AccountError::DatabaseError
            })
    }

    // -------------------------------------------------------------------------
    // Aggregate operations (B21 — thin orchestrators)
    // -------------------------------------------------------------------------

    /// Records a purchase of an asset into the account (TRX-020, TRX-026).
    ///
    /// Loads the Account aggregate, delegates to `Account::buy_holding`, saves
    /// atomically. Returns a typed `AccountError` — same composite as
    /// the cash methods, since cash deposit/withdrawal and asset buy/sell are
    /// all kinds of holding transaction.
    #[allow(clippy::too_many_arguments)]
    pub async fn buy_holding(
        &self,
        account_id: &str,
        asset_id: String,
        date: String,
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        fees: i64,
        note: Option<String>,
    ) -> Result<Transaction, AccountError> {
        info!(target: BACKEND, account_id = %account_id, asset_id = %asset_id, "buy_holding");
        let mut account = load_account(&*self.account_repo, account_id).await?;
        let tx = account
            .buy_holding(
                asset_id,
                date,
                quantity,
                unit_price,
                exchange_rate,
                fees,
                note,
            )
            .map_err(to_holding_tx_error)?
            .clone();
        save_account(&*self.account_repo, &mut account).await?;
        self.emit_transaction_updated();
        Ok(tx)
    }

    /// Records a sale of an asset from the account (SEL-012, SEL-021, SEL-023, SEL-024).
    ///
    /// Loads the Account aggregate, delegates to `Account::sell_holding`, saves atomically.
    #[allow(clippy::too_many_arguments)]
    pub async fn sell_holding(
        &self,
        account_id: &str,
        asset_id: String,
        date: String,
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        fees: i64,
        note: Option<String>,
    ) -> Result<Transaction, AccountError> {
        info!(target: BACKEND, account_id = %account_id, asset_id = %asset_id, "sell_holding");
        let mut account = load_account(&*self.account_repo, account_id).await?;
        let tx = account
            .sell_holding(
                asset_id,
                date,
                quantity,
                unit_price,
                exchange_rate,
                fees,
                note,
            )
            .map_err(to_holding_tx_error)?
            .clone();
        save_account(&*self.account_repo, &mut account).await?;
        self.emit_transaction_updated();
        Ok(tx)
    }

    /// Corrects an existing transaction and recalculates the affected holding (TRX-031, SEL-031).
    ///
    /// Loads the Account aggregate, delegates to `Account::correct_transaction`, saves atomically.
    #[allow(clippy::too_many_arguments)]
    pub async fn correct_transaction(
        &self,
        account_id: &str,
        tx_id: &str,
        date: String,
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        fees: i64,
        note: Option<String>,
    ) -> Result<Transaction, AccountError> {
        info!(target: BACKEND, account_id = %account_id, tx_id = %tx_id, "correct_transaction");
        let mut account = load_account(&*self.account_repo, account_id).await?;
        let tx = account
            .correct_transaction(tx_id, date, quantity, unit_price, exchange_rate, fees, note)
            .map_err(to_holding_tx_error)?
            .clone();
        save_account(&*self.account_repo, &mut account).await?;
        self.emit_transaction_updated();
        Ok(tx)
    }

    /// Deletes a transaction and recalculates (or removes) the associated holding (TRX-034).
    ///
    /// Loads the Account aggregate, delegates to `Account::cancel_transaction`, saves atomically.
    pub async fn cancel_transaction(
        &self,
        account_id: &str,
        tx_id: &str,
    ) -> Result<(), AccountError> {
        info!(target: BACKEND, account_id = %account_id, tx_id = %tx_id, "cancel_transaction");
        let mut account = load_account(&*self.account_repo, account_id).await?;
        account
            .cancel_transaction(tx_id)
            .map_err(to_holding_tx_error)?;
        save_account(&*self.account_repo, &mut account).await?;
        self.emit_transaction_updated();
        Ok(())
    }

    /// Seeds a holding directly from a quantity and total cost (TRX-042, TRX-047).
    ///
    /// Asset existence and archived-status checks are the caller's responsibility
    /// (handled by `HoldingTransactionUseCase::open_holding` — TRX-050, TRX-056).
    /// Returns the use-case-owned `OpenHoldingError`; the service-internal slice
    /// (load + aggregate + save) raises `Application(AccountNotFound | DatabaseError)`,
    /// `Validation(InvalidTotalCost)`, or `TxValidation(...)`. Cross-BC asset
    /// rejections never reach this method — the orchestrator raises them before
    /// delegating.
    pub async fn open_holding(
        &self,
        account_id: &str,
        asset_id: String,
        date: String,
        quantity: i64,
        total_cost: i64,
    ) -> Result<Transaction, OpenHoldingError> {
        info!(target: BACKEND, account_id = %account_id, asset_id = %asset_id, "open_holding");
        let mut account = load_account_for_open_holding(&*self.account_repo, account_id).await?;
        let tx = account
            .open_holding(asset_id, date, quantity, total_cost)
            .map_err(to_open_holding_error)?
            .clone();
        save_account_for_open_holding(&*self.account_repo, &mut account).await?;
        self.emit_transaction_updated();
        Ok(tx)
    }

    /// Records a Deposit (CSH-022) — cash inflow into the account.
    ///
    /// Application-layer composition: loads the Account, builds the Transaction
    /// via `Transaction::new_deposit` (TRX-020 enforced by the factory), applies
    /// it via `Account::apply_deposit` (CSH-080 enforced by the aggregate),
    /// then saves atomically. Returns a typed `AccountError` — no
    /// `anyhow` at this boundary; the caller (orchestrator / api) propagates
    /// the typed enum directly.
    pub async fn record_deposit(
        &self,
        account_id: &str,
        date: String,
        amount: i64,
        note: Option<String>,
    ) -> Result<Transaction, AccountError> {
        info!(target: BACKEND, account_id = %account_id, amount = amount, "record_deposit");
        let mut account = load_account(&*self.account_repo, account_id).await?;
        let tx = Transaction::new_deposit(
            account.id.clone(),
            account.cash_asset_id(),
            date,
            amount,
            note,
        )?;
        let tx = account.apply_deposit(tx)?;
        save_account(&*self.account_repo, &mut account).await?;
        self.emit_transaction_updated();
        Ok(tx)
    }

    /// Records a Withdrawal (CSH-032) — cash outflow from the account.
    ///
    /// Application-layer composition mirroring `record_deposit`. Raises
    /// `InsufficientCash` (CSH-080) when no Cash Holding exists or its balance
    /// is below `amount` — the check lives inside `Account::apply_withdrawal`.
    pub async fn record_withdrawal(
        &self,
        account_id: &str,
        date: String,
        amount: i64,
        note: Option<String>,
    ) -> Result<Transaction, AccountError> {
        info!(target: BACKEND, account_id = %account_id, amount = amount, "record_withdrawal");
        let mut account = load_account(&*self.account_repo, account_id).await?;
        let tx = Transaction::new_withdrawal(
            account.id.clone(),
            account.cash_asset_id(),
            date,
            amount,
            note,
        )?;
        let tx = account.apply_withdrawal(tx)?;
        save_account(&*self.account_repo, &mut account).await?;
        self.emit_transaction_updated();
        Ok(tx)
    }

    // -------------------------------------------------------------------------
    // Cross-BC guard queries (called by use cases)
    // -------------------------------------------------------------------------

    /// Returns true if any account holds a non-zero quantity of the given asset.
    /// Used by the archive_asset use case to enforce OQ-6. Translates raw
    /// infra failure into `AccountError::DatabaseError` per the
    /// gold infra-translation rule.
    pub async fn has_active_holdings_for_asset(
        &self,
        asset_id: &str,
    ) -> StdResult<bool, AccountError> {
        self.holding_repo
            .has_active_holdings_for_asset(asset_id)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, asset_id = %asset_id, err = ?e, "has_active_holdings_for_asset: repository failure");
                AccountError::DatabaseError
            })
    }

    /// Returns true if any transaction references the given asset.
    /// Used by the delete_asset use case to block hard-deletion when history
    /// exists. Translates raw infra failure into
    /// `AccountError::DatabaseError` per the gold infra-translation rule.
    pub async fn has_holding_entries_for_asset(
        &self,
        asset_id: &str,
    ) -> StdResult<bool, AccountError> {
        self.transaction_repo
            .has_transactions_for_asset(asset_id)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, asset_id = %asset_id, err = ?e, "has_holding_entries_for_asset: repository failure");
                AccountError::DatabaseError
            })
    }

    /// Returns the count of active holdings and total transactions for an account (ACC-020).
    pub async fn get_deletion_summary(
        &self,
        account_id: &str,
    ) -> StdResult<(u32, u32), AccountError> {
        tokio::try_join!(
            self.holding_repo.count_active_for_account(account_id),
            self.transaction_repo.count_by_account(account_id),
        )
        .map_err(|e| {
            tracing::error!(target: BACKEND, account_id = %account_id, err = ?e, "get_deletion_summary: repository failure");
            AccountError::DatabaseError
        })
    }

    /// Records a cash Dividend attributed to a held paying asset (DIV-023).
    ///
    /// Application-layer composition: loads the Account, builds the Transaction
    /// via `Transaction::new_dividend` (DIV-021/022 enforced by the factory),
    /// applies it via `Account::apply_dividend` (credit-only; no InsufficientCash),
    /// then saves atomically. Returns a typed `AccountError`.
    pub async fn record_dividend(
        &self,
        account_id: &str,
        paying_asset_id: String,
        date: String,
        amount_micros: i64,
        exchange_rate: i64,
        note: Option<String>,
    ) -> Result<Transaction, AccountError> {
        info!(target: BACKEND, account_id = %account_id, asset_id = %paying_asset_id, amount = amount_micros, "record_dividend");
        let mut account = load_account(&*self.account_repo, account_id).await?;
        let tx = Transaction::new_dividend(
            account.id.clone(),
            paying_asset_id,
            date,
            amount_micros,
            exchange_rate,
            note,
        )?;
        let tx = account.apply_dividend(tx)?;
        save_account(&*self.account_repo, &mut account).await?;
        self.emit_transaction_updated();
        Ok(tx)
    }

    /// Records a FreeShares distribution attributed to a held distributing asset
    /// (FSD-022).
    ///
    /// Application-layer composition: loads the Account, builds the Transaction
    /// via `Transaction::free_shares` (FSD-021 enforced by the factory), applies
    /// it via `Account::apply_free_shares` (holding quantity rises at zero cost,
    /// no cash leg), then saves atomically. Returns a typed
    /// `AccountError`.
    pub async fn record_free_shares(
        &self,
        account_id: &str,
        asset_id: String,
        date: String,
        quantity: i64,
        note: Option<String>,
    ) -> Result<Transaction, AccountError> {
        info!(target: BACKEND, account_id = %account_id, asset_id = %asset_id, quantity = quantity, "record_free_shares");
        let mut account = load_account(&*self.account_repo, account_id).await?;
        let tx = Transaction::free_shares(account.id.clone(), asset_id, date, quantity, note)?;
        let tx = account.apply_free_shares(tx).map_err(to_holding_tx_error)?;
        save_account(&*self.account_repo, &mut account).await?;
        self.emit_transaction_updated();
        Ok(tx)
    }

    // -------------------------------------------------------------------------
    // FEE-012/021/022/023/027 — management fee recording
    // -------------------------------------------------------------------------

    /// Records a one-off management fee deduction on a held asset (FEE-012).
    ///
    /// `percent_micros` is the fee in micro-percent (1% = 1_000_000). The number
    /// of shares removed is `floor(holding_qty_as_of(date) × percent_micros /
    /// 100_000_000)`. Cost basis is unchanged (VWAP concentrates — FEE-023).
    /// No cash leg. Raises `CascadingOversell` if a chronological replay of
    /// subsequent transactions would drive the holding negative (FEE-027).
    pub async fn record_management_fee(
        &self,
        account_id: &str,
        asset_id: String,
        date: String,
        percent_micros: i64,
        note: Option<String>,
    ) -> Result<Transaction, AccountError> {
        info!(target: BACKEND, account_id = %account_id, asset_id = %asset_id, percent_micros = percent_micros, "record_management_fee");
        // FEE-021 — percentage is micro-percent: strictly positive, at most 100%.
        if percent_micros <= 0 {
            return Err(AccountError::PercentageNotPositive);
        }
        if percent_micros > 100_000_000 {
            return Err(AccountError::PercentageAboveHundred);
        }
        let mut account = load_account(&*self.account_repo, account_id).await?;
        // FEE-077 — the % fee mechanism must be enabled on the account.
        account.ensure_management_fees_enabled()?;
        // FEE-022a — removed qty = floor(holding_qty_as_of(date) × percent / 100%).
        let quantity_as_of = account.holding_quantity_as_of(&asset_id, &date);
        let removed = (quantity_as_of as i128 * percent_micros as i128 / 100_000_000) as i64;
        let tx = Transaction::management_fee(account.id.clone(), asset_id, date, removed, note)?;
        let tx = account
            .apply_management_fee(tx)
            .map_err(to_holding_tx_error)?;
        save_account(&*self.account_repo, &mut account).await?;
        self.emit_transaction_updated();
        Ok(tx)
    }

    // -------------------------------------------------------------------------
    // INT-021/022/023/024/025 — interest recording
    // -------------------------------------------------------------------------

    /// Records an Interest credit on a held asset or the account's cash line
    /// (INT-021/022/023/024).
    ///
    /// Exactly one of `percent_micros` / `quantity_micros` must be provided
    /// (INT-021). Percent mode: the credited quantity is
    /// `floor(holding_qty_as_of(date) × percent_micros / 100_000_000)` (INT-022);
    /// a computed credit of 0 (empty holding or rate too small) is rejected as
    /// `QuantityNotPositive` by the `Transaction::interest` factory. Cost basis
    /// is unchanged (VWAP dilutes — INT-024); on the Cash Asset the credit goes
    /// through the cash replay (INT-023). Not gated by the account's
    /// `management_fees_enabled` parameter (INT-050).
    pub async fn record_interest(
        &self,
        account_id: &str,
        asset_id: String,
        date: String,
        percent_micros: Option<i64>,
        quantity_micros: Option<i64>,
        note: Option<String>,
    ) -> Result<Transaction, AccountError> {
        info!(target: BACKEND, account_id = %account_id, asset_id = %asset_id, percent_micros = percent_micros, quantity_micros = quantity_micros, "record_interest");
        // INT-021 — exactly one of percent / quantity; bounds per mode.
        match (percent_micros, quantity_micros) {
            (Some(percent_micros), None) => {
                if percent_micros <= 0 {
                    return Err(AccountError::PercentageNotPositive);
                }
                if percent_micros > 100_000_000 {
                    return Err(AccountError::PercentageAboveHundred);
                }
            }
            (None, Some(quantity_micros)) => {
                if quantity_micros <= 0 {
                    return Err(AccountError::QuantityNotPositive);
                }
            }
            _ => return Err(AccountError::InterestAmountInvalid),
        }
        let mut account = load_account(&*self.account_repo, account_id).await?;
        let credited = match (percent_micros, quantity_micros) {
            // INT-022 — credited qty = floor(holding_qty_as_of(date) × percent / 100%).
            // The cash line's balance moves with Purchase/Sell/Dividend transactions
            // whose asset_id is NOT the cash asset, so its percent base comes from the
            // dedicated cash replay, not the per-asset holding replay (INT-023).
            (Some(percent_micros), None) => {
                let quantity_as_of = if crate::core::cash::is_cash_asset(&asset_id) {
                    Account::cash_balance_as_of(&account.transactions, &date)
                } else {
                    account.holding_quantity_as_of(&asset_id, &date)
                };
                (quantity_as_of as i128 * percent_micros as i128 / 100_000_000) as i64
            }
            (None, Some(quantity_micros)) => quantity_micros,
            // Unreachable — both/neither already rejected above (INT-021).
            _ => return Err(AccountError::InterestAmountInvalid),
        };
        let tx = Transaction::interest(account.id.clone(), asset_id, date, credited, note)?;
        let tx = account.apply_interest(tx).map_err(to_holding_tx_error)?;
        save_account(&*self.account_repo, &mut account).await?;
        self.emit_transaction_updated();
        Ok(tx)
    }

    // -------------------------------------------------------------------------
    // FEE-030/031/032/033/034/060/061/062 — fee schedule CRUD
    // -------------------------------------------------------------------------

    /// Creates a new fee schedule for the (account, asset) pair (FEE-030).
    ///
    /// FEE-031 — rejects if a schedule already exists for the pair.
    /// FEE-032 — validates rate > 0 (`RateNotPositive`), rate ≤ 100% micro-percent
    /// (`RateAboveHundred`), end_date > start_date (`EndBeforeStart`).
    pub async fn create_fee_schedule(
        &self,
        account_id: &str,
        asset_id: String,
        annual_rate_percent_micros: i64,
        frequency: super::domain::FeeFrequency,
        start_date: String,
        end_date: Option<String>,
    ) -> Result<FeeSchedule, AccountError> {
        info!(target: BACKEND, account_id = %account_id, asset_id = %asset_id, "create_fee_schedule");
        // FEE-077 — the % fee mechanism must be enabled on the account.
        self.get_by_id(account_id)
            .await?
            .ok_or_else(|| AccountError::AccountNotFound {
                account_id: account_id.to_string(),
            })?
            .ensure_management_fees_enabled()?;
        let repo = self.fee_schedule_repo()?;
        // FEE-031 — at most one schedule per (account, asset).
        let existing = repo
            .get_by_account_asset(account_id, &asset_id)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, err = ?e, "create_fee_schedule: lookup failed");
                AccountError::DatabaseError
            })?;
        if existing.is_some() {
            return Err(AccountError::ScheduleAlreadyExists);
        }
        // FEE-032 — validates rate (>0, ≤100%) and end_date > start_date.
        let schedule = FeeSchedule::new(
            account_id.to_string(),
            asset_id,
            annual_rate_percent_micros,
            frequency,
            start_date,
            end_date,
        )?;
        repo.insert(&schedule).await.map_err(|e| {
            tracing::error!(target: BACKEND, err = ?e, "create_fee_schedule: insert failed");
            AccountError::DatabaseError
        })?;
        self.emit_fee_schedule_updated();
        Ok(schedule)
    }

    /// Updates an existing fee schedule (FEE-060/061).
    ///
    /// FEE-060 — rejects with `ScheduleNotFound` if no schedule exists.
    /// Editable fields: `annual_rate_percent_micros`, `end_date`, `active`.
    /// `frequency` and `start_date` are immutable after creation.
    pub async fn update_fee_schedule(
        &self,
        account_id: &str,
        asset_id: &str,
        annual_rate_percent_micros: i64,
        end_date: Option<String>,
        active: bool,
    ) -> Result<FeeSchedule, AccountError> {
        info!(target: BACKEND, account_id = %account_id, asset_id = %asset_id, "update_fee_schedule");
        let repo = self.fee_schedule_repo()?;
        let schedule = repo
            .get_by_account_asset(account_id, asset_id)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, err = ?e, "update_fee_schedule: lookup failed");
                AccountError::DatabaseError
            })?
            .ok_or(AccountError::ScheduleNotFound)?
            .update_from(annual_rate_percent_micros, end_date, active)?;
        repo.update(&schedule).await.map_err(|e| {
            tracing::error!(target: BACKEND, err = ?e, "update_fee_schedule: persist failed");
            AccountError::DatabaseError
        })?;
        self.emit_fee_schedule_updated();
        Ok(schedule)
    }

    /// Deletes the fee schedule for the (account, asset) pair (FEE-062).
    ///
    /// Silent no-op if no schedule exists (mirrors `delete_account` precedent).
    pub async fn delete_fee_schedule(
        &self,
        account_id: &str,
        asset_id: &str,
    ) -> Result<(), AccountError> {
        info!(target: BACKEND, account_id = %account_id, asset_id = %asset_id, "delete_fee_schedule");
        let repo = self.fee_schedule_repo()?;
        repo.delete_by_account_asset(account_id, asset_id)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, err = ?e, "delete_fee_schedule: delete failed");
                AccountError::DatabaseError
            })?;
        self.emit_fee_schedule_updated();
        Ok(())
    }

    /// Returns the fee schedule for the (account, asset) pair, or None (FEE-030).
    pub async fn get_fee_schedule(
        &self,
        account_id: &str,
        asset_id: &str,
    ) -> Result<Option<FeeSchedule>, AccountError> {
        let repo = self.fee_schedule_repo()?;
        repo.get_by_account_asset(account_id, asset_id)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, err = ?e, "get_fee_schedule: lookup failed");
                AccountError::DatabaseError
            })
    }

    /// Returns every active fee schedule across all accounts (FEE-040 catch-up).
    pub async fn list_active_fee_schedules(&self) -> Result<Vec<FeeSchedule>, AccountError> {
        let repo = self.fee_schedule_repo()?;
        repo.get_all_active().await.map_err(|e| {
            tracing::error!(target: BACKEND, err = ?e, "list_active_fee_schedules: query failed");
            AccountError::DatabaseError
        })
    }

    /// Returns the active fee schedules of one account (FEE-074).
    pub async fn list_active_fee_schedules_for_account(
        &self,
        account_id: &str,
    ) -> Result<Vec<FeeSchedule>, AccountError> {
        let repo = self.fee_schedule_repo()?;
        repo.get_active_by_account(account_id).await.map_err(|e| {
            tracing::error!(target: BACKEND, err = ?e, "list_active_fee_schedules_for_account: query failed");
            AccountError::DatabaseError
        })
    }

    /// Advances a schedule's catch-up cursor to `last_applied_period` (FEE-043).
    /// Silent no-op if the schedule no longer exists.
    pub async fn advance_fee_schedule_cursor(
        &self,
        account_id: &str,
        asset_id: &str,
        last_applied_period: String,
    ) -> Result<(), AccountError> {
        let repo = self.fee_schedule_repo()?;
        let existing = repo
            .get_by_account_asset(account_id, asset_id)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, err = ?e, "advance_fee_schedule_cursor: lookup failed");
                AccountError::DatabaseError
            })?;
        if let Some(schedule) = existing {
            let schedule = schedule.advance_cursor(last_applied_period);
            repo.update(&schedule).await.map_err(|e| {
                tracing::error!(target: BACKEND, err = ?e, "advance_fee_schedule_cursor: persist failed");
                AccountError::DatabaseError
            })?;
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    fn emit_account_updated(&self) {
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AccountUpdated);
        }
    }

    fn emit_transaction_updated(&self) {
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::TransactionUpdated);
        }
    }

    fn emit_fee_schedule_updated(&self) {
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::FeeScheduleUpdated);
        }
    }
}

/// Loads an Account aggregate (with holdings + transactions) for the
/// holding-transaction family. Translates repository failures into typed
/// `AccountError` — `AccountNotFound` for `Ok(None)`, `DatabaseError` for any
/// anyhow error (logged at the same site).
async fn load_account(
    repo: &dyn AccountRepository,
    account_id: &str,
) -> Result<Account, AccountError> {
    match repo.get_with_holdings_and_transactions(account_id).await {
        Ok(Some(acc)) => Ok(acc),
        Ok(None) => Err(AccountError::AccountNotFound {
            account_id: account_id.to_string(),
        }),
        Err(e) => {
            tracing::error!(target: BACKEND, account_id = %account_id, err = ?e, "load_account: repository failure");
            Err(AccountError::DatabaseError)
        }
    }
}

/// Persists an Account aggregate's pending changes for the holding-transaction
/// family. Translates repository failures into `AccountError::DatabaseError`
/// after logging the underlying error.
async fn save_account(
    repo: &dyn AccountRepository,
    account: &mut Account,
) -> Result<(), AccountError> {
    repo.save(account).await.map_err(|e| {
        tracing::error!(target: BACKEND, account_id = %account.id, err = ?e, "save_account: repository failure");
        AccountError::DatabaseError
    })
}

/// CRUD-family parallel to the load/save helpers above. Wraps the
/// `find_by_name` uniqueness pre-check used by `create` and `update`,
/// translating any repository failure into
/// `AccountError::DatabaseError`.
///
/// Unlike `load_account`, `Ok(None)` is the **success** path here (the name
/// is available); the caller decides what to do with `Some(existing)`.
async fn find_account_by_name(
    repo: &dyn AccountRepository,
    name: &str,
) -> Result<Option<Account>, AccountError> {
    repo.find_by_name(name).await.map_err(|e| {
        tracing::error!(target: BACKEND, name = %name, err = ?e, "find_by_name: repository failure");
        AccountError::DatabaseError
    })
}

/// Open-holding parallel to `load_account`. Same shape; targets `OpenHoldingError`.
async fn load_account_for_open_holding(
    repo: &dyn AccountRepository,
    account_id: &str,
) -> Result<Account, OpenHoldingError> {
    match repo.get_with_holdings_and_transactions(account_id).await {
        Ok(Some(acc)) => Ok(acc),
        Ok(None) => Err(AccountError::AccountNotFound {
            account_id: account_id.to_string(),
        }
        .into()),
        Err(e) => {
            tracing::error!(target: BACKEND, account_id = %account_id, err = ?e, "load_account_for_open_holding: repository failure");
            Err(AccountError::DatabaseError.into())
        }
    }
}

/// Open-holding parallel to `save_account`. Same shape; targets `OpenHoldingError`.
async fn save_account_for_open_holding(
    repo: &dyn AccountRepository,
    account: &mut Account,
) -> Result<(), OpenHoldingError> {
    repo.save(account).await.map_err(|e| {
        tracing::error!(target: BACKEND, account_id = %account.id, err = ?e, "save_account_for_open_holding: repository failure");
        AccountError::DatabaseError.into()
    })
}

/// Converts the `anyhow::Error` returned by the buy/sell/correct/cancel
/// aggregate methods into the typed `AccountError` it boxes. Errors that don't
/// downcast to `AccountError` are logged and surfaced as
/// `AccountError::DatabaseError` — the same translation target as the
/// load/save helpers above.
fn to_holding_tx_error(e: anyhow::Error) -> AccountError {
    match e.downcast::<AccountError>() {
        Ok(err) => err,
        Err(e) => {
            tracing::error!(target: BACKEND, err = ?e, "unexpected error in holding-tx service method");
            AccountError::DatabaseError
        }
    }
}

/// Bridge for the open_holding aggregate method, which still returns
/// `anyhow::Result` boxing an `AccountError` (`InvalidTotalCost` or any variant
/// reachable from `Transaction::new`). Same shape as `to_holding_tx_error`;
/// targets `OpenHoldingError` instead.
fn to_open_holding_error(e: anyhow::Error) -> OpenHoldingError {
    match e.downcast::<AccountError>() {
        Ok(err) => err.into(),
        Err(e) => {
            tracing::error!(target: BACKEND, err = ?e, "unexpected error in open_holding service method");
            AccountError::DatabaseError.into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // This module contains both SQLite-backed integration tests (real in-memory DB,
    // catch constraint violations) and mock-based unit tests (fast delegation checks).
    // SQLite tests are grouped first; mock-based unit tests follow after the section header.
    use crate::context::account::{
        AccountError, Holding, MockAccountRepository, MockHoldingRepository,
        MockTransactionRepository, SqliteAccountRepository, SqliteFeeScheduleRepository,
        SqliteHoldingRepository, SqliteTransactionRepository,
    };
    use sqlx::sqlite::SqlitePoolOptions;

    #[derive(Debug, thiserror::Error)]
    #[error("simulated DB failure")]
    struct SimulatedSaveError;

    // to_holding_tx_error is the anyhow→typed bridge for the four
    // holding-tx aggregate methods (buy/sell/correct/cancel) that still return
    // `anyhow::Result`. One global test covers the three branches: known
    // domain leaves route to their typed variant; everything else translates
    // to Application(DatabaseError).
    #[test]
    fn to_holding_tx_error_maps_every_branch() {
        // AccountError leaf → Operation
        let op_err = AccountError::Oversell {
            available: 10,
            requested: 99,
        };
        assert!(matches!(
            to_holding_tx_error(anyhow::Error::new(op_err)),
            AccountError::Oversell {
                available: 10,
                requested: 99
            }
        ));

        // AccountError leaf → Validation
        assert!(matches!(
            to_holding_tx_error(anyhow::Error::new(AccountError::DateInFuture)),
            AccountError::DateInFuture
        ));

        // Anything else → Application(DatabaseError) (the catch-all path)
        assert!(matches!(
            to_holding_tx_error(anyhow::anyhow!("synthetic infra failure")),
            AccountError::DatabaseError
        ));
    }

    // to_open_holding_error is the anyhow→typed bridge for `Account::open_holding`
    // (which still returns `anyhow::Result`). One global test covers the three
    // branches: known domain leaves route to their typed variants; unrecognized
    // errors translate to Application(DatabaseError).
    #[test]
    fn to_open_holding_error_maps_every_branch() {
        use crate::context::account::AccountError;
        use OpenHoldingError;

        // AccountError leaf → Validation
        assert!(matches!(
            to_open_holding_error(anyhow::Error::new(AccountError::InvalidTotalCost)),
            OpenHoldingError::Account(AccountError::InvalidTotalCost)
        ));

        // AccountError leaf → TxValidation
        assert!(matches!(
            to_open_holding_error(anyhow::Error::new(AccountError::QuantityNotPositive)),
            OpenHoldingError::Account(AccountError::QuantityNotPositive)
        ));

        // Anything else → Application(DatabaseError) (the catch-all path)
        assert!(matches!(
            to_open_holding_error(anyhow::anyhow!("synthetic infra failure")),
            OpenHoldingError::Account(AccountError::DatabaseError)
        ));
    }

    async fn setup(pool: &sqlx::Pool<sqlx::Sqlite>) -> (AccountService, String) {
        let svc = AccountService::new(
            Box::new(SqliteAccountRepository::new(pool.clone())),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
            Box::new(SqliteTransactionRepository::new(pool.clone())),
        )
        .with_fee_schedule_repo(Box::new(SqliteFeeScheduleRepository::new(pool.clone())));
        let asset_id = "test-asset-id".to_string();
        sqlx::query(
            "INSERT INTO assets (id, name, reference, asset_class, category_id, currency, risk_level)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&asset_id)
        .bind("TestAsset")
        .bind("TST")
        .bind("Stocks")
        .bind("default-uncategorized")
        .bind("USD")
        .bind(1_i64)
        .execute(pool)
        .await
        .expect("seed asset row");
        (svc, asset_id)
    }

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

    async fn setup_service() -> AccountService {
        let pool = make_pool().await;
        AccountService::new(
            Box::new(SqliteAccountRepository::new(pool.clone())),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
            Box::new(SqliteTransactionRepository::new(pool.clone())),
        )
    }

    async fn enable_management_fees(svc: &AccountService, account: &Account) -> Account {
        svc.update(
            account.id.clone(),
            account.name.clone(),
            String::new(),
            account.currency.clone(),
            account.update_frequency,
            true,
        )
        .await
        .unwrap()
    }

    // FEE-077 — record_management_fee rejects when the account has the mechanism disabled
    #[tokio::test]
    async fn fee_077_record_management_fee_rejects_disabled_account() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Gated".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let err = svc
            .record_management_fee(
                &account.id,
                asset_id,
                "2026-01-15".to_string(),
                1_000_000,
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::ManagementFeesDisabled),
            "got: {err:?}"
        );
    }

    // FEE-077 — create_fee_schedule rejects when the account has the mechanism disabled
    #[tokio::test]
    async fn fee_077_create_fee_schedule_rejects_disabled_account() {
        use crate::context::account::FeeFrequency;
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Gated".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let err = svc
            .create_fee_schedule(
                &account.id,
                asset_id,
                1_000_000,
                FeeFrequency::Monthly,
                "2026-01-01".to_string(),
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::ManagementFeesDisabled),
            "got: {err:?}"
        );
    }

    // FEE-074 — the scoped query returns only the given account's active
    // schedules: another account's active schedule and the account's own
    // inactive schedule are both excluded.
    #[tokio::test]
    async fn list_active_fee_schedules_for_account_excludes_other_accounts_and_inactive() {
        use crate::context::account::FeeFrequency;
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let second_asset_id = "test-asset-id-2".to_string();
        sqlx::query(
            "INSERT INTO assets (id, name, reference, asset_class, category_id, currency, risk_level)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&second_asset_id)
        .bind("TestAsset2")
        .bind("TST2")
        .bind("Stocks")
        .bind("default-uncategorized")
        .bind("USD")
        .bind(1_i64)
        .execute(&pool)
        .await
        .expect("seed second asset row");

        let target = svc
            .create(
                "Target".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let target = enable_management_fees(&svc, &target).await;
        let other = svc
            .create(
                "Other".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let other = enable_management_fees(&svc, &other).await;

        svc.create_fee_schedule(
            &target.id,
            asset_id.clone(),
            1_000_000,
            FeeFrequency::Monthly,
            "2026-01-01".to_string(),
            None,
        )
        .await
        .unwrap();
        svc.create_fee_schedule(
            &target.id,
            second_asset_id.clone(),
            2_000_000,
            FeeFrequency::Monthly,
            "2026-01-01".to_string(),
            None,
        )
        .await
        .unwrap();
        svc.update_fee_schedule(&target.id, &second_asset_id, 2_000_000, None, false)
            .await
            .unwrap();
        svc.create_fee_schedule(
            &other.id,
            asset_id.clone(),
            3_000_000,
            FeeFrequency::Monthly,
            "2026-01-01".to_string(),
            None,
        )
        .await
        .unwrap();

        let schedules = svc
            .list_active_fee_schedules_for_account(&target.id)
            .await
            .unwrap();
        assert_eq!(
            schedules.len(),
            1,
            "only the target account's active schedule must return, got: {schedules:?}"
        );
        assert_eq!(schedules[0].account_id, target.id);
        assert_eq!(schedules[0].asset_id, asset_id);
        assert!(schedules[0].active);
    }

    /// Seeds the system Cash Asset row + a large Deposit so existing buy/sell tests can
    /// satisfy CSH-041 (purchase eligibility). Bypasses `AssetService` because these tests
    /// only construct an `AccountService`.
    async fn seed_cash_for_account(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        svc: &AccountService,
        account_id: &str,
        currency: &str,
    ) {
        let cash_asset_id = format!("system-cash-{}", currency.to_lowercase());
        sqlx::query(
            "INSERT OR IGNORE INTO categories (id, name, is_deleted) VALUES ('system-cash-category', 'cash', 0)",
        )
        .execute(pool)
        .await
        .expect("seed cash category");
        sqlx::query(
            "INSERT OR IGNORE INTO assets (id, name, reference, asset_class, category_id, currency, risk_level) \
             VALUES (?, ?, ?, 'Cash', 'system-cash-category', ?, 1)",
        )
        .bind(&cash_asset_id)
        .bind(format!("Cash {}", currency.to_uppercase()))
        .bind(currency.to_uppercase())
        .bind(currency)
        .execute(pool)
        .await
        .expect("seed cash asset");
        svc.record_deposit(
            account_id,
            "2020-01-01".to_string(),
            1_000_000_000_000,
            None,
        )
        .await
        .expect("seed cash deposit");
    }

    // R3 — duplicate name (case-insensitive) is rejected at creation
    #[tokio::test]
    async fn test_create_rejects_duplicate_name_case_insensitive() {
        let svc = setup_service().await;
        svc.create(
            "Alpha".to_string(),
            String::new(),
            "EUR".to_string(),
            UpdateFrequency::ManualMonth,
            false,
        )
        .await
        .unwrap();
        let err = svc
            .create(
                "alpha".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::NameAlreadyExists),
            "got: {err:?}"
        );
    }

    // R3 — updating an account to a name used by another account is rejected
    #[tokio::test]
    async fn test_update_rejects_name_collision_with_other_account() {
        let svc = setup_service().await;
        svc.create(
            "Alpha".to_string(),
            String::new(),
            "EUR".to_string(),
            UpdateFrequency::ManualMonth,
            false,
        )
        .await
        .unwrap();
        let beta = svc
            .create(
                "Beta".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let err = svc
            .update(
                beta.id,
                "ALPHA".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::NameAlreadyExists),
            "got: {err:?}"
        );
    }

    // R3 — updating an account with its own name (same case) must succeed
    #[tokio::test]
    async fn test_update_allows_same_name_on_same_account() {
        let svc = setup_service().await;
        let account = svc
            .create(
                "Alpha".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let result = svc
            .update(
                account.id,
                "Alpha".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualDay,
                false,
            )
            .await;
        assert!(result.is_ok());
    }

    fn micro(v: i64) -> i64 {
        v * 1_000_000
    }

    // TRX-026 / TRX-030 — buy_holding persists transaction and updates holding VWAP
    #[tokio::test]
    async fn test_buy_holding_persists_transaction_and_holding() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Acc".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;
        let tx = svc
            .buy_holding(
                &account.id,
                asset_id.clone(),
                "2024-01-01".to_string(),
                micro(2),
                micro(100),
                micro(1),
                0,
                None,
            )
            .await
            .unwrap();
        assert_eq!(tx.account_id, account.id);
        assert_eq!(tx.asset_id, asset_id);
        let holdings = svc.get_holdings_for_account(&account.id).await.unwrap();
        let asset_holding = holdings
            .iter()
            .find(|h| h.asset_id == asset_id)
            .expect("asset holding present");
        assert_eq!(asset_holding.quantity, micro(2));
        assert_eq!(asset_holding.average_price, micro(100));
    }

    // SEL-021 — sell_holding rejects oversell via AccountError
    #[tokio::test]
    async fn test_sell_holding_rejects_oversell() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Acc".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;
        svc.buy_holding(
            &account.id,
            asset_id.clone(),
            "2024-01-01".to_string(),
            micro(1),
            micro(100),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();
        let err = svc
            .sell_holding(
                &account.id,
                asset_id,
                "2024-06-01".to_string(),
                micro(2),
                micro(100),
                micro(1),
                0,
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::Oversell { .. }),
            "expected Oversell, got: {err:?}"
        );
    }

    // TRX-034 — cancel_transaction removes the holding when it was the last transaction
    #[tokio::test]
    async fn test_cancel_transaction_removes_holding_when_last() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Acc".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;
        let tx = svc
            .buy_holding(
                &account.id,
                asset_id.clone(),
                "2024-01-01".to_string(),
                micro(1),
                micro(100),
                micro(1),
                0,
                None,
            )
            .await
            .unwrap();
        svc.cancel_transaction(&account.id, &tx.id).await.unwrap();
        let holdings = svc.get_holdings_for_account(&account.id).await.unwrap();
        assert!(
            holdings.iter().all(|h| h.asset_id != asset_id),
            "asset holding should be removed after cancel"
        );
        let txs = svc.get_transactions(&account.id, &asset_id).await.unwrap();
        assert!(
            txs.is_empty(),
            "transactions for the asset should be removed after cancel"
        );
    }

    // SEL-026 — full sell retains holding at quantity=0 with VWAP preserved
    #[tokio::test]
    async fn test_full_sell_retains_holding_at_zero_with_last_vwap() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Acc".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;
        svc.buy_holding(
            &account.id,
            asset_id.clone(),
            "2024-01-01".to_string(),
            micro(2),
            micro(100),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();
        svc.sell_holding(
            &account.id,
            asset_id.clone(),
            "2024-06-01".to_string(),
            micro(2),
            micro(120),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();
        let holdings = svc.get_holdings_for_account(&account.id).await.unwrap();
        let h = holdings
            .iter()
            .find(|h| h.asset_id == asset_id)
            .expect("holding should exist after full sell");
        assert_eq!(h.quantity, 0, "holding should be retained at qty=0");
        assert_eq!(h.average_price, micro(100), "VWAP should be preserved");
    }

    // SEL-032 — correcting a purchase to a lower qty that would cause a cascading oversell is rejected
    #[tokio::test]
    async fn test_correct_purchase_rejected_when_causes_cascading_oversell() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Acc".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;
        let buy = svc
            .buy_holding(
                &account.id,
                asset_id.clone(),
                "2024-01-01".to_string(),
                micro(3),
                micro(100),
                micro(1),
                0,
                None,
            )
            .await
            .unwrap();
        svc.sell_holding(
            &account.id,
            asset_id.clone(),
            "2024-06-01".to_string(),
            micro(2),
            micro(120),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();
        let err = svc
            .correct_transaction(
                &account.id,
                &buy.id,
                "2024-01-01".to_string(),
                micro(1),
                micro(100),
                micro(1),
                0,
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::CascadingOversell),
            "expected CascadingOversell, got: {err:?}"
        );
    }

    // TRX-027 — buy_holding propagates save failure so no partial state is committed
    #[tokio::test]
    async fn test_buy_holding_returns_error_when_save_fails() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_get_with_holdings_and_transactions()
            .once()
            .returning(|_| {
                let mut acc = Account::new(
                    "Test".to_string(),
                    String::new(),
                    "EUR".to_string(),
                    UpdateFrequency::ManualMonth,
                    false,
                )
                .unwrap();
                // Seed enough cash so CSH-041 doesn't short-circuit before save() is called.
                acc.record_deposit("2020-01-01".to_string(), 1_000_000_000_000, None)
                    .unwrap();
                acc.pending_changes.clear();
                Ok(Some(acc))
            });
        mock_ar
            .expect_save()
            .once()
            .returning(|_| Err(SimulatedSaveError.into()));

        let svc = AccountService::new(
            Box::new(mock_ar),
            Box::new(MockHoldingRepository::new()),
            Box::new(MockTransactionRepository::new()),
        );

        let result = svc
            .buy_holding(
                "any-account-id",
                "asset-1".to_string(),
                "2024-01-01".to_string(),
                micro(1),
                micro(100),
                micro(1),
                0,
                None,
            )
            .await;

        let err = result.unwrap_err();
        // The repo save error is opaqued at the service boundary — translated
        // to AccountError::DatabaseError; the hint is preserved
        // server-side via tracing::error! at the same site.
        assert!(
            matches!(err, AccountError::DatabaseError),
            "buy_holding must surface save failures as Application(DatabaseError), got: {err:?}"
        );
    }

    // TDI-012 — an unparseable date is rejected before any repository call.
    #[tokio::test]
    async fn holding_snapshot_as_of_rejects_an_unparseable_date() {
        let svc = AccountService::new(
            Box::new(MockAccountRepository::new()),
            Box::new(MockHoldingRepository::new()),
            Box::new(MockTransactionRepository::new()),
        );
        let err = svc
            .holding_snapshot_as_of("acc-1", "asset-1", "not-a-date")
            .await
            .unwrap_err();
        assert!(matches!(err, AccountError::InvalidDate));
    }

    // TDI-010 — a valid date over an empty history yields an empty snapshot.
    #[tokio::test]
    async fn holding_snapshot_as_of_returns_empty_for_an_account_with_no_transactions() {
        let mut mock_tr = MockTransactionRepository::new();
        mock_tr
            .expect_get_by_account_asset()
            .once()
            .returning(|_, _| Ok(vec![]));
        let svc = AccountService::new(
            Box::new(MockAccountRepository::new()),
            Box::new(MockHoldingRepository::new()),
            Box::new(mock_tr),
        );
        let snap = svc
            .holding_snapshot_as_of("acc-1", "asset-1", "2024-07-01")
            .await
            .expect("a valid date with no transactions yields an empty snapshot");
        assert_eq!(snap.quantity, 0);
        assert_eq!(snap.average_price, 0);
    }

    // A repository failure on the transaction fetch surfaces as DatabaseError.
    #[tokio::test]
    async fn holding_snapshot_as_of_surfaces_repository_failure_as_database_error() {
        let mut mock_tr = MockTransactionRepository::new();
        mock_tr
            .expect_get_by_account_asset()
            .once()
            .returning(|_, _| Err(anyhow::anyhow!("db down")));
        let svc = AccountService::new(
            Box::new(MockAccountRepository::new()),
            Box::new(MockHoldingRepository::new()),
            Box::new(mock_tr),
        );
        let err = svc
            .holding_snapshot_as_of("acc-1", "asset-1", "2024-07-01")
            .await
            .unwrap_err();
        assert!(matches!(err, AccountError::DatabaseError));
    }

    // -------------------------------------------------------------------------
    // open_holding service tests (TRX-042 through TRX-056)
    // -------------------------------------------------------------------------

    // open_holding propagates save failure as Application(DatabaseError) — mirrors
    // test_buy_holding_returns_error_when_save_fails for the typed-Result path
    // (save_account_for_open_holding Err branch).
    #[tokio::test]
    async fn test_open_holding_returns_database_error_when_save_fails() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_get_with_holdings_and_transactions()
            .once()
            .returning(|_| {
                let acc = Account::new(
                    "Test".to_string(),
                    String::new(),
                    "EUR".to_string(),
                    UpdateFrequency::ManualMonth,
                    false,
                )
                .unwrap();
                Ok(Some(acc))
            });
        mock_ar
            .expect_save()
            .once()
            .returning(|_| Err(SimulatedSaveError.into()));

        let svc = AccountService::new(
            Box::new(mock_ar),
            Box::new(MockHoldingRepository::new()),
            Box::new(MockTransactionRepository::new()),
        );

        let result = svc
            .open_holding(
                "any-account-id",
                "asset-1".to_string(),
                "2024-01-01".to_string(),
                micro(1),
                micro(100),
            )
            .await;

        let err = result.unwrap_err();
        use crate::use_cases::holding_transaction::OpenHoldingError;
        assert!(
            matches!(err, OpenHoldingError::Account(AccountError::DatabaseError)),
            "open_holding must surface save failures as Application(DatabaseError), got: {err:?}"
        );
    }

    // open_holding propagates repo load failure as Application(DatabaseError).
    // Distinct from test_open_holding_returns_account_not_found (which exercises
    // Ok(None) → Application(AccountNotFound)). This covers the Err branch of
    // load_account_for_open_holding.
    #[tokio::test]
    async fn test_open_holding_returns_database_error_when_load_fails() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_get_with_holdings_and_transactions()
            .once()
            .returning(|_| Err(SimulatedSaveError.into()));

        let svc = AccountService::new(
            Box::new(mock_ar),
            Box::new(MockHoldingRepository::new()),
            Box::new(MockTransactionRepository::new()),
        );

        let result = svc
            .open_holding(
                "any-account-id",
                "asset-1".to_string(),
                "2024-01-01".to_string(),
                micro(1),
                micro(100),
            )
            .await;

        let err = result.unwrap_err();
        use crate::use_cases::holding_transaction::OpenHoldingError;
        assert!(
            matches!(err, OpenHoldingError::Account(AccountError::DatabaseError)),
            "open_holding must surface load failures as Application(DatabaseError), got: {err:?}"
        );
    }

    // TRX-056 — open_holding returns AccountNotFound when account does not exist
    #[tokio::test]
    async fn test_open_holding_returns_account_not_found() {
        let svc = setup_service().await;
        let err = svc
            .open_holding(
                "nonexistent-account-id",
                "some-asset-id".to_string(),
                "2024-01-01".to_string(),
                micro(1),
                micro(100),
            )
            .await
            .unwrap_err();
        use OpenHoldingError;
        assert!(
            matches!(
                err,
                OpenHoldingError::Account(AccountError::AccountNotFound { .. })
            ),
            "expected Application(AccountNotFound), got: {err:?}"
        );
    }

    // TRX-044 — open_holding propagates QuantityNotPositive through the service
    #[tokio::test]
    async fn test_open_holding_propagates_quantity_not_positive() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Acc".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();

        let err = svc
            .open_holding(
                &account.id,
                asset_id,
                "2024-01-01".to_string(),
                0, // quantity ≤ 0
                micro(100),
            )
            .await
            .unwrap_err();

        use crate::context::account::AccountError;
        use OpenHoldingError;
        assert!(
            matches!(
                err,
                OpenHoldingError::Account(AccountError::QuantityNotPositive)
            ),
            "expected TxValidation(QuantityNotPositive), got: {err:?}"
        );
    }

    // TRX-045 — open_holding propagates InvalidTotalCost through the service
    #[tokio::test]
    async fn test_open_holding_propagates_invalid_total_cost() {
        use crate::context::account::AccountError;
        use OpenHoldingError;
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Acc".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();

        let err = svc
            .open_holding(
                &account.id,
                asset_id,
                "2024-01-01".to_string(),
                micro(1),
                -1, // negative total_cost (zero is now valid, TRX-045)
            )
            .await
            .unwrap_err();

        assert!(
            matches!(
                err,
                OpenHoldingError::Account(AccountError::InvalidTotalCost)
            ),
            "expected Validation(InvalidTotalCost), got: {err:?}"
        );
    }

    // TRX-047 — open_holding persists transaction and holding with correct fields
    #[tokio::test]
    async fn test_open_holding_persists_transaction_and_holding() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Acc".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();

        let tx = svc
            .open_holding(
                &account.id,
                asset_id.clone(),
                "2024-01-01".to_string(),
                micro(2),
                micro(200),
            )
            .await
            .unwrap();

        use crate::context::account::TransactionType;
        assert_eq!(tx.transaction_type, TransactionType::OpeningBalance);
        assert_eq!(tx.total_amount, micro(200), "total_amount = total_cost");
        assert_eq!(tx.fees, 0, "fees = 0");
        assert_eq!(tx.exchange_rate, 1_000_000, "exchange_rate = 1.0");
        // unit_price = floor(200_000_000 * 1_000_000 / 2_000_000) = 100_000_000
        assert_eq!(
            tx.unit_price,
            micro(100),
            "unit_price = total_cost / quantity"
        );

        let holdings = svc.get_holdings_for_account(&account.id).await.unwrap();
        assert_eq!(holdings.len(), 1);
        assert_eq!(holdings[0].quantity, micro(2));
        assert_eq!(holdings[0].average_price, micro(100));
    }

    // TRX-048 — open_holding participates in VWAP alongside Purchase
    #[tokio::test]
    async fn test_open_holding_participates_in_vwap_alongside_purchase() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Acc".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;

        // OpeningBalance: 2 units, total_cost = 200
        svc.open_holding(
            &account.id,
            asset_id.clone(),
            "2024-01-01".to_string(),
            micro(2),
            micro(200),
        )
        .await
        .unwrap();

        // Purchase: 2 units @ 100 → total = 200
        svc.buy_holding(
            &account.id,
            asset_id.clone(),
            "2024-02-01".to_string(),
            micro(2),
            micro(100),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();

        let holdings = svc.get_holdings_for_account(&account.id).await.unwrap();
        let h = holdings
            .iter()
            .find(|h| h.asset_id == asset_id)
            .expect("holding must exist after VWAP test operations");
        // VWAP = (200 + 200) / 4 = 100
        assert_eq!(h.quantity, micro(4));
        assert_eq!(
            h.average_price,
            micro(100),
            "VWAP must include OpeningBalance totals"
        );
    }

    // TRX-049 — multiple open_holding entries for same pair are all persisted
    #[tokio::test]
    async fn test_open_holding_allows_multiple_for_same_pair() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Acc".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();

        svc.open_holding(
            &account.id,
            asset_id.clone(),
            "2023-01-01".to_string(),
            micro(1),
            micro(100),
        )
        .await
        .unwrap();
        svc.open_holding(
            &account.id,
            asset_id.clone(),
            "2023-06-01".to_string(),
            micro(2),
            micro(200),
        )
        .await
        .unwrap();

        let txs = svc.get_transactions(&account.id, &asset_id).await.unwrap();
        assert_eq!(txs.len(), 2, "both opening balance rows must be persisted");

        let holdings = svc.get_holdings_for_account(&account.id).await.unwrap();
        assert_eq!(holdings[0].quantity, micro(3), "quantities must accumulate");
    }

    // TRX-031 — correct_transaction updates the persisted holding
    #[tokio::test]
    async fn test_correct_transaction_updates_holding() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Acc".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;
        let tx = svc
            .buy_holding(
                &account.id,
                asset_id.clone(),
                "2024-01-01".to_string(),
                micro(2),
                micro(100),
                micro(1),
                0,
                None,
            )
            .await
            .unwrap();
        svc.correct_transaction(
            &account.id,
            &tx.id,
            "2024-01-01".to_string(),
            micro(2),
            micro(200),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();
        let holdings = svc.get_holdings_for_account(&account.id).await.unwrap();
        let asset_holding = holdings
            .iter()
            .find(|h| h.asset_id == asset_id)
            .expect("asset holding present");
        assert_eq!(
            asset_holding.average_price,
            micro(200),
            "VWAP should update to 200"
        );
    }

    // Note: pure delegate methods — read-side (get_all, get_by_id,
    // get_holdings_for_account, get_holding_by_account_asset,
    // get_transaction_by_id, get_transactions, get_asset_ids_for_account)
    // and write-side (delete) — are exercised end-to-end against a real
    // SQLite repository in tests/account_service_crud.rs
    // (B33 — avoid trivial mock-passthrough tests).

    // CSH-100 — record_deposit and record_withdrawal publish TransactionUpdated.
    // Frontend reactivity (ACD-039, MKT-036) re-fetches on this signal.
    //
    // Pattern: do all setup first, THEN subscribe to the bus. New subscribers
    // see the latest value but `changed()` only fires on subsequent updates,
    // so this avoids racing against events emitted during setup.
    #[tokio::test]
    async fn csh_100_record_deposit_publishes_transaction_updated_event() {
        use std::time::Duration;
        let pool = make_pool().await;
        let bus = Arc::new(SideEffectEventBus::new());
        let svc = AccountService::new(
            Box::new(SqliteAccountRepository::new(pool.clone())),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
            Box::new(SqliteTransactionRepository::new(pool.clone())),
        )
        .with_event_bus(Arc::clone(&bus));
        let account = svc
            .create(
                "Acc".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;

        // Subscribe AFTER setup — `changed()` fires only on the next publish.
        let mut rx = bus.subscribe();
        svc.record_deposit(&account.id, "2020-02-01".to_string(), 50_000_000, None)
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_millis(200), rx.changed())
            .await
            .expect("TransactionUpdated event not received within 200ms")
            .expect("watch sender dropped before event fired");
        assert_eq!(*rx.borrow(), Event::TransactionUpdated);
    }

    #[tokio::test]
    async fn csh_100_record_withdrawal_publishes_transaction_updated_event() {
        use std::time::Duration;
        let pool = make_pool().await;
        let bus = Arc::new(SideEffectEventBus::new());
        let svc = AccountService::new(
            Box::new(SqliteAccountRepository::new(pool.clone())),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
            Box::new(SqliteTransactionRepository::new(pool.clone())),
        )
        .with_event_bus(Arc::clone(&bus));
        let account = svc
            .create(
                "Acc".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;

        // Subscribe AFTER setup so we only observe the withdrawal's event.
        let mut rx = bus.subscribe();
        svc.record_withdrawal(&account.id, "2020-02-01".to_string(), 100_000_000, None)
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_millis(200), rx.changed())
            .await
            .expect("TransactionUpdated event not received within 200ms")
            .expect("watch sender dropped before event fired");
        assert_eq!(*rx.borrow(), Event::TransactionUpdated);
    }

    // -------------------------------------------------------------------------
    // Typed cash service error paths (B34) — mock-based unit tests for
    // record_deposit / record_withdrawal covering all four typed-Result
    // variants of AccountError. Happy paths are covered by the SQLite
    // csh_100_* tests above.
    // -------------------------------------------------------------------------

    fn mock_cash_svc(ar: MockAccountRepository) -> AccountService {
        AccountService::new(
            Box::new(ar),
            Box::new(MockHoldingRepository::new()),
            Box::new(MockTransactionRepository::new()),
        )
    }

    // CSH-021 — non-positive deposit amount surfaces from `Transaction::new_deposit`
    // (the cash factory's input validation, per Rule B').
    #[tokio::test]
    async fn record_deposit_returns_amount_not_positive_on_zero() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_get_with_holdings_and_transactions()
            .once()
            .returning(|_| {
                Ok(Some(
                    Account::new(
                        "Test".to_string(),
                        String::new(),
                        "EUR".to_string(),
                        UpdateFrequency::ManualMonth,
                        false,
                    )
                    .unwrap(),
                ))
            });
        let svc = mock_cash_svc(mock_ar);
        let err = svc
            .record_deposit("acc", "2020-01-01".to_string(), 0, None)
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::AmountNotPositive),
            "got: {err:?}"
        );
    }

    // load_account translates Ok(None) → AccountError::AccountNotFound.
    #[tokio::test]
    async fn record_deposit_returns_account_not_found_when_repo_returns_none() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_get_with_holdings_and_transactions()
            .once()
            .returning(|_| Ok(None));
        let svc = mock_cash_svc(mock_ar);
        let err = svc
            .record_deposit("missing", "2020-01-01".to_string(), 100, None)
            .await
            .unwrap_err();
        match err {
            AccountError::AccountNotFound { account_id } => {
                assert_eq!(account_id, "missing");
            }
            other => panic!("expected AccountNotFound{{missing}}, got: {other:?}"),
        }
    }

    // load_account translates a repo Err → AccountError::DatabaseError.
    #[tokio::test]
    async fn record_deposit_returns_infrastructure_when_load_fails() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_get_with_holdings_and_transactions()
            .once()
            .returning(|_| Err(SimulatedSaveError.into()));
        let svc = mock_cash_svc(mock_ar);
        let err = svc
            .record_deposit("acc", "2020-01-01".to_string(), 100, None)
            .await
            .unwrap_err();
        assert!(matches!(err, AccountError::DatabaseError), "got: {err:?}");
    }

    // save_account translates a repo Err → AccountError::DatabaseError.
    #[tokio::test]
    async fn record_deposit_returns_infrastructure_when_save_fails() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_get_with_holdings_and_transactions()
            .once()
            .returning(|_| {
                let acc = Account::new(
                    "Test".to_string(),
                    String::new(),
                    "EUR".to_string(),
                    UpdateFrequency::ManualMonth,
                    false,
                )
                .unwrap();
                Ok(Some(acc))
            });
        mock_ar
            .expect_save()
            .once()
            .returning(|_| Err(SimulatedSaveError.into()));
        let svc = mock_cash_svc(mock_ar);
        let err = svc
            .record_deposit("acc", "2020-01-01".to_string(), 100, None)
            .await
            .unwrap_err();
        assert!(matches!(err, AccountError::DatabaseError), "got: {err:?}");
    }

    // CSH-031 — non-positive withdrawal amount surfaces from
    // `Transaction::new_withdrawal` (the cash factory's input validation).
    #[tokio::test]
    async fn record_withdrawal_returns_amount_not_positive_on_zero() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_get_with_holdings_and_transactions()
            .once()
            .returning(|_| {
                Ok(Some(
                    Account::new(
                        "Test".to_string(),
                        String::new(),
                        "EUR".to_string(),
                        UpdateFrequency::ManualMonth,
                        false,
                    )
                    .unwrap(),
                ))
            });
        let svc = mock_cash_svc(mock_ar);
        let err = svc
            .record_withdrawal("acc", "2020-01-01".to_string(), 0, None)
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::AmountNotPositive),
            "got: {err:?}"
        );
    }

    // load_account translates Ok(None) → AccountError::AccountNotFound.
    #[tokio::test]
    async fn record_withdrawal_returns_account_not_found_when_repo_returns_none() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_get_with_holdings_and_transactions()
            .once()
            .returning(|_| Ok(None));
        let svc = mock_cash_svc(mock_ar);
        let err = svc
            .record_withdrawal("missing", "2020-01-01".to_string(), 100, None)
            .await
            .unwrap_err();
        match err {
            AccountError::AccountNotFound { account_id } => {
                assert_eq!(account_id, "missing");
            }
            other => panic!("expected AccountNotFound{{missing}}, got: {other:?}"),
        }
    }

    // load_account translates a repo Err → AccountError::DatabaseError.
    #[tokio::test]
    async fn record_withdrawal_returns_infrastructure_when_load_fails() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_get_with_holdings_and_transactions()
            .once()
            .returning(|_| Err(SimulatedSaveError.into()));
        let svc = mock_cash_svc(mock_ar);
        let err = svc
            .record_withdrawal("acc", "2020-01-01".to_string(), 100, None)
            .await
            .unwrap_err();
        assert!(matches!(err, AccountError::DatabaseError), "got: {err:?}");
    }

    // save_account translates a repo Err → AccountError::DatabaseError.
    #[tokio::test]
    async fn record_withdrawal_returns_infrastructure_when_save_fails() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_get_with_holdings_and_transactions()
            .once()
            .returning(|_| {
                // Seed both a Cash Holding AND a matching Deposit Transaction
                // via `Account::restore_with_positions`. Both are required:
                // `apply_withdrawal` first checks `cash_holding_quantity()` (which
                // reads from `holdings`), then runs `replay_cash_holding()` (which
                // rebuilds the running balance from `transactions`). A holding
                // without a corresponding deposit would pass the snapshot check
                // but trip the chronological replay.
                let acc = Account::new(
                    "Test".to_string(),
                    String::new(),
                    "EUR".to_string(),
                    UpdateFrequency::ManualMonth,
                    false,
                )
                .unwrap();
                // CSH-080 only fails when current cash < requested amount. Seed
                // micro(1_000) (≈ €1,000) — comfortably above the test's micro(100)
                // withdrawal. Exact value isn't load-bearing; only the inequality.
                let cash_holding = Holding::restore(
                    "h-cash".to_string(),
                    acc.id.clone(),
                    acc.cash_asset_id(),
                    micro(1_000),
                    1_000_000,
                    0,
                    None,
                );
                let seed_deposit = Transaction::new_deposit(
                    acc.id.clone(),
                    acc.cash_asset_id(),
                    "2020-01-01".to_string(),
                    micro(1_000),
                    None,
                )
                .expect("seed deposit must validate");
                Ok(Some(Account::restore_with_positions(
                    acc.id,
                    acc.name,
                    acc.bank_name,
                    acc.currency,
                    acc.update_frequency,
                    acc.management_fees_enabled,
                    vec![cash_holding],
                    vec![seed_deposit],
                )))
            });
        mock_ar
            .expect_save()
            .once()
            .returning(|_| Err(SimulatedSaveError.into()));
        let svc = mock_cash_svc(mock_ar);
        let err = svc
            .record_withdrawal("acc", "2020-02-01".to_string(), micro(100), None)
            .await
            .unwrap_err();
        assert!(matches!(err, AccountError::DatabaseError), "got: {err:?}");
    }

    // -------------------------------------------------------------------------
    // Account CRUD typed-error coverage (PR 5)
    // -------------------------------------------------------------------------

    // create surfaces find_by_name repo failure as Application(DatabaseError).
    #[tokio::test]
    async fn test_create_returns_database_error_when_find_by_name_fails() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_find_by_name()
            .once()
            .returning(|_| Err(SimulatedSaveError.into()));
        let svc = AccountService::new(
            Box::new(mock_ar),
            Box::new(MockHoldingRepository::new()),
            Box::new(MockTransactionRepository::new()),
        );
        let err = svc
            .create(
                "Test".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AccountError::DatabaseError), "got: {err:?}");
    }

    // create surfaces repo.create failure (after passing the uniqueness
    // pre-check) as Application(DatabaseError).
    #[tokio::test]
    async fn test_create_returns_database_error_when_repo_create_fails() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar.expect_find_by_name().once().returning(|_| Ok(None));
        mock_ar
            .expect_create()
            .once()
            .returning(|_| Err(SimulatedSaveError.into()));
        let svc = AccountService::new(
            Box::new(mock_ar),
            Box::new(MockHoldingRepository::new()),
            Box::new(MockTransactionRepository::new()),
        );
        let err = svc
            .create(
                "Test".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AccountError::DatabaseError), "got: {err:?}");
    }

    // delete surfaces repo failure as AccountError::DatabaseError.
    #[tokio::test]
    async fn test_delete_returns_database_error_when_repo_fails() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_delete()
            .once()
            .returning(|_| Err(SimulatedSaveError.into()));
        let svc = AccountService::new(
            Box::new(mock_ar),
            Box::new(MockHoldingRepository::new()),
            Box::new(MockTransactionRepository::new()),
        );
        let err = svc.delete("any-id").await.unwrap_err();
        assert!(matches!(err, AccountError::DatabaseError), "got: {err:?}");
    }

    // get_by_id translates raw repo failure to AccountError::DatabaseError.
    #[tokio::test]
    async fn get_by_id_translates_repo_failure_to_database_error() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_get_by_id()
            .once()
            .returning(|_| Err(SimulatedSaveError.into()));
        let svc = AccountService::new(
            Box::new(mock_ar),
            Box::new(MockHoldingRepository::new()),
            Box::new(MockTransactionRepository::new()),
        );
        let err = svc.get_by_id("any-id").await.unwrap_err();
        assert!(matches!(err, AccountError::DatabaseError), "got: {err:?}");
    }

    // get_holdings_for_account translates raw repo failure to AccountError::DatabaseError.
    #[tokio::test]
    async fn get_holdings_for_account_translates_repo_failure_to_database_error() {
        let mut mock_hr = MockHoldingRepository::new();
        mock_hr
            .expect_get_by_account()
            .once()
            .returning(|_| Err(SimulatedSaveError.into()));
        let svc = AccountService::new(
            Box::new(MockAccountRepository::new()),
            Box::new(mock_hr),
            Box::new(MockTransactionRepository::new()),
        );
        let err = svc.get_holdings_for_account("any-id").await.unwrap_err();
        assert!(matches!(err, AccountError::DatabaseError), "got: {err:?}");
    }

    // -------------------------------------------------------------------------
    // FSD-022/026 — record_free_shares service method
    // -------------------------------------------------------------------------

    // FSD-022 — record_free_shares persists the transaction and updates the
    // holding: quantity increases, cost basis unchanged (VWAP dilutes).
    #[tokio::test]
    async fn fsd_022_record_free_shares_persists_transaction_and_updates_holding() {
        // FSD-022 — end-to-end through AccountService (real SQLite)
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "FSD Account".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;

        // Buy 10 units @ 100
        svc.buy_holding(
            &account.id,
            asset_id.clone(),
            "2024-01-01".to_string(),
            micro(10),
            micro(100),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();

        let holdings_before = svc.get_holdings_for_account(&account.id).await.unwrap();
        let cost_basis_before = holdings_before
            .iter()
            .find(|h| h.asset_id == asset_id)
            .map(|h| h.quantity as i128 * h.average_price as i128 / 1_000_000)
            .unwrap();

        // Record 5 free shares
        let tx = svc
            .record_free_shares(
                &account.id,
                asset_id.clone(),
                "2024-06-15".to_string(),
                micro(5),
                None,
            )
            .await
            .unwrap();

        // FSD-022 — returned Transaction must carry the FreeShares type and correct fields
        assert_eq!(
            tx.transaction_type,
            crate::context::account::TransactionType::FreeShares
        );
        assert_eq!(tx.asset_id, asset_id);
        assert_eq!(tx.quantity, micro(5));
        assert_eq!(tx.unit_price, 0, "unit_price must be 0 (FSD-023)");
        assert_eq!(
            tx.exchange_rate, 1_000_000,
            "exchange_rate must be 1_000_000"
        );
        assert_eq!(tx.fees, 0, "fees must be 0");
        assert_eq!(tx.total_amount, 0, "total_amount must be 0 (FSD-023)");
        assert!(tx.realized_pnl.is_none(), "realized_pnl must be None");

        let holdings_after = svc.get_holdings_for_account(&account.id).await.unwrap();
        let holding_after = holdings_after
            .iter()
            .find(|h| h.asset_id == asset_id)
            .unwrap();

        // FSD-022a — quantity increased
        assert_eq!(
            holding_after.quantity,
            micro(15),
            "quantity must be 15 after 10 + 5 free"
        );
        // FSD-023 — underlying cost unchanged → VWAP dilutes to the exact floored
        // value (TRX-026 floor convention).
        let expected_diluted_vwap =
            (cost_basis_before * 1_000_000 / holding_after.quantity as i128) as i64;
        assert_eq!(
            holding_after.average_price, expected_diluted_vwap,
            "average price must equal floor(cost_basis / new_quantity) after free-share distribution"
        );
    }

    // FSD-026 — record_free_shares publishes TransactionUpdated event on success.
    #[tokio::test]
    async fn fsd_026_record_free_shares_publishes_transaction_updated_event() {
        // FSD-026 — TransactionUpdated must be emitted after a successful distribution
        use std::time::Duration;
        let pool = make_pool().await;
        let bus = Arc::new(SideEffectEventBus::new());
        let svc = AccountService::new(
            Box::new(SqliteAccountRepository::new(pool.clone())),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
            Box::new(SqliteTransactionRepository::new(pool.clone())),
        )
        .with_event_bus(Arc::clone(&bus));

        let (_, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "FSD Acc".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;
        svc.buy_holding(
            &account.id,
            asset_id.clone(),
            "2024-01-01".to_string(),
            micro(10),
            micro(100),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();

        // Subscribe AFTER setup so only the free-shares event is observed.
        let mut rx = bus.subscribe();
        svc.record_free_shares(
            &account.id,
            asset_id.clone(),
            "2024-06-15".to_string(),
            micro(5),
            None,
        )
        .await
        .unwrap();

        tokio::time::timeout(Duration::from_millis(200), rx.changed())
            .await
            .expect("TransactionUpdated event not received within 200ms")
            .expect("watch sender dropped");
        use crate::core::event_bus::Event;
        assert_eq!(
            *rx.borrow(),
            Event::TransactionUpdated,
            "record_free_shares must publish TransactionUpdated (FSD-026)"
        );
    }

    // INT-025/050 — record_interest publishes TransactionUpdated on success; the
    // account has management_fees_enabled = false, proving the interest path is
    // independent of the fee parameter (INT-050).
    #[tokio::test]
    async fn int_025_record_interest_publishes_transaction_updated_event() {
        use std::time::Duration;
        let pool = make_pool().await;
        let bus = Arc::new(SideEffectEventBus::new());
        let svc = AccountService::new(
            Box::new(SqliteAccountRepository::new(pool.clone())),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
            Box::new(SqliteTransactionRepository::new(pool.clone())),
        )
        .with_event_bus(Arc::clone(&bus));

        let (_, asset_id) = setup(&pool).await;
        // INT-050 — fees stay disabled; interest must work regardless.
        let account = svc
            .create(
                "INT Acc".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;
        svc.buy_holding(
            &account.id,
            asset_id.clone(),
            "2024-01-01".to_string(),
            micro(10),
            micro(100),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();

        // Subscribe AFTER setup so only the interest event is observed.
        let mut rx = bus.subscribe();
        svc.record_interest(
            &account.id,
            asset_id.clone(),
            "2024-06-15".to_string(),
            None,
            Some(micro(1)),
            None,
        )
        .await
        .unwrap();

        tokio::time::timeout(Duration::from_millis(200), rx.changed())
            .await
            .expect("TransactionUpdated event not received within 200ms")
            .expect("watch sender dropped");
        use crate::core::event_bus::Event;
        assert_eq!(
            *rx.borrow(),
            Event::TransactionUpdated,
            "record_interest must publish TransactionUpdated (INT-025)"
        );
    }

    // FSD-022 — record_free_shares propagates save failure as Application(DatabaseError).
    #[tokio::test]
    async fn fsd_022_record_free_shares_returns_database_error_when_save_fails() {
        // FSD-022 — Unit-of-Work failure surface
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_get_with_holdings_and_transactions()
            .once()
            .returning(|_| {
                // Seed a holding so the apply_free_shares call reaches save()
                let mut acc = Account::new(
                    "Test".to_string(),
                    String::new(),
                    "EUR".to_string(),
                    UpdateFrequency::ManualMonth,
                    false,
                )
                .unwrap();
                // Seed cash first (CSH-041 — a buy needs sufficient cash), then a buy
                // so the holding exists.
                acc.record_deposit("2024-01-01".to_string(), micro(1_000), None)
                    .expect("seed deposit");
                acc.buy_holding(
                    "asset-1".to_string(),
                    "2024-01-01".to_string(),
                    micro(5),
                    micro(100),
                    micro(1),
                    0,
                    None,
                )
                .expect("seed buy");
                acc.pending_changes.clear();
                Ok(Some(acc))
            });
        mock_ar
            .expect_save()
            .once()
            .returning(|_| Err(SimulatedSaveError.into()));

        let svc = AccountService::new(
            Box::new(mock_ar),
            Box::new(MockHoldingRepository::new()),
            Box::new(MockTransactionRepository::new()),
        );

        let err = svc
            .record_free_shares(
                "any-id",
                "asset-1".to_string(),
                "2024-06-15".to_string(),
                micro(3),
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                AccountError::DatabaseError
            ),
            "record_free_shares must surface save failures as Application(DatabaseError), got: {err:?}"
        );
    }

    // -------------------------------------------------------------------------
    // FEE-012/021/022/023/027 — record_management_fee service method
    // -------------------------------------------------------------------------

    // FEE-012 — record_management_fee persists the transaction and updates the holding:
    // quantity decreases, cost basis unchanged (VWAP concentrates).
    #[tokio::test]
    async fn fee_012_record_management_fee_persists_transaction_and_updates_holding() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "FEE Account".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let account = enable_management_fees(&svc, &account).await;
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;

        // Buy 100 units @ 50 (total cost = 5000 in account currency).
        svc.buy_holding(
            &account.id,
            asset_id.clone(),
            "2024-01-01".to_string(),
            micro(100),
            micro(50),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();

        // Record a 1% management fee (1_000_000 micro-percent).
        // Expected removed qty = floor(100_000_000 × 1_000_000 / 100_000_000) = 1_000_000 (1 unit).
        let tx = svc
            .record_management_fee(
                &account.id,
                asset_id.clone(),
                "2024-06-30".to_string(),
                1_000_000,
                None,
            )
            .await
            .unwrap();

        // FEE-012 — returned Transaction must carry ManagementFee type and zero-cost packing.
        assert_eq!(
            tx.transaction_type,
            crate::context::account::TransactionType::ManagementFee
        );
        assert_eq!(tx.asset_id, asset_id);
        assert_eq!(tx.unit_price, 0, "unit_price must be 0 (FEE-023)");
        assert_eq!(
            tx.exchange_rate, 1_000_000,
            "exchange_rate must be 1_000_000"
        );
        assert_eq!(tx.fees, 0, "fees must be 0");
        assert_eq!(tx.total_amount, 0, "total_amount must be 0 (FEE-023)");
        assert!(tx.realized_pnl.is_none(), "realized_pnl must be None");

        let holdings_after = svc.get_holdings_for_account(&account.id).await.unwrap();
        let holding_after = holdings_after
            .iter()
            .find(|h| h.asset_id == asset_id)
            .expect("holding for asset_id must exist after management fee deduction");

        // FEE-023 — VWAP concentrates: qty decreases, cost basis unchanged.
        assert!(
            holding_after.quantity < micro(100),
            "quantity must decrease after fee deduction"
        );
    }

    // FEE-050 — a fee deduction reduces quantity but leaves the recorded cost
    // basis UNCHANGED, so the average cost per share rises (VWAP concentrates).
    #[tokio::test]
    async fn fee_050_management_fee_preserves_cost_basis_and_raises_average_price() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "FEE-050".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let account = enable_management_fees(&svc, &account).await;
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;

        // Buy 100 units @ 50 → cost basis = 100 × 50 = 5000 (account currency).
        svc.buy_holding(
            &account.id,
            asset_id.clone(),
            "2024-01-01".to_string(),
            micro(100),
            micro(50),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();

        let holdings_before = svc.get_holdings_for_account(&account.id).await.unwrap();
        let holding_before = holdings_before
            .iter()
            .find(|h| h.asset_id == asset_id)
            .expect("holding must exist after buy")
            .clone();
        let cost_basis_before =
            holding_before.quantity as i128 * holding_before.average_price as i128 / 1_000_000;

        // Record a 1% management fee — removes floor(100 × 1%) = 1 unit.
        svc.record_management_fee(
            &account.id,
            asset_id.clone(),
            "2024-06-30".to_string(),
            1_000_000,
            None,
        )
        .await
        .unwrap();

        let holdings_after = svc.get_holdings_for_account(&account.id).await.unwrap();
        let holding_after = holdings_after
            .iter()
            .find(|h| h.asset_id == asset_id)
            .expect("holding must still exist after fee deduction")
            .clone();
        let cost_basis_after =
            holding_after.quantity as i128 * holding_after.average_price as i128 / 1_000_000;

        // FEE-050 — quantity dropped …
        assert!(
            holding_after.quantity < holding_before.quantity,
            "quantity must decrease after a fee deduction (FEE-050)"
        );
        // … the recorded cost basis is left UNCHANGED — the average price absorbs the
        // change by concentrating to floor(cost_basis / new_quantity) (FEE-023, TRX-026
        // floor convention). The stored VWAP must equal that exact floored value.
        let expected_concentrated_vwap =
            (cost_basis_before * 1_000_000 / holding_after.quantity as i128) as i64;
        assert_eq!(
            holding_after.average_price, expected_concentrated_vwap,
            "average price must equal floor(cost_basis / new_quantity) — cost basis unchanged (FEE-050/023)"
        );
        // … so the average cost per share rises (VWAP concentrates).
        assert!(
            holding_after.average_price > holding_before.average_price,
            "average price must rise as the cost basis concentrates over fewer shares (FEE-050)"
        );
        // The cost basis is preserved up to the per-share floor error (< 1 micro-unit
        // per remaining share); it can only round DOWN, never up.
        let shares_after = holding_after.quantity / 1_000_000;
        assert!(
            cost_basis_after <= cost_basis_before
                && cost_basis_after > cost_basis_before - (shares_after as i128 + 1),
            "cost basis must be unchanged within the floor tolerance (FEE-050): before={cost_basis_before}, after={cost_basis_after}"
        );
    }

    // FEE-021 — PercentageNotPositive when percent_micros <= 0.
    #[tokio::test]
    async fn fee_021_record_management_fee_rejects_zero_percent() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "FEE Zero".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();

        let err = svc
            .record_management_fee(
                &account.id,
                asset_id.clone(),
                "2024-06-30".to_string(),
                0,
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::PercentageNotPositive),
            "expected PercentageNotPositive, got: {err:?}"
        );
    }

    // FEE-021 — PercentageAboveHundred when percent_micros > 100_000_000.
    #[tokio::test]
    async fn fee_021_record_management_fee_rejects_above_hundred_percent() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "FEE Above100".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();

        let err = svc
            .record_management_fee(
                &account.id,
                asset_id.clone(),
                "2024-06-30".to_string(),
                100_000_001,
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::PercentageAboveHundred),
            "expected PercentageAboveHundred, got: {err:?}"
        );
    }

    // FEE-022 — AccountNotFound when the account does not exist.
    #[tokio::test]
    async fn fee_022_record_management_fee_rejects_unknown_account() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;

        let err = svc
            .record_management_fee(
                "nonexistent-account",
                asset_id.clone(),
                "2024-06-30".to_string(),
                1_000_000,
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::AccountNotFound { .. }),
            "expected AccountNotFound, got: {err:?}"
        );
    }

    // FEE-027 — record_management_fee propagates save failure as DatabaseError.
    #[tokio::test]
    async fn fee_027_record_management_fee_returns_database_error_when_save_fails() {
        let mut mock_ar = MockAccountRepository::new();
        mock_ar
            .expect_get_with_holdings_and_transactions()
            .once()
            .returning(|_| {
                let mut acc = Account::new(
                    "Test".to_string(),
                    String::new(),
                    "EUR".to_string(),
                    UpdateFrequency::ManualMonth,
                    false,
                )
                .unwrap();
                acc.management_fees_enabled = true;
                acc.record_deposit("2024-01-01".to_string(), micro(1_000), None)
                    .expect("seed deposit");
                acc.buy_holding(
                    "asset-1".to_string(),
                    "2024-01-01".to_string(),
                    micro(10),
                    micro(100),
                    micro(1),
                    0,
                    None,
                )
                .expect("seed buy");
                acc.pending_changes.clear();
                Ok(Some(acc))
            });
        mock_ar
            .expect_save()
            .once()
            .returning(|_| Err(SimulatedSaveError.into()));

        let svc = AccountService::new(
            Box::new(mock_ar),
            Box::new(MockHoldingRepository::new()),
            Box::new(MockTransactionRepository::new()),
        );

        let err = svc
            .record_management_fee(
                "any-id",
                "asset-1".to_string(),
                "2024-06-30".to_string(),
                1_000_000,
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::DatabaseError),
            "record_management_fee must surface save failures as DatabaseError, got: {err:?}"
        );
    }

    // FEE-027 — a one-off fee dated before a later Sell is rejected when the
    // removal would starve that Sell in chronological replay (CascadingOversell).
    #[tokio::test]
    async fn fee_027_record_management_fee_rejects_one_off_downstream_oversell() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "FEE-027".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let account = enable_management_fees(&svc, &account).await;
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;

        // Buy 100 on 2024-01-01, then sell all 100 on 2024-03-01 (closes the position).
        svc.buy_holding(
            &account.id,
            asset_id.clone(),
            "2024-01-01".to_string(),
            micro(100),
            micro(50),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();
        svc.sell_holding(
            &account.id,
            asset_id.clone(),
            "2024-03-01".to_string(),
            micro(100),
            micro(60),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();

        // A 10% fee on 2024-02-01 would remove 10 units, leaving 90 — the later
        // 2024-03-01 sell of 100 can then no longer source its quantity.
        let err = svc
            .record_management_fee(
                &account.id,
                asset_id.clone(),
                "2024-02-01".to_string(),
                10_000_000,
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::CascadingOversell),
            "a one-off fee that starves a later sell must be rejected with CascadingOversell, got: {err:?}"
        );
    }

    // -------------------------------------------------------------------------
    // INT-021/022/023/024 — record_interest service method
    // -------------------------------------------------------------------------

    // INT-022 — percent mode: credited qty = floor(holding_qty_as_of(date) × percent / 100%);
    // the holding gains the credit at zero cost (VWAP dilutes, INT-024).
    #[tokio::test]
    async fn int_022_record_interest_percent_mode_credits_and_dilutes() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "INT Account".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;

        // Buy 100 units @ 50.
        svc.buy_holding(
            &account.id,
            asset_id.clone(),
            "2024-01-01".to_string(),
            micro(100),
            micro(50),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();

        let holdings_before = svc.get_holdings_for_account(&account.id).await.unwrap();
        let holding_before = holdings_before
            .iter()
            .find(|h| h.asset_id == asset_id)
            .unwrap()
            .clone();
        let cost_basis_before =
            holding_before.quantity as i128 * holding_before.average_price as i128 / 1_000_000;

        // Record a 10% interest (10_000_000 micro-percent).
        // Expected credited qty = floor(100_000_000 × 10_000_000 / 100_000_000) = 10 units.
        let tx = svc
            .record_interest(
                &account.id,
                asset_id.clone(),
                "2024-12-31".to_string(),
                Some(10_000_000),
                None,
                None,
            )
            .await
            .unwrap();

        // INT-024 — returned Transaction must carry the Interest type and zero-cost packing.
        assert_eq!(
            tx.transaction_type,
            crate::context::account::TransactionType::Interest
        );
        assert_eq!(tx.asset_id, asset_id);
        assert_eq!(tx.quantity, micro(10), "credited qty must be 10% of 100");
        assert_eq!(tx.unit_price, 0, "unit_price must be 0 (INT-024)");
        assert_eq!(
            tx.exchange_rate, 1_000_000,
            "exchange_rate must be 1_000_000"
        );
        assert_eq!(tx.fees, 0, "fees must be 0");
        assert_eq!(tx.total_amount, 0, "total_amount must be 0 (INT-024)");
        assert!(tx.realized_pnl.is_none(), "realized_pnl must be None");

        let holdings_after = svc.get_holdings_for_account(&account.id).await.unwrap();
        let holding_after = holdings_after
            .iter()
            .find(|h| h.asset_id == asset_id)
            .unwrap();
        assert_eq!(
            holding_after.quantity,
            micro(110),
            "quantity must be 110 after 100 + 10 interest"
        );
        // INT-024 — cost basis unchanged → VWAP dilutes to floor(cost_basis / new_quantity).
        let expected_diluted_vwap =
            (cost_basis_before * 1_000_000 / holding_after.quantity as i128) as i64;
        assert_eq!(
            holding_after.average_price, expected_diluted_vwap,
            "average price must equal floor(cost_basis / new_quantity) after the interest credit"
        );
    }

    // INT-021 — quantity mode: the provided quantity is credited directly.
    #[tokio::test]
    async fn int_021_record_interest_quantity_mode_credits_units() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "INT Qty".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;
        svc.buy_holding(
            &account.id,
            asset_id.clone(),
            "2024-01-01".to_string(),
            micro(100),
            micro(50),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();

        let tx = svc
            .record_interest(
                &account.id,
                asset_id.clone(),
                "2024-12-31".to_string(),
                None,
                Some(micro(5)),
                None,
            )
            .await
            .unwrap();
        assert_eq!(tx.quantity, micro(5));

        let holdings_after = svc.get_holdings_for_account(&account.id).await.unwrap();
        let holding_after = holdings_after
            .iter()
            .find(|h| h.asset_id == asset_id)
            .unwrap();
        assert_eq!(
            holding_after.quantity,
            micro(105),
            "quantity must be 105 after 100 + 5 interest"
        );
    }

    // INT-021 — both or neither of percent / quantity → InterestAmountInvalid.
    #[tokio::test]
    async fn int_021_record_interest_rejects_both_and_neither() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "INT XOR".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();

        let err = svc
            .record_interest(
                &account.id,
                asset_id.clone(),
                "2024-12-31".to_string(),
                Some(1_000_000),
                Some(micro(5)),
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::InterestAmountInvalid),
            "both provided must be rejected, got: {err:?}"
        );

        let err = svc
            .record_interest(
                &account.id,
                asset_id.clone(),
                "2024-12-31".to_string(),
                None,
                None,
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::InterestAmountInvalid),
            "neither provided must be rejected, got: {err:?}"
        );
    }

    // INT-023 — quantity-mode interest on the Cash Asset credits the cash balance:
    // deposit 1000, interest 50 → 1050.
    #[tokio::test]
    async fn int_023_record_interest_credits_cash_line() {
        let pool = make_pool().await;
        let (svc, _asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "INT Cash".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        // Seed the system Cash Asset row (FK target), then deposit 1000.
        sqlx::query(
            "INSERT OR IGNORE INTO categories (id, name, is_deleted) VALUES ('system-cash-category', 'cash', 0)",
        )
        .execute(&pool)
        .await
        .expect("seed cash category");
        sqlx::query(
            "INSERT OR IGNORE INTO assets (id, name, reference, asset_class, category_id, currency, risk_level) \
             VALUES ('system-cash-eur', 'Cash EUR', 'EUR', 'Cash', 'system-cash-category', 'EUR', 1)",
        )
        .execute(&pool)
        .await
        .expect("seed cash asset");
        svc.record_deposit(&account.id, "2024-01-01".to_string(), micro(1_000), None)
            .await
            .unwrap();

        svc.record_interest(
            &account.id,
            "system-cash-eur".to_string(),
            "2024-06-15".to_string(),
            None,
            Some(micro(50)),
            None,
        )
        .await
        .unwrap();

        let holdings = svc.get_holdings_for_account(&account.id).await.unwrap();
        let cash = holdings
            .iter()
            .find(|h| h.asset_id == "system-cash-eur")
            .expect("cash holding must exist");
        assert_eq!(
            cash.quantity,
            micro(1_050),
            "cash balance must be 1000 + 50 after the interest credit (INT-023)"
        );
    }

    // -------------------------------------------------------------------------
    // FEE-030/031/032/033/034 — create_fee_schedule service method
    // -------------------------------------------------------------------------

    // FEE-030 — create_fee_schedule persists the schedule and returns it.
    #[tokio::test]
    async fn fee_030_create_fee_schedule_returns_schedule() {
        use crate::context::account::FeeFrequency;
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Schedule Acct".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let account = enable_management_fees(&svc, &account).await;

        let schedule = svc
            .create_fee_schedule(
                &account.id,
                asset_id.clone(),
                1_000_000, // 1%
                FeeFrequency::Monthly,
                "2024-01-01".to_string(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(schedule.account_id, account.id);
        assert_eq!(schedule.asset_id, asset_id);
        assert_eq!(schedule.annual_rate_percent_micros, 1_000_000);
        assert!(matches!(schedule.frequency, FeeFrequency::Monthly));
        assert!(schedule.active);
        assert!(schedule.last_applied_period.is_none());
    }

    // FEE-033 — creating a schedule persists it but removes NO shares; the holding
    // quantity is unchanged after create_fee_schedule (deductions come only from
    // generation, FEE-04x).
    #[tokio::test]
    async fn fee_033_create_fee_schedule_removes_no_shares() {
        use crate::context::account::FeeFrequency;
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "FEE-033".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let account = enable_management_fees(&svc, &account).await;
        seed_cash_for_account(&pool, &svc, &account.id, "EUR").await;

        // Establish a holding of 100 units.
        svc.buy_holding(
            &account.id,
            asset_id.clone(),
            "2024-01-01".to_string(),
            micro(100),
            micro(50),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();

        let quantity_before = svc
            .get_holdings_for_account(&account.id)
            .await
            .unwrap()
            .iter()
            .find(|h| h.asset_id == asset_id)
            .expect("holding must exist after buy")
            .quantity;

        // Create a schedule with a start_date well in the past — if creation itself
        // removed shares (or eagerly generated), the quantity would drop here.
        svc.create_fee_schedule(
            &account.id,
            asset_id.clone(),
            1_000_000, // 1%
            FeeFrequency::Monthly,
            "2024-01-01".to_string(),
            None,
        )
        .await
        .unwrap();

        let quantity_after = svc
            .get_holdings_for_account(&account.id)
            .await
            .unwrap()
            .iter()
            .find(|h| h.asset_id == asset_id)
            .expect("holding must still exist after create_fee_schedule")
            .quantity;

        // FEE-033 — schedule creation removes no shares.
        assert_eq!(
            quantity_after, quantity_before,
            "create_fee_schedule must not remove any shares (FEE-033)"
        );
    }

    // FEE-031 — ScheduleAlreadyExists when a schedule exists for the (account, asset) pair.
    #[tokio::test]
    async fn fee_031_create_fee_schedule_rejects_duplicate() {
        use crate::context::account::FeeFrequency;
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Dup Sched".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let account = enable_management_fees(&svc, &account).await;

        svc.create_fee_schedule(
            &account.id,
            asset_id.clone(),
            1_000_000,
            FeeFrequency::Monthly,
            "2024-01-01".to_string(),
            None,
        )
        .await
        .unwrap();

        let err = svc
            .create_fee_schedule(
                &account.id,
                asset_id.clone(),
                2_000_000,
                FeeFrequency::Quarterly,
                "2024-01-01".to_string(),
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::ScheduleAlreadyExists),
            "expected ScheduleAlreadyExists, got: {err:?}"
        );
    }

    // FEE-032 — RateNotPositive when annual_rate_percent_micros <= 0.
    #[tokio::test]
    async fn fee_032_create_fee_schedule_rejects_zero_rate() {
        use crate::context::account::FeeFrequency;
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Rate Zero".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let account = enable_management_fees(&svc, &account).await;

        let err = svc
            .create_fee_schedule(
                &account.id,
                asset_id.clone(),
                0,
                FeeFrequency::Monthly,
                "2024-01-01".to_string(),
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::RateNotPositive),
            "expected RateNotPositive, got: {err:?}"
        );
    }

    // FEE-032 — RateAboveHundred when rate > 100_000_000.
    #[tokio::test]
    async fn fee_032_create_fee_schedule_rejects_rate_above_hundred() {
        use crate::context::account::FeeFrequency;
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Rate High".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let account = enable_management_fees(&svc, &account).await;

        let err = svc
            .create_fee_schedule(
                &account.id,
                asset_id.clone(),
                100_000_001,
                FeeFrequency::Monthly,
                "2024-01-01".to_string(),
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::RateAboveHundred),
            "expected RateAboveHundred, got: {err:?}"
        );
    }

    // FEE-032 — EndBeforeStart when end_date <= start_date.
    #[tokio::test]
    async fn fee_032_create_fee_schedule_rejects_end_before_start() {
        use crate::context::account::FeeFrequency;
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "End Before".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let account = enable_management_fees(&svc, &account).await;

        let err = svc
            .create_fee_schedule(
                &account.id,
                asset_id.clone(),
                1_000_000,
                FeeFrequency::Monthly,
                "2024-12-01".to_string(),
                Some("2024-01-01".to_string()),
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::EndBeforeStart),
            "expected EndBeforeStart, got: {err:?}"
        );
    }

    // -------------------------------------------------------------------------
    // FEE-060/061 — update_fee_schedule service method
    // -------------------------------------------------------------------------

    // FEE-060 — update_fee_schedule returns the updated schedule.
    #[tokio::test]
    async fn fee_060_update_fee_schedule_returns_updated_schedule() {
        use crate::context::account::FeeFrequency;
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Update Sched".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let account = enable_management_fees(&svc, &account).await;

        svc.create_fee_schedule(
            &account.id,
            asset_id.clone(),
            1_000_000,
            FeeFrequency::Monthly,
            "2024-01-01".to_string(),
            None,
        )
        .await
        .unwrap();

        let updated = svc
            .update_fee_schedule(
                &account.id,
                &asset_id,
                2_000_000,
                Some("2025-12-31".to_string()),
                true,
            )
            .await
            .unwrap();

        assert_eq!(updated.annual_rate_percent_micros, 2_000_000);
        assert_eq!(updated.end_date, Some("2025-12-31".to_string()));
        assert!(updated.active);
        // FEE-061 — frequency and start_date are NOT changed by update.
        assert!(matches!(updated.frequency, FeeFrequency::Monthly));
        assert_eq!(updated.start_date, "2024-01-01");
    }

    // FEE-060 — ScheduleNotFound when no schedule exists for the pair.
    #[tokio::test]
    async fn fee_060_update_fee_schedule_rejects_missing_schedule() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "NoSched".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();

        let err = svc
            .update_fee_schedule(&account.id, &asset_id, 1_000_000, None, true)
            .await
            .unwrap_err();
        assert!(
            matches!(err, AccountError::ScheduleNotFound),
            "expected ScheduleNotFound, got: {err:?}"
        );
    }

    // -------------------------------------------------------------------------
    // FEE-062 — delete_fee_schedule service method
    // -------------------------------------------------------------------------

    // FEE-062 — delete_fee_schedule succeeds and the schedule is gone.
    #[tokio::test]
    async fn fee_062_delete_fee_schedule_removes_schedule() {
        use crate::context::account::FeeFrequency;
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Del Sched".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let account = enable_management_fees(&svc, &account).await;

        svc.create_fee_schedule(
            &account.id,
            asset_id.clone(),
            1_000_000,
            FeeFrequency::Monthly,
            "2024-01-01".to_string(),
            None,
        )
        .await
        .unwrap();

        svc.delete_fee_schedule(&account.id, &asset_id)
            .await
            .unwrap();

        let found = svc.get_fee_schedule(&account.id, &asset_id).await.unwrap();
        assert!(found.is_none(), "schedule must be absent after delete");
    }

    // FEE-062 — delete_fee_schedule is a no-op when no schedule exists (silent).
    #[tokio::test]
    async fn fee_062_delete_fee_schedule_is_silent_on_missing() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Del None".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();

        // Should not error when no schedule exists.
        svc.delete_fee_schedule(&account.id, &asset_id)
            .await
            .unwrap();
    }

    // -------------------------------------------------------------------------
    // FEE-030 (read) — get_fee_schedule service method
    // -------------------------------------------------------------------------

    // FEE-030 — get_fee_schedule returns None when no schedule exists.
    #[tokio::test]
    async fn fee_030_get_fee_schedule_returns_none_when_absent() {
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Get None".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();

        let found = svc.get_fee_schedule(&account.id, &asset_id).await.unwrap();
        assert!(found.is_none());
    }

    // FEE-030 — get_fee_schedule returns the schedule when it exists.
    #[tokio::test]
    async fn fee_030_get_fee_schedule_returns_schedule_when_present() {
        use crate::context::account::FeeFrequency;
        let pool = make_pool().await;
        let (svc, asset_id) = setup(&pool).await;
        let account = svc
            .create(
                "Get Present".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let account = enable_management_fees(&svc, &account).await;

        svc.create_fee_schedule(
            &account.id,
            asset_id.clone(),
            1_500_000,
            FeeFrequency::Quarterly,
            "2024-01-01".to_string(),
            None,
        )
        .await
        .unwrap();

        let found = svc.get_fee_schedule(&account.id, &asset_id).await.unwrap();
        assert!(found.is_some());
        let schedule = found.unwrap();
        assert_eq!(schedule.annual_rate_percent_micros, 1_500_000);
        assert!(matches!(schedule.frequency, FeeFrequency::Quarterly));
    }
}
