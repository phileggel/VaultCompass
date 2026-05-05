//! Holding-transaction use cases.
//!
//! Cross-context orchestrators for every operation that mutates a `Holding`
//! through a `Transaction`: opening balance, buy, sell, correct, cancel.
//! Each use case injects `AccountService` + `AssetService` and shares
//! `ensure_cash_asset(currency)` (Cash Asset seeding helper, CSH-010).

/// Tauri command handlers for transaction-recording operations.
mod api;
/// Cross-BC orchestrators (one struct per operation).
mod orchestrator;
/// Shared helpers used by multiple orchestrators.
mod shared;

pub use api::*;
pub use orchestrator::{
    BuyHoldingUseCase, CancelTransactionUseCase, CorrectTransactionUseCase, OpenHoldingUseCase,
    SellHoldingUseCase,
};
