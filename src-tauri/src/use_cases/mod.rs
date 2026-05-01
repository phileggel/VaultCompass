//! Application use cases layer.
//!
//! Cross-cutting application use cases that orchestrate multiple bounded
//! contexts or platform capabilities.

/// Account Deletion: pre-deletion summary of holdings and transactions (ACC-020).
pub mod account_deletion;
/// Account Details: cross-context read of holdings + asset metadata (ACD feature).
pub mod account_details;
/// Archive asset: guards archiving against active holdings across bounded contexts (OQ-6).
pub mod archive_asset;
/// Asset Web Lookup: OpenFIGI search to pre-fill the Add Asset form (WEB).
pub mod asset_web_lookup;
/// Delete asset: guards hard-deletion against existing transactions.
pub mod delete_asset;
/// Application auto-update: detection, download, and installation (R1–R27).
pub mod update_checker;
