// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use super::domain::{Account, UpdateFrequency};
use crate::context::account::{
    AccountDomainError, AccountOperationError, Transaction, TransactionDomainError,
};
use crate::core::logger::BACKEND;
use crate::AppState;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

// --- DTOs ---

/// Parameters for creating a new account.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct CreateAccountDTO {
    /// Display name.
    pub name: String,
    /// ISO 4217 currency code.
    pub currency: String,
    /// Update frequency.
    pub update_frequency: UpdateFrequency,
}

/// Parameters for updating an existing account.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct UpdateAccountDTO {
    /// Target account ID.
    pub id: String,
    /// New display name.
    pub name: String,
    /// ISO 4217 currency code.
    pub currency: String,
    /// New update frequency.
    pub update_frequency: UpdateFrequency,
}

// --- Boundary error ---

/// Typed error returned to the frontend for account commands.
#[derive(Debug, Serialize, Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum AccountCommandError {
    /// Account name is empty or whitespace-only.
    #[error("Account name cannot be empty")]
    NameEmpty,
    /// An account with the same name already exists.
    #[error("An account with this name already exists")]
    NameAlreadyExists,
    /// The currency string is not a valid ISO 4217 code.
    #[error("Invalid currency code")]
    InvalidCurrency,
    /// An unexpected server-side error occurred.
    #[error("An unexpected error occurred")]
    Unknown,
}

fn to_account_error(e: anyhow::Error) -> AccountCommandError {
    if let Some(err) = e.downcast_ref::<AccountDomainError>() {
        match err {
            AccountDomainError::NameEmpty => AccountCommandError::NameEmpty,
            AccountDomainError::NameAlreadyExists => AccountCommandError::NameAlreadyExists,
            AccountDomainError::InvalidCurrency(_) => AccountCommandError::InvalidCurrency,
        }
    } else {
        tracing::error!(target: BACKEND, err = ?e, "unexpected error in account command");
        AccountCommandError::Unknown
    }
}

// --- Commands ---

/// Retrieves all accounts.
#[tauri::command]
#[specta::specta]
pub async fn get_accounts(state: State<'_, AppState>) -> Result<Vec<Account>, AccountCommandError> {
    state
        .account_service
        .get_all()
        .await
        .map_err(to_account_error)
}

/// Adds a new account.
#[tauri::command]
#[specta::specta]
pub async fn add_account(
    state: State<'_, AppState>,
    dto: CreateAccountDTO,
) -> Result<Account, AccountCommandError> {
    state
        .account_service
        .create(dto.name, dto.currency, dto.update_frequency)
        .await
        .map_err(to_account_error)
}

/// Updates an existing account.
#[tauri::command]
#[specta::specta]
pub async fn update_account(
    state: State<'_, AppState>,
    dto: UpdateAccountDTO,
) -> Result<Account, AccountCommandError> {
    state
        .account_service
        .update(dto.id, dto.name, dto.currency, dto.update_frequency)
        .await
        .map_err(to_account_error)
}

/// Deletes an account.
#[tauri::command]
#[specta::specta]
pub async fn delete_account(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AccountCommandError> {
    state
        .account_service
        .delete(&id)
        .await
        .map_err(to_account_error)
}

/// Returns the distinct asset IDs that have transactions for the given account (TXL-013).
#[tauri::command]
#[specta::specta]
pub async fn get_asset_ids_for_account(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<String>, AccountCommandError> {
    state
        .account_service
        .get_asset_ids_for_account(&account_id)
        .await
        .map_err(|e| {
            tracing::error!(err = ?e, "unexpected error in get_asset_ids_for_account");
            AccountCommandError::Unknown
        })
}

// =============================================================================
// Holding operation DTOs
// =============================================================================

/// Parameters for recording a purchase of an asset into an account.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BuyHoldingDTO {
    /// Account where the purchase is recorded.
    pub account_id: String,
    /// Financial asset being purchased.
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

/// Parameters for recording a sale of an asset from an account.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SellHoldingDTO {
    /// Account where the sale is recorded.
    pub account_id: String,
    /// Financial asset being sold.
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

/// Parameters for correcting an existing transaction.
/// `account_id` and `asset_id` are immutable — taken from the existing transaction.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CorrectTransactionDTO {
    /// Corrected transaction date (YYYY-MM-DD).
    pub date: String,
    /// Corrected quantity in micro-units.
    pub quantity: i64,
    /// Corrected unit price in asset currency (micro-units).
    pub unit_price: i64,
    /// Corrected exchange rate asset→account currency (micro-units).
    pub exchange_rate: i64,
    /// Corrected fees in account currency (micro-units).
    pub fees: i64,
    /// Optional user note.
    pub note: Option<String>,
}

// =============================================================================
// Holding operation boundary error
// =============================================================================

/// Typed error returned to the frontend for holding operation commands.
#[derive(Debug, serde::Serialize, specta::Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum TransactionCommandError {
    /// No transaction exists with the requested ID.
    #[error("Transaction not found")]
    TransactionNotFound,
    /// No account exists with the requested ID.
    #[error("Account not found")]
    AccountNotFound,
    /// Sell requested but holding has zero available units.
    #[error("No units available to sell")]
    ClosedPosition,
    /// Sell quantity exceeds currently held units.
    #[error("Oversell: requested exceeds available")]
    Oversell {
        /// Units currently held before the sale.
        available: i64,
        /// Units the transaction attempts to sell.
        requested: i64,
    },
    /// Editing would leave a later transaction with insufficient units.
    #[error("Editing this transaction would create a cascading oversell")]
    CascadingOversell,
    /// Date string could not be parsed as YYYY-MM-DD.
    #[error("Invalid date format — expected YYYY-MM-DD")]
    InvalidDate,
    /// Transaction date is in the future.
    #[error("Transaction date cannot be in the future")]
    DateInFuture,
    /// Transaction date is before 1900-01-01.
    #[error("Transaction date cannot be before 1900-01-01")]
    DateTooOld,
    /// Quantity is zero or negative.
    #[error("Quantity must be strictly positive")]
    QuantityNotPositive,
    /// Unit price is negative.
    #[error("Unit price cannot be negative")]
    UnitPriceNegative,
    /// Fees amount is negative.
    #[error("Fees cannot be negative")]
    FeesNegative,
    /// Exchange rate is zero or negative.
    #[error("Exchange rate must be strictly positive")]
    ExchangeRateNotPositive,
    /// Total amount is zero or negative.
    #[error("Total amount must be strictly positive")]
    TotalAmountNotPositive,
    /// An unexpected server-side error occurred.
    #[error("An unexpected error occurred")]
    Unknown,
}

