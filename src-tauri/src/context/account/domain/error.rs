use super::transaction_error::TransactionDomainError;

/// Typed errors for the account bounded context.
///
/// All domain error enums in this module derive `serde::Serialize` + `specta::Type` +
/// `#[serde(tag = "code")]` so they can be exposed verbatim at the Tauri boundary.
/// Boundary error types compose them via untagged unions to avoid redefining variants
/// (review feedback on PR #5).
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type, Clone)]
#[serde(tag = "code")]
pub enum AccountDomainError {
    /// Account name is empty or whitespace-only.
    #[error("Account name cannot be empty")]
    NameEmpty,
    /// An account with the same name (case-insensitive) already exists.
    #[error("An account with this name already exists")]
    NameAlreadyExists,
    /// The currency string is not a valid ISO 4217 code.
    #[error("Invalid currency code: {0}")]
    InvalidCurrency(String),
    /// No account exists with the requested ID.
    #[error("Account not found: {0}")]
    AccountNotFound(String),
}

/// Typed errors raised during holding validation.
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type, Clone)]
#[serde(tag = "code")]
pub enum HoldingDomainError {
    /// Holding quantity is negative.
    #[error("Holding quantity cannot be negative")]
    NegativeQuantity,
    /// Holding average_price is negative.
    #[error("Holding average_price cannot be negative")]
    NegativeAveragePrice,
}

/// Typed errors raised by the open_holding operation (TRX-042 through TRX-056).
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type, Clone)]
#[serde(tag = "code")]
pub enum OpeningBalanceDomainError {
    /// total_cost was zero or negative (TRX-045).
    #[error("Total cost must be strictly positive")]
    InvalidTotalCost,
    /// No asset with the given ID exists (TRX-056).
    #[error("Asset not found")]
    AssetNotFound,
    /// The target asset is archived — no auto-unarchive (TRX-050).
    #[error("Cannot open a holding for an archived asset")]
    ArchivedAsset,
    /// Attempt to record an OpeningBalance against a Cash Asset (CSH-061).
    /// User must record initial cash via `record_deposit` instead.
    #[error("Opening balance cannot be recorded against a cash asset; use record_deposit")]
    OpeningBalanceOnCashAsset,
}

/// Typed errors raised by Account aggregate operations (buy/sell/correct/cancel/cash).
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type, Clone)]
#[serde(tag = "code")]
pub enum AccountOperationError {
    /// Attempt to sell an asset with no open position (quantity = 0).
    #[error("No units available to sell (closed position)")]
    ClosedPosition,
    /// Sell quantity exceeds the currently held units.
    #[error("Oversell: requested {requested} exceeds available {available}")]
    Oversell {
        /// Units currently held before the sale.
        available: i64,
        /// Units the operation attempts to sell.
        requested: i64,
    },
    /// Correcting a transaction would leave a later sell with insufficient units.
    #[error("Editing this transaction would create a cascading oversell")]
    CascadingOversell,
    /// No transaction with the given ID exists within this account.
    #[error("Transaction not found")]
    TransactionNotFound,
    /// Attempted cash debit (or chronological replay step) would drive the cash holding strictly negative (CSH-080).
    #[error("Insufficient cash: current balance {current_balance_micros} {currency}")]
    InsufficientCash {
        /// Cash holding's running balance at the point of rejection (micro-units, account currency).
        current_balance_micros: i64,
        /// ISO 4217 currency code of the offending account's cash holding.
        currency: String,
    },
    /// Deposit / Withdrawal amount was zero or negative (CSH-021, CSH-031).
    #[error("Amount must be greater than 0")]
    AmountNotPositive,
}

/// Typed error returned by `Account::record_deposit` / `Account::record_withdrawal`.
///
/// Unifies the two domain error sources cash-recording methods can fail with —
/// `AccountOperationError` (aggregate-level invariants like InsufficientCash,
/// AmountNotPositive) and `TransactionDomainError` (invalid date variants raised
/// by `Transaction::new`). `#[from]` lets `?` convert both source types
/// automatically. The boundary layer downcasts this enum and surfaces its inner
/// variants to the FE without redefinition.
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type, Clone)]
#[serde(untagged)]
pub enum CashOperationError {
    /// Aggregate-level operation error (AmountNotPositive, InsufficientCash).
    #[error(transparent)]
    Operation(#[from] AccountOperationError),
    /// Transaction validation error (invalid date, date in future, etc.).
    #[error(transparent)]
    Validation(#[from] TransactionDomainError),
}
