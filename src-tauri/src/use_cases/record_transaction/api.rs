// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use super::orchestrator::{CreateTransactionDTO, RecordTransactionUseCase};
use crate::context::transaction::Transaction;
use tauri::State;

/// Creates a new purchase transaction and updates the Holding atomically (TRX-027).
#[tauri::command]
#[specta::specta]
pub async fn add_transaction(
    state: State<'_, RecordTransactionUseCase>,
    dto: CreateTransactionDTO,
) -> Result<Transaction, String> {
    state
        .create_transaction(dto)
        .await
        .map_err(|e| e.to_string())
}

/// Updates an existing transaction and recalculates the affected Holding(s) (TRX-031, TRX-032).
#[tauri::command]
#[specta::specta]
pub async fn update_transaction(
    state: State<'_, RecordTransactionUseCase>,
    id: String,
    dto: CreateTransactionDTO,
) -> Result<Transaction, String> {
    state
        .update_transaction(id, dto)
        .await
        .map_err(|e| e.to_string())
}

/// Deletes a transaction and recalculates (or removes) the associated Holding (TRX-034).
#[tauri::command]
#[specta::specta]
pub async fn delete_transaction(
    state: State<'_, RecordTransactionUseCase>,
    id: String,
) -> Result<(), String> {
    state
        .delete_transaction(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieves all transactions for an account/asset pair.
#[tauri::command]
#[specta::specta]
pub async fn get_transactions(
    state: State<'_, RecordTransactionUseCase>,
    account_id: String,
    asset_id: String,
) -> Result<Vec<Transaction>, String> {
    state
        .get_transactions(&account_id, &asset_id)
        .await
        .map_err(|e| e.to_string())
}
