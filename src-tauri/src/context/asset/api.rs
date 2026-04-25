// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use crate::use_cases::archive_asset::ArchiveAssetUseCase;
use crate::use_cases::delete_asset::DeleteAssetUseCase;
use crate::AppState;
use serde::{Deserialize, Serialize};
use specta::Type;

use tauri::State;

use super::domain::{Asset, AssetCategory, AssetClass};

// --- DTOs ---

/// Parameters for creating a new asset.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct CreateAssetDTO {
    /// Display name.
    pub name: String,
    /// Ticker, ISIN, or user-defined reference (mandatory — R1).
    pub reference: String,
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
    /// New reference (mandatory — R1).
    pub reference: String,
    /// New classification.
    pub class: AssetClass,
    /// New currency.
    pub currency: String,
    /// New risk level.
    pub risk_level: u8,
    /// New category link.
    pub category_id: String,
}

// --- Assets ---

/// Fetches all active (non-archived) assets.
#[tauri::command]
#[specta::specta]
pub async fn get_assets(state: State<'_, AppState>) -> Result<Vec<Asset>, String> {
    state
        .asset_service
        .get_all_assets()
        .await
        .map_err(|e| e.to_string())
}

/// Fetches all assets including archived ones.
#[tauri::command]
#[specta::specta]
pub async fn get_assets_with_archived(state: State<'_, AppState>) -> Result<Vec<Asset>, String> {
    state
        .asset_service
        .get_all_assets_with_archived()
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

/// Archives an asset, guarded against active holdings (R6, OQ-6).
#[tauri::command]
#[specta::specta]
pub async fn archive_asset(uc: State<'_, ArchiveAssetUseCase>, id: String) -> Result<(), String> {
    uc.archive_asset(&id).await.map_err(|e| e.to_string())
}

/// Unarchives an asset (R18).
#[tauri::command]
#[specta::specta]
pub async fn unarchive_asset(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .asset_service
        .unarchive_asset(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Deletes an asset, guarded against existing transactions.
#[tauri::command]
#[specta::specta]
pub async fn delete_asset(uc: State<'_, DeleteAssetUseCase>, id: String) -> Result<(), String> {
    uc.delete_asset(&id).await.map_err(|e| e.to_string())
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
