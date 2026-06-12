#![allow(clippy::unreachable)]

use std::sync::Arc;
use tauri::State;

use super::error::{FetchAccountAssetPricesError, FetchAllAssetPricesError};
use super::orchestrator::AssetPriceFetchUseCase;

/// Dispatches an all-accounts auto-fetch task (MKT-122, MKT-130).
/// Returns `Ok(())` immediately after successful dispatch; per-asset results
/// arrive asynchronously via `AssetPriceUpdated` events (MKT-112).
/// `use_api_key` carries the device-local Stooq fetch mode (KEY-050/053):
/// `true` = keyed (resolve + send the key), `false` = keyless (anonymous).
#[tauri::command]
#[specta::specta]
pub async fn fetch_all_asset_prices(
    uc: State<'_, Arc<AssetPriceFetchUseCase>>,
    use_api_key: bool,
) -> Result<(), FetchAllAssetPricesError> {
    uc.fetch_all(use_api_key).await
}

/// Dispatches a per-account price-fetch task (MKT-132, MKT-131).
/// Returns `Ok(())` immediately after successful dispatch.
/// `use_api_key` carries the device-local Stooq fetch mode (KEY-050/053).
#[tauri::command]
#[specta::specta]
pub async fn fetch_account_asset_prices(
    uc: State<'_, Arc<AssetPriceFetchUseCase>>,
    account_id: String,
    use_api_key: bool,
) -> Result<(), FetchAccountAssetPricesError> {
    uc.fetch_for_account(&account_id, use_api_key).await
}
