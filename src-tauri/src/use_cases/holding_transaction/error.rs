use crate::context::account::{
    AccountApplicationError, OpeningBalanceDomainError, TransactionDomainError,
};

/// Application-layer rejections specific to the `record_dividend` use case —
/// cross-BC asset and holding checks performed by the orchestrator before
/// delegating to `AccountService::record_dividend`.
///
/// Tagged with `#[serde(tag = "code")]` so it serializes verbatim across the
/// Tauri boundary into a flat `{ code: "..." }` shape.
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type, Clone)]
#[serde(tag = "code")]
pub enum DividendApplicationError {
    /// No asset exists with the requested ID (DIV-011).
    #[error("Asset not found")]
    AssetNotFound,
    /// The asset is not currently held (quantity = 0 or no holding) (DIV-011).
    #[error("Asset is not currently held in this account")]
    AssetNotHeld,
    /// Target asset is a system Cash Asset — dividends must be on non-cash
    /// holdings (DIV-011).
    #[error("Dividends cannot be recorded against a cash asset")]
    DividendOnCashAsset,
}

/// Use-case composite for the **record dividend** failure surface.
///
/// Each leaf lives in its rightful layer:
/// - `AccountApplicationError` — application layer, raises `AccountNotFound`
///   and `DatabaseError`.
/// - `DividendApplicationError` — use-case-owned (this file), raises the
///   cross-BC asset/holding checks.
/// - `TransactionDomainError` — domain layer, raises date / amount / rate
///   validation errors.
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type)]
#[serde(untagged)]
pub enum DividendError {
    /// Account-side rejection (`AccountNotFound`, `DatabaseError`).
    #[error(transparent)]
    Application(#[from] AccountApplicationError),
    /// Use-case-layer rejection (cross-BC asset checks).
    #[error(transparent)]
    UseCase(#[from] DividendApplicationError),
    /// Transaction-factory / domain validation rejection.
    #[error(transparent)]
    Validation(#[from] TransactionDomainError),
}

#[cfg(test)]
mod dividend_error_wire_tests {
    use super::*;

    /// error-model.md — every `DividendError` variant must serialize to a flat
    /// object carrying a string `code` (guards the `#[serde(untagged)]`
    /// null-collapse regression across all three leaves).
    #[test]
    fn each_variant_emits_a_code() {
        let cases: Vec<DividendError> = vec![
            AccountApplicationError::AccountNotFound {
                account_id: "acc-1".to_string(),
            }
            .into(),
            DividendApplicationError::AssetNotFound.into(),
            DividendApplicationError::AssetNotHeld.into(),
            DividendApplicationError::DividendOnCashAsset.into(),
            TransactionDomainError::AmountNotPositive.into(),
            TransactionDomainError::ExchangeRateNotPositive.into(),
        ];
        for err in cases {
            let value = serde_json::to_value(&err).expect("serialize DividendError");
            assert!(
                value.get("code").and_then(|c| c.as_str()).is_some(),
                "DividendError variant did not emit a string `code`: {value}"
            );
        }
    }
}

/// Application-layer rejections specific to the `record_free_shares` use case —
/// cross-BC asset and holding checks performed by the orchestrator before
/// delegating to `AccountService::record_free_shares`.
///
/// Tagged with `#[serde(tag = "code")]` so it serializes verbatim across the
/// Tauri boundary into a flat `{ code: "..." }` shape.
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type, Clone)]
#[serde(tag = "code")]
pub enum FreeSharesApplicationError {
    /// No asset exists with the requested ID (FSD-011).
    #[error("Asset not found")]
    AssetNotFound,
    /// The asset is not currently held (quantity = 0 or no holding) (FSD-011).
    #[error("Asset is not currently held in this account")]
    AssetNotHeld,
    /// Target asset is a system Cash Asset — free shares must be on non-cash
    /// holdings (FSD-011).
    #[error("Free shares cannot be recorded against a cash asset")]
    FreeSharesOnCashAsset,
}

/// Use-case composite for the **record free shares** failure surface.
///
/// Each leaf lives in its rightful layer:
/// - `AccountApplicationError` — application layer, raises `AccountNotFound`
///   and `DatabaseError`.
/// - `FreeSharesApplicationError` — use-case-owned (this file), raises the
///   cross-BC asset/holding checks.
/// - `TransactionDomainError` — domain layer, raises date / quantity
///   validation errors.
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type)]
#[serde(untagged)]
pub enum FreeSharesError {
    /// Account-side rejection (`AccountNotFound`, `DatabaseError`).
    #[error(transparent)]
    Application(#[from] AccountApplicationError),
    /// Use-case-layer rejection (cross-BC asset checks).
    #[error(transparent)]
    UseCase(#[from] FreeSharesApplicationError),
    /// Transaction-factory / domain validation rejection.
    #[error(transparent)]
    Validation(#[from] TransactionDomainError),
}

#[cfg(test)]
mod free_shares_error_wire_tests {
    use super::*;

    /// error-model.md — every `FreeSharesError` variant must serialize to a flat
    /// object carrying a string `code` (guards the `#[serde(untagged)]`
    /// null-collapse regression across all three leaves).
    #[test]
    fn each_variant_emits_a_code() {
        // FSD-011 — all eligibility error variants must serialize correctly
        let cases: Vec<FreeSharesError> = vec![
            AccountApplicationError::AccountNotFound {
                account_id: "acc-1".to_string(),
            }
            .into(),
            FreeSharesApplicationError::AssetNotFound.into(),
            FreeSharesApplicationError::AssetNotHeld.into(),
            FreeSharesApplicationError::FreeSharesOnCashAsset.into(),
            TransactionDomainError::QuantityNotPositive.into(),
            TransactionDomainError::DateInFuture.into(),
        ];
        for err in cases {
            let value = serde_json::to_value(&err).expect("serialize FreeSharesError");
            assert!(
                value.get("code").and_then(|c| c.as_str()).is_some(),
                "FreeSharesError variant did not emit a string `code`: {value}"
            );
        }
    }
}

/// Application-layer rejections specific to the `open_holding` use case —
/// cross-BC asset checks performed by the orchestrator before delegating to
/// `AccountService::open_holding`.
///
/// Tagged with `#[serde(tag = "code")]` so it serializes verbatim across the
/// Tauri boundary into a flat `{ code: "..." }` shape.
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type, Clone)]
#[serde(tag = "code")]
pub enum OpenHoldingApplicationError {
    /// No asset exists with the requested ID (TRX-056).
    #[error("Asset not found")]
    AssetNotFound,
    /// Target asset is archived — cannot open a holding (TRX-050).
    /// The orchestrator does not auto-unarchive; the caller must unarchive
    /// explicitly through the asset BC first.
    #[error("Cannot open a holding for an archived asset")]
    ArchivedAsset,
    /// Target asset is a system Cash Asset (CSH-061). Initial cash should be
    /// recorded via `record_deposit`, which goes through the cash-recording
    /// path and lazy-creates the Cash Holding.
    #[error("Opening balance cannot be recorded against a cash asset; use record_deposit instead")]
    OpeningBalanceOnCashAsset,
}

/// Use-case composite for the **open holding** failure surface — the single
/// command `open_holding` (TRX-042) and its full chain of rejections.
///
/// Each leaf lives in its rightful layer:
/// - `AccountApplicationError` — application layer (`account/application/`),
///   raises `AccountNotFound { account_id }` and `DatabaseError` from
///   account-side service operations. Asset-side `DatabaseError` from the
///   cross-BC `get_asset_by_id` lookup is also tunnelled through this leaf
///   (the orchestrator translates at the call site) so the FE wire surface
///   carries a single `{ code: "DatabaseError" }` shape.
/// - `OpenHoldingApplicationError` — use-case-owned (this file), raises the
///   3 cross-BC rejections (`AssetNotFound`, `ArchivedAsset`,
///   `OpeningBalanceOnCashAsset`).
/// - `OpeningBalanceDomainError` — domain layer (`account/domain/`), raises
///   `InvalidTotalCost` from `Account::open_holding` on its own input.
/// - `TransactionDomainError` — domain layer (`account/domain/`), raises
///   the date / quantity invariants enforced by `Transaction::new`.
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type)]
#[serde(untagged)]
pub enum OpenHoldingError {
    /// Account-side rejection (`AccountNotFound`, `DatabaseError`).
    #[error(transparent)]
    Application(#[from] AccountApplicationError),
    /// Use-case-layer rejection (cross-BC asset checks).
    #[error(transparent)]
    UseCase(#[from] OpenHoldingApplicationError),
    /// Aggregate-level domain rejection (`InvalidTotalCost`).
    #[error(transparent)]
    Validation(#[from] OpeningBalanceDomainError),
    /// Transaction-factory validation rejection (invalid date, negative
    /// quantity, etc. — subset of variants reachable from `open_holding`).
    #[error(transparent)]
    TxValidation(#[from] TransactionDomainError),
}
