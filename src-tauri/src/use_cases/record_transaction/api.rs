// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use super::error::RecordTransactionError;
use super::orchestrator::{CreateTransactionDTO, RecordTransactionUseCase};
// TransactionDomainError lives in the transaction bounded context; importing it here is acceptable
// because this use-case layer sits above that context and aggregates both error sources into one
// boundary enum — it does not create a circular dependency.
use crate::context::transaction::{Transaction, TransactionDomainError};
use tauri::State;

// --- Boundary error ---

/// Typed error returned to the frontend for transaction commands.
#[derive(Debug, serde::Serialize, specta::Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum TransactionCommandError {
    /// No transaction exists with the requested ID.
    #[error("Transaction not found")]
    TransactionNotFound,
    /// No account exists with the requested ID.
    #[error("Account not found")]
    AccountNotFound,
    /// No asset exists with the requested ID.
    #[error("Asset not found")]
    AssetNotFound,
    /// The transaction type string is not a known variant.
    #[error("Unknown transaction type")]
    InvalidType,
    /// Attempt to change the type of an existing transaction.
    #[error("Cannot change transaction type")]
    TypeImmutable,
    /// Attempt to sell shares of an archived asset.
    #[error("Cannot sell an archived asset")]
    ArchivedAssetSell,
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
    if let Some(err) = e.downcast_ref::<RecordTransactionError>() {
        return match err {
            RecordTransactionError::TransactionNotFound => {
                TransactionCommandError::TransactionNotFound
            }
            RecordTransactionError::AccountNotFound => TransactionCommandError::AccountNotFound,
            RecordTransactionError::AssetNotFound => TransactionCommandError::AssetNotFound,
            RecordTransactionError::InvalidType => TransactionCommandError::InvalidType,
            RecordTransactionError::TypeImmutable => TransactionCommandError::TypeImmutable,
            RecordTransactionError::ArchivedAssetSell => TransactionCommandError::ArchivedAssetSell,
            RecordTransactionError::ClosedPosition => TransactionCommandError::ClosedPosition,
            RecordTransactionError::Oversell {
                available,
                requested,
            } => TransactionCommandError::Oversell {
                available: *available,
                requested: *requested,
            },
            RecordTransactionError::CascadingOversell => TransactionCommandError::CascadingOversell,
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
    tracing::error!(err = ?e, "unexpected error in transaction command");
    TransactionCommandError::Unknown
}

// --- Commands ---

/// Creates a new purchase transaction and updates the Holding atomically (TRX-027).
#[tauri::command]
#[specta::specta]
pub async fn add_transaction(
    state: State<'_, RecordTransactionUseCase>,
    dto: CreateTransactionDTO,
) -> Result<Transaction, TransactionCommandError> {
    state
        .create_transaction(dto)
        .await
        .map_err(to_transaction_error)
}

/// Updates an existing transaction and recalculates the affected Holding(s) (TRX-031, TRX-032).
#[tauri::command]
#[specta::specta]
pub async fn update_transaction(
    state: State<'_, RecordTransactionUseCase>,
    id: String,
    dto: CreateTransactionDTO,
) -> Result<Transaction, TransactionCommandError> {
    state
        .update_transaction(id, dto)
        .await
        .map_err(to_transaction_error)
}

/// Deletes a transaction and recalculates (or removes) the associated Holding (TRX-034).
#[tauri::command]
#[specta::specta]
pub async fn delete_transaction(
    state: State<'_, RecordTransactionUseCase>,
    id: String,
) -> Result<(), TransactionCommandError> {
    state
        .delete_transaction(&id)
        .await
        .map_err(to_transaction_error)
}

/// Retrieves all transactions for an account/asset pair.
#[tauri::command]
#[specta::specta]
pub async fn get_transactions(
    state: State<'_, RecordTransactionUseCase>,
    account_id: String,
    asset_id: String,
) -> Result<Vec<Transaction>, TransactionCommandError> {
    state
        .get_transactions(&account_id, &asset_id)
        .await
        .map_err(to_transaction_error)
}
