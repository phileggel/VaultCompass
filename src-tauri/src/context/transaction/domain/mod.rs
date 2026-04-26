/// Typed error enums for the transaction domain.
pub mod error;
mod transaction;

pub use error::TransactionDomainError;
pub use transaction::*;
