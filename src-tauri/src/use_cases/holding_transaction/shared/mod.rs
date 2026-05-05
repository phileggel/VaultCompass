//! Shared helpers for holding-transaction use cases.

/// Cash Asset seeding helper (CSH-010).
mod ensure_cash_asset;

#[allow(unused_imports)]
pub use ensure_cash_asset::ensure_cash_asset;
