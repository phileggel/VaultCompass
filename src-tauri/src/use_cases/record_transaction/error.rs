/// Typed errors raised by the RecordTransaction use case.
#[derive(Debug, thiserror::Error)]
pub enum RecordTransactionError {
    /// No transaction exists with the requested ID (update/delete path).
    #[error("Transaction not found")]
    TransactionNotFound,
    /// No account exists with the requested ID.
    #[error("Account not found")]
    AccountNotFound,
    /// No asset exists with the requested ID.
    #[error("Asset not found")]
    AssetNotFound,
    /// The transaction type string could not be parsed into a known variant.
    #[error("Unknown transaction type")]
    InvalidType,
    /// Attempt to change the type of an existing transaction.
    #[error("Cannot change transaction type")]
    TypeImmutable,
    /// Attempt to sell shares of an archived asset.
    #[error("Cannot sell an archived asset")]
    ArchivedAssetSell,
    /// Sell requested but the holding has zero available units.
    #[error("No units available to sell")]
    ClosedPosition,
    /// Sell quantity exceeds the units currently held.
    #[error("Quantity ({requested}) exceeds available holding ({available})")]
    Oversell {
        /// Units currently held before the sale.
        available: i64,
        /// Units the transaction attempts to sell.
        requested: i64,
    },
    /// Editing the transaction would leave a later transaction with insufficient units.
    #[error("Editing this transaction would create a cascading oversell")]
    CascadingOversell,
}
