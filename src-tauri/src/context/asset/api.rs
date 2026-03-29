// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use crate::AppState;
use serde::{Deserialize, Serialize};
use specta::Type;

use tauri::State;

use super::domain::{Asset, AssetCategory, AssetClass, AssetPrice};

// --- DTOs ---

/// Parameters for creating a new asset.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct CreateAssetDTO {
    /// Display name.
    pub name: String,
    /// Optional ticker or reference.
    pub reference: Option<String>,
    /// Classification type.
    pub class: AssetClass,
    /// ISO currency code.
    pub currency: String,
    /// 1-5 risk score.
    pub risk_level: u8,
    /// ID of the primary category.
    pub category_id: String,
}

/// Parameters for updating an existing asset.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct UpdateAssetDTO {
    /// Target asset ID.
    pub asset_id: String,
    /// New display name.
    pub name: String,
    /// New reference.
    pub reference: Option<String>,
    /// New classification.
    pub class: AssetClass,
    /// New currency.
    pub currency: String,
    /// New risk level.
    pub risk_level: u8,
    /// New category link.
    pub category_id: String,
}

/// Parameters for creating a new asset price.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct CreatePriceDTO {
    /// ID of the linked asset.
    pub asset_id: String,
    /// Valuation at the specific date.
    pub price: f64,
    /// ISO 8601 formatted date (YYYY-MM-DD).
    pub date: String,
}

// --- Assets ---

/// Fetches all non-deleted assets.
#[tauri::command]
#[specta::specta]
pub async fn get_assets(state: State<'_, AppState>) -> Result<Vec<Asset>, String> {
    state
        .asset_service
        .get_all_assets()
        .await
        .map_err(|e| e.to_string())
}

/// Adds a new asset.
#[tauri::command]
#[specta::specta]
pub async fn add_asset(state: State<'_, AppState>, dto: CreateAssetDTO) -> Result<Asset, String> {
    state
        .asset_service
        .create_asset(dto)
        .await
        .map_err(|e| e.to_string())
}

/// Updates an existing asset.
#[tauri::command]
#[specta::specta]
pub async fn update_asset(
    state: State<'_, AppState>,
    dto: UpdateAssetDTO,
) -> Result<Asset, String> {
    state
        .asset_service
        .update_asset(dto)
        .await
        .map_err(|e| e.to_string())
}

/// Deletes an asset.
#[tauri::command]
#[specta::specta]
pub async fn delete_asset(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .asset_service
        .delete_asset(&id)
        .await
        .map_err(|e| e.to_string())
}

// --- Prices ---

/// Creates a new price for an asset.
#[tauri::command]
#[specta::specta]
pub async fn create_asset_price(
    state: State<'_, AppState>,
    dto: CreatePriceDTO,
) -> Result<AssetPrice, String> {
    state
        .asset_service
        .create_price(dto)
        .await
        .map_err(|e| e.to_string())
}

// --- Categories ---

/// Fetches all active categories.
#[tauri::command]
#[specta::specta]
pub async fn get_categories(state: State<'_, AppState>) -> Result<Vec<AssetCategory>, String> {
    state
        .asset_service
        .get_all_categories()
        .await
        .map_err(|e| e.to_string())
}

/// Creates a new category.
#[tauri::command]
#[specta::specta]
pub async fn add_category(
    label: String,
    state: State<'_, AppState>,
) -> Result<AssetCategory, String> {
    state
        .asset_service
        .create_category(&label)
        .await
        .map_err(|e| e.to_string())
}

/// Updates an existing category.
#[tauri::command]
#[specta::specta]
pub async fn update_category(
    id: String,
    label: String,
    state: State<'_, AppState>,
) -> Result<AssetCategory, String> {
    state
        .asset_service
        .update_category(&id, &label)
        .await
        .map_err(|e| e.to_string())
}

/// Deletes a category.
#[tauri::command]
#[specta::specta]
pub async fn delete_category(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .asset_service
        .delete_category(&id)
        .await
        .map_err(|e| e.to_string())
}
