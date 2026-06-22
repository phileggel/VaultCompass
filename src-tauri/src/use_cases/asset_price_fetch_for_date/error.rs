use crate::context::account::AccountApplicationError;
use crate::context::asset::AssetError;
use serde::Serialize;
use specta::Type;

/// Use-case-specific outcomes for the date-scoped price fetch.
///
/// `#[serde(tag = "code")]` gives every variant a `{ "code": "..." }` payload so the
/// surrounding `#[serde(untagged)]` composite emits a flat, narrowable wire shape.
#[derive(Debug, thiserror::Error, Serialize, Type, Clone)]
#[serde(tag = "code")]
pub enum FetchPriceForDateTask {
    /// The supplied date is not a well-formed ISO `yyyy-mm-dd` string. The raw input
    /// is not echoed back on the wire — the caller already holds it.
    #[error("Invalid date")]
    InvalidDate,
    /// The supplied date is in the future — no price can exist for it.
    #[error("Date is in the future")]
    DateInFuture,
    /// Catch-all for unexpected runtime failures not attributable to a specific BC.
    #[error("Unexpected error")]
    UnknownError,
}

/// Wire-facing error composite for `fetch_account_asset_prices_for_date`.
///
/// `#[serde(untagged)]` lets every arm surface its inner `{ "code": "..." }` payload
/// directly on the wire; each arm carries a tagged inner type so the discriminator
/// survives the untagging.
// The `Asset` and `Account` arms both carry a `DatabaseError` code; under
// `#[serde(untagged)]` they collide to the same wire shape. This mirrors the shipped
// sibling `FetchAccountAssetPricesError` and is intentional — the frontend maps both
// to the single `error.DatabaseError` key, so the collision is unobservable.
#[derive(Debug, thiserror::Error, Serialize, Type)]
#[serde(untagged)]
pub enum FetchAccountAssetPricesForDateError {
    /// Propagates asset-BC failures (e.g. `DatabaseError`) via `?`.
    #[error(transparent)]
    Asset(#[from] AssetError),
    /// Propagates account-BC failures (`AccountNotFound`, `DatabaseError`) via `?`.
    #[error(transparent)]
    Account(#[from] AccountApplicationError),
    /// Use-case-specific failures (`InvalidDate`, `DateInFuture`, `UnknownError`).
    #[error(transparent)]
    Failure(#[from] FetchPriceForDateTask),
}
