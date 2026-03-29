// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use super::domain::{Account, AssetAccount, UpdateFrequency};
use crate::AppState;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

// --- DTOs ---

/// Parameters for creating a new account.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct CreateAccountDTO {
    /// Display name.
    pub name: String,
    /// Update frequency.
    pub update_frequency: UpdateFrequency,
}

/// Parameters for updating an existing account.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct UpdateAccountDTO {
    /// Target account ID.
    pub id: String,
    /// New display name.
    pub name: String,
    /// New update frequency.
    pub update_frequency: UpdateFrequency,
}

/// Parameters for recording asset holdings in an account.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct UpsertHoldingDTO {
    /// Linked account ID.
    pub account_id: String,
    /// Linked asset ID.
    pub asset_id: String,
    /// Average purchase price in the asset's currency.
    pub average_price: f64,
    /// Quantity of the asset held.
    pub quantity: f64,
}

// --- Commands ---

/// Retrieves all accounts.
#[tauri::command]
#[specta::specta]
pub async fn get_accounts(state: State<'_, AppState>) -> Result<Vec<Account>, String> {
    state
        .account_service
        .get_all()
        .await
        .map_err(|e| e.to_string())
}

/// Adds a new account.
#[tauri::command]
#[specta::specta]
pub async fn add_account(
    state: State<'_, AppState>,
    dto: CreateAccountDTO,
) -> Result<Account, String> {
    state
        .account_service
        .create(dto.name, dto.update_frequency)
        .await
        .map_err(|e| e.to_string())
}

/// Updates an existing account.
#[tauri::command]
#[specta::specta]
pub async fn update_account(
    state: State<'_, AppState>,
    dto: UpdateAccountDTO,
) -> Result<Account, String> {
    state
        .account_service
        .update(dto.id, dto.name, dto.update_frequency)
        .await
        .map_err(|e| e.to_string())
}

/// Deletes an account.
#[tauri::command]
#[specta::specta]
pub async fn delete_account(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .account_service
        .delete(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Gets holdings for an account.
#[tauri::command]
#[specta::specta]
pub async fn get_account_holdings(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<AssetAccount>, String> {
    state
        .account_service
        .get_holdings(&account_id)
        .await
        .map_err(|e| e.to_string())
}

/// Updates or creates an asset holding in an account.
#[tauri::command]
#[specta::specta]
pub async fn upsert_account_holding(
    state: State<'_, AppState>,
    dto: UpsertHoldingDTO,
) -> Result<AssetAccount, String> {
    state
        .account_service
        .upsert_holding(
            dto.account_id,
            dto.asset_id,
            dto.average_price,
            dto.quantity,
        )
        .await
        .map_err(|e| e.to_string())
}

/// Removes an asset holding from an account.
#[tauri::command]
#[specta::specta]
pub async fn remove_account_holding(
    state: State<'_, AppState>,
    account_id: String,
    asset_id: String,
) -> Result<(), String> {
    state
        .account_service
        .remove_holding(&account_id, &asset_id)
        .await
        .map_err(|e| e.to_string())
}
