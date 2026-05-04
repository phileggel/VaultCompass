// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use super::OpenHoldingUseCase;
use crate::context::account::{
    AccountDomainError, OpeningBalanceDomainError, Transaction, TransactionDomainError,
};
use crate::core::logger::BACKEND;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

/// Parameters for recording an opening balance for an asset in an account (TRX-042).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OpenHoldingDTO {
    /// Account where the opening balance is recorded.
    pub account_id: String,
    /// Financial asset being seeded.
    pub asset_id: String,
    /// Date of the opening balance (YYYY-MM-DD).
    pub date: String,
    /// Quantity in micro-units; strictly positive (TRX-044).
    pub quantity: i64,
    /// Total cost paid in account currency (micro-units); strictly positive (TRX-045).
    pub total_cost: i64,
}

/// Typed error returned to the frontend for the open_holding command.
#[derive(Debug, Serialize, Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum OpenHoldingCommandError {
    /// No account exists with the requested ID.
    #[error("Account not found")]
    AccountNotFound,
    /// No asset exists with the requested ID.
    #[error("Asset not found")]
    AssetNotFound,
    /// Asset is archived — cannot open a holding (TRX-050).
    #[error("Cannot open a holding for an archived asset")]
    ArchivedAsset,
    /// Total cost is zero or negative (TRX-045).
    #[error("Total cost must be strictly positive")]
    InvalidTotalCost,
    /// Quantity is zero or negative (TRX-044).
    #[error("Quantity must be strictly positive")]
    QuantityNotPositive,
    /// Date string could not be parsed as YYYY-MM-DD.
    #[error("Invalid date format — expected YYYY-MM-DD")]
    InvalidDate,
    /// Transaction date is in the future.
    #[error("Transaction date cannot be in the future")]
    DateInFuture,
    /// Transaction date is before 1900-01-01.
    #[error("Transaction date cannot be before 1900-01-01")]
    DateTooOld,
    /// An unexpected server-side error occurred.
    #[error("An unexpected error occurred")]
    Unknown,
}

fn to_open_holding_error(e: anyhow::Error) -> OpenHoldingCommandError {
    if let Some(err) = e.downcast_ref::<OpeningBalanceDomainError>() {
        return match err {
            OpeningBalanceDomainError::InvalidTotalCost => {
                OpenHoldingCommandError::InvalidTotalCost
            }
            OpeningBalanceDomainError::AssetNotFound => OpenHoldingCommandError::AssetNotFound,
            OpeningBalanceDomainError::ArchivedAsset => OpenHoldingCommandError::ArchivedAsset,
        };
    }
    if let Some(err) = e.downcast_ref::<TransactionDomainError>() {
        return match err {
            TransactionDomainError::InvalidDate => OpenHoldingCommandError::InvalidDate,
            TransactionDomainError::DateInFuture => OpenHoldingCommandError::DateInFuture,
            TransactionDomainError::DateTooOld => OpenHoldingCommandError::DateTooOld,
            TransactionDomainError::QuantityNotPositive => {
                OpenHoldingCommandError::QuantityNotPositive
            }
            // These four variants require a user-supplied unit_price, fees, or exchange_rate.
            // open_holding computes all three as constants, so they cannot fire in practice.
            // Enumerated explicitly (not wildcarded) so a future regression triggers a compile error.
            TransactionDomainError::UnitPriceNegative
            | TransactionDomainError::FeesNegative
            | TransactionDomainError::ExchangeRateNotPositive
            | TransactionDomainError::TotalAmountNotPositive => {
                tracing::error!(target: BACKEND, err = ?err, "BUG: impossible TransactionDomainError in open_holding");
                OpenHoldingCommandError::Unknown
            }
        };
    }
    if let Some(err) = e.downcast_ref::<AccountDomainError>() {
        return match err {
            AccountDomainError::AccountNotFound(_) => OpenHoldingCommandError::AccountNotFound,
            // NameEmpty / NameAlreadyExists / InvalidCurrency cannot fire from open_holding.
            _ => {
                tracing::error!(target: BACKEND, err = ?err, "BUG: unexpected AccountDomainError in open_holding command");
                OpenHoldingCommandError::Unknown
            }
        };
    }
    tracing::error!(target: BACKEND, err = ?e, "unexpected error in open_holding command");
    OpenHoldingCommandError::Unknown
}

/// Seeds a holding directly from a known quantity and total cost (TRX-042, TRX-047).
#[tauri::command]
#[specta::specta]
pub async fn open_holding(
    uc: State<'_, OpenHoldingUseCase>,
    dto: OpenHoldingDTO,
) -> Result<Transaction, OpenHoldingCommandError> {
    uc.open_holding(
        &dto.account_id,
        dto.asset_id,
        dto.date,
        dto.quantity,
        dto.total_cost,
    )
    .await
    .map_err(to_open_holding_error)
}
