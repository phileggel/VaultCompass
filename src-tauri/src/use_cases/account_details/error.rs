/// Typed errors raised by the AccountDetails use case.
#[derive(Debug, thiserror::Error)]
pub enum AccountDetailsError {
    /// No account exists with the requested ID.
    #[error("Account not found")]
    AccountNotFound,
}
