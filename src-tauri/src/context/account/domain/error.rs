/// Typed errors for the account bounded context.
#[derive(Debug, thiserror::Error)]
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
#[derive(Debug, thiserror::Error)]
pub enum HoldingDomainError {
    /// Holding quantity is negative.
    #[error("Holding quantity cannot be negative")]
    NegativeQuantity,
    /// Holding average_price is negative.
    #[error("Holding average_price cannot be negative")]
    NegativeAveragePrice,
}

/// Typed errors raised by the open_holding operation (TRX-042 through TRX-056).
#[derive(Debug, thiserror::Error)]
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
}

/// Typed errors raised by Account aggregate operations (buy/sell/correct/cancel).
#[derive(Debug, thiserror::Error)]
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
}
