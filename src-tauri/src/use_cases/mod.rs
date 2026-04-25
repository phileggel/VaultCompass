//! Application use cases layer.
//!
//! Cross-cutting application use cases that orchestrate multiple bounded
//! contexts or platform capabilities.

/// Account Details: cross-context read of holdings + asset metadata (ACD feature).
pub mod account_details;
/// Archive asset: guards archiving against active holdings across bounded contexts (OQ-6).
pub mod archive_asset;
/// Delete asset: guards hard-deletion against existing transactions.
pub mod delete_asset;
/// Record transaction: create, update, delete transactions and update Holdings (TRX feature).
pub mod record_transaction;
/// Application auto-update: detection, download, and installation (R1–R27).
pub mod update_checker;
