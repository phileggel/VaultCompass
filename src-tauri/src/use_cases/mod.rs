//! Application use cases layer.
//!
//! Cross-cutting application use cases that orchestrate multiple bounded
//! contexts or platform capabilities.

/// Record transaction: create, update, delete transactions and update Holdings (TRX feature).
pub mod record_transaction;
/// Application auto-update: detection, download, and installation (R1–R27).
pub mod update_checker;
