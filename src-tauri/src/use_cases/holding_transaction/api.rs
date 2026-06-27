// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use super::error::{DividendError, OpenHoldingError};
use super::HoldingTransactionUseCase;
use crate::context::account::{AccountError, Transaction};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

// =============================================================================
// Opening Balance — DTO + dedicated error
// =============================================================================

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

// =============================================================================
// Buy / Sell / Correct — DTOs (shared AccountError composite)
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
// Commands
// =============================================================================

/// Seeds a holding directly from a known quantity and total cost (TRX-042, TRX-047).
#[tauri::command]
#[specta::specta]
pub async fn open_holding(
    uc: State<'_, HoldingTransactionUseCase>,
    dto: OpenHoldingDTO,
) -> Result<Transaction, OpenHoldingError> {
    uc.open_holding(
        &dto.account_id,
        dto.asset_id,
        dto.date,
        dto.quantity,
        dto.total_cost,
    )
    .await
}

/// Records a purchase of an asset into an account (TRX-027).
#[tauri::command]
#[specta::specta]
pub async fn buy_holding(
    uc: State<'_, HoldingTransactionUseCase>,
    dto: BuyHoldingDTO,
) -> Result<Transaction, AccountError> {
    uc.buy_holding(
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
}

/// Records a sale of an asset from an account (SEL-012, SEL-021, SEL-023, SEL-024).
#[tauri::command]
#[specta::specta]
pub async fn sell_holding(
    uc: State<'_, HoldingTransactionUseCase>,
    dto: SellHoldingDTO,
) -> Result<Transaction, AccountError> {
    uc.sell_holding(
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
}

/// Corrects an existing transaction and recalculates the affected holding (TRX-031).
#[tauri::command]
#[specta::specta]
pub async fn correct_transaction(
    uc: State<'_, HoldingTransactionUseCase>,
    id: String,
    account_id: String,
    dto: CorrectTransactionDTO,
) -> Result<Transaction, AccountError> {
    uc.correct_transaction(
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
}

/// Cancels a transaction and recalculates (or removes) the associated holding (TRX-034).
#[tauri::command]
#[specta::specta]
pub async fn cancel_transaction(
    uc: State<'_, HoldingTransactionUseCase>,
    id: String,
    account_id: String,
) -> Result<(), AccountError> {
    uc.cancel_transaction(&account_id, &id).await
}

// =============================================================================
// Cash Transactions — DTOs + dedicated errors (CSH-022 / CSH-032)
// =============================================================================

/// Parameters for recording a cash deposit (CSH-020).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DepositDTO {
    /// Account receiving the cash.
    pub account_id: String,
    /// Transaction date (YYYY-MM-DD).
    pub date: String,
    /// Deposited amount in account currency (micro-units); strictly positive (CSH-021).
    pub amount_micros: i64,
    /// Optional user note.
    pub note: Option<String>,
}

/// Parameters for recording a cash withdrawal (CSH-030).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct WithdrawalDTO {
    /// Account from which to withdraw cash.
    pub account_id: String,
    /// Transaction date (YYYY-MM-DD).
    pub date: String,
    /// Withdrawn amount in account currency (micro-units); strictly positive (CSH-031).
    pub amount_micros: i64,
    /// Optional user note.
    pub note: Option<String>,
}

/// Records a cash deposit into an account (CSH-022).
#[tauri::command]
#[specta::specta]
pub async fn record_deposit(
    uc: State<'_, HoldingTransactionUseCase>,
    dto: DepositDTO,
) -> Result<Transaction, AccountError> {
    uc.record_deposit(&dto.account_id, dto.date, dto.amount_micros, dto.note)
        .await
}

/// Records a cash withdrawal from an account (CSH-032).
#[tauri::command]
#[specta::specta]
pub async fn record_withdrawal(
    uc: State<'_, HoldingTransactionUseCase>,
    dto: WithdrawalDTO,
) -> Result<Transaction, AccountError> {
    uc.record_withdrawal(&dto.account_id, dto.date, dto.amount_micros, dto.note)
        .await
}

// =============================================================================
// Dividend — DTO + command (DIV-020/023)
// =============================================================================

/// Parameters for recording a cash dividend attributed to a held asset (DIV-020).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct DividendDTO {
    /// Account receiving the dividend.
    pub account_id: String,
    /// The paying asset — must be actively held (quantity > 0) and not a Cash Asset (DIV-011).
    pub asset_id: String,
    /// Business date the dividend was received (YYYY-MM-DD, DIV-021).
    pub date: String,
    /// Net dividend in the asset's native currency (micro-units, strictly positive, DIV-021).
    pub amount_micros: i64,
    /// Asset→account conversion rate (micro-units, strictly positive; 1_000_000 when currencies match, DIV-022).
    pub exchange_rate: i64,
    /// Optional user note.
    pub note: Option<String>,
}

/// Records a cash dividend attributed to a held asset (DIV-023).
#[tauri::command]
#[specta::specta]
pub async fn record_dividend(
    uc: State<'_, HoldingTransactionUseCase>,
    dto: DividendDTO,
) -> Result<Transaction, DividendError> {
    uc.record_dividend(
        &dto.account_id,
        dto.asset_id,
        dto.date,
        dto.amount_micros,
        dto.exchange_rate,
        dto.note,
    )
    .await
}

// =============================================================================
// Free Shares — DTO + command (FSD-020/022)
// =============================================================================

/// Parameters for recording a zero-cost free-share distribution from a held
/// distributing asset (FSD-020). No amount, no unit price, no exchange rate,
/// no fees — no money changes hands.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct FreeSharesDTO {
    /// Account whose holding receives the free shares.
    pub account_id: String,
    /// The distributing asset — must be actively held (quantity > 0) and not a Cash Asset (FSD-011).
    pub asset_id: String,
    /// Business date the shares were received (YYYY-MM-DD, FSD-021).
    pub date: String,
    /// Number of free shares received (micro-units, strictly positive, FSD-021).
    pub quantity: i64,
    /// Optional user note.
    pub note: Option<String>,
}

/// Records a zero-cost free-share distribution attributed to a held asset (FSD-022).
#[tauri::command]
#[specta::specta]
pub async fn record_free_shares(
    uc: State<'_, HoldingTransactionUseCase>,
    dto: FreeSharesDTO,
) -> Result<Transaction, super::error::FreeSharesError> {
    uc.record_free_shares(
        &dto.account_id,
        dto.asset_id,
        dto.date,
        dto.quantity,
        dto.note,
    )
    .await
}
