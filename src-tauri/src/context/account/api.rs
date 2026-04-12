// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use super::domain::{Account, UpdateFrequency};
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
