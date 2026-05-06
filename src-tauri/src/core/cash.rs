//! Cross-context constants and helpers for the system Cash Asset / Cash Category
//! introduced by the cash-tracking spec (CSH-010, CSH-014, CSH-017).
//!
//! Lives in `core/` because both `context/account/` and `context/asset/` need to
//! agree on the deterministic ID format without violating the no-cross-context-import
//! rule (B13).

/// Deterministic id of the system Cash Category (CSH-017). Hidden from category
/// management surfaces; seeded lazily by `AssetService::seed_cash_asset`.
pub const SYSTEM_CASH_CATEGORY_ID: &str = "system-cash-category";

/// i18n key used as the Cash Category's label at seed time.
pub const SYSTEM_CASH_CATEGORY_LABEL: &str = "generic.cash";

/// Returns the deterministic id of the system Cash Asset for `currency` (CSH-014).
/// Format: `system-cash-{lowercase_currency}` (e.g. `system-cash-eur`).
pub fn system_cash_asset_id(currency: &str) -> String {
    format!("system-cash-{}", currency.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_currency_in_id() {
        assert_eq!(system_cash_asset_id("EUR"), "system-cash-eur");
        assert_eq!(system_cash_asset_id("usd"), "system-cash-usd");
        assert_eq!(system_cash_asset_id("UsD"), "system-cash-usd");
    }
}
