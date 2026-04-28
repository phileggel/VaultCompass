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
}
