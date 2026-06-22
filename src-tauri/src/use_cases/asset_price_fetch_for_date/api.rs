#![allow(clippy::unreachable)]

use std::sync::Arc;
use tauri::State;

use super::error::FetchAccountAssetPricesForDateError;
use super::orchestrator::{AssetPriceFetchForDateUseCase, FetchForDateOutcome};

/// Fetches each fetchable holding's close at (or carried back to) `date` for the
/// account and stores it keyed to that date. Keyless (ADR-017). Unlike the latest
/// auto-fetch, this awaits every fetch and returns a [`FetchForDateOutcome`]
/// summarizing how many prices were stored and which assets had no data.
#[tauri::command]
#[specta::specta]
pub async fn fetch_account_asset_prices_for_date(
    uc: State<'_, Arc<AssetPriceFetchForDateUseCase>>,
    account_id: String,
    date: String,
) -> Result<FetchForDateOutcome, FetchAccountAssetPricesForDateError> {
    uc.fetch_for_account_on_date(&account_id, &date).await
}