fn to_transaction_error(e: anyhow::Error) -> TransactionCommandError {
    if let Some(err) = e.downcast_ref::<AccountOperationError>() {
        return match err {
            AccountOperationError::ClosedPosition => TransactionCommandError::ClosedPosition,
            AccountOperationError::Oversell {
                available,
                requested,
            } => TransactionCommandError::Oversell {
                available: *available,
                requested: *requested,
            },
            AccountOperationError::CascadingOversell => TransactionCommandError::CascadingOversell,
            AccountOperationError::TransactionNotFound => {
                TransactionCommandError::TransactionNotFound
            }
        };
    }
    if let Some(err) = e.downcast_ref::<TransactionDomainError>() {
        return match err {
            TransactionDomainError::InvalidDate => TransactionCommandError::InvalidDate,
            TransactionDomainError::DateInFuture => TransactionCommandError::DateInFuture,
            TransactionDomainError::DateTooOld => TransactionCommandError::DateTooOld,
            TransactionDomainError::QuantityNotPositive => {
                TransactionCommandError::QuantityNotPositive
            }
            TransactionDomainError::UnitPriceNegative => TransactionCommandError::UnitPriceNegative,
            TransactionDomainError::FeesNegative => TransactionCommandError::FeesNegative,
            TransactionDomainError::ExchangeRateNotPositive => {
                TransactionCommandError::ExchangeRateNotPositive
            }
            TransactionDomainError::TotalAmountNotPositive => {
                TransactionCommandError::TotalAmountNotPositive
            }
        };
    }
    // "account not found" is surfaced as a plain anyhow error from the service
    if e.to_string().contains("account not found") {
        return TransactionCommandError::AccountNotFound;
    }
    tracing::error!(target: BACKEND, err = ?e, "unexpected error in transaction command");
    TransactionCommandError::Unknown
}

// =============================================================================
// Holding operation commands
// =============================================================================

/// Records a purchase of an asset into an account (TRX-027).
#[tauri::command]
#[specta::specta]
pub async fn buy_holding(
    state: State<'_, AppState>,
    dto: BuyHoldingDTO,
) -> Result<Transaction, TransactionCommandError> {
    state
        .account_service
        .buy_holding(
            &dto.account_id,
            dto.asset_id,
            dto.date,
            dto.quantity,
            dto.unit_price,
            dto.exchange_rate,
            dto.fees,
            dto.note,
        )
        .await
        .map_err(to_transaction_error)
}

/// Records a sale of an asset from an account (SEL-012, SEL-021, SEL-023, SEL-024).
#[tauri::command]
#[specta::specta]
pub async fn sell_holding(
    state: State<'_, AppState>,
    dto: SellHoldingDTO,
) -> Result<Transaction, TransactionCommandError> {
    state
        .account_service
        .sell_holding(
            &dto.account_id,
            dto.asset_id,
            dto.date,
            dto.quantity,
            dto.unit_price,
            dto.exchange_rate,
            dto.fees,
            dto.note,
        )
        .await
        .map_err(to_transaction_error)
}

/// Corrects an existing transaction and recalculates the affected holding (TRX-031).
#[tauri::command]
#[specta::specta]
pub async fn correct_transaction(
    state: State<'_, AppState>,
    id: String,
    account_id: String,
    dto: CorrectTransactionDTO,
) -> Result<Transaction, TransactionCommandError> {
    state
        .account_service
        .correct_transaction(
            &account_id,
            &id,
            dto.date,
            dto.quantity,
            dto.unit_price,
            dto.exchange_rate,
            dto.fees,
            dto.note,
        )
        .await
        .map_err(to_transaction_error)
}

/// Cancels a transaction and recalculates (or removes) the associated holding (TRX-034).
#[tauri::command]
#[specta::specta]
pub async fn cancel_transaction(
    state: State<'_, AppState>,
    id: String,
    account_id: String,
) -> Result<(), TransactionCommandError> {
    state
        .account_service
        .cancel_transaction(&account_id, &id)
        .await
        .map_err(to_transaction_error)
}

/// Retrieves all transactions for an account/asset pair (TRX-036).
#[tauri::command]
#[specta::specta]
pub async fn get_transactions(
    state: State<'_, AppState>,
    account_id: String,
    asset_id: String,
) -> Result<Vec<Transaction>, TransactionCommandError> {
    state
        .account_service
        .get_transactions(&account_id, &asset_id)
        .await
        .map_err(to_transaction_error)
}
