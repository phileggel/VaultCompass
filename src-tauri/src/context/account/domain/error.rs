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
