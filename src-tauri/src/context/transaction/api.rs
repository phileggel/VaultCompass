// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use crate::context::transaction::service::TransactionService;
use tauri::State;

/// Returns the distinct asset IDs that have transactions for the given account (TXL-013).
#[tauri::command]
#[specta::specta]
pub async fn get_asset_ids_for_account(
    state: State<'_, TransactionService>,
    account_id: String,
) -> Result<Vec<String>, String> {
    state
        .get_asset_ids_for_account(&account_id)
        .await
        .map_err(|e| e.to_string())
}
