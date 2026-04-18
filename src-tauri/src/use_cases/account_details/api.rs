// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use super::orchestrator::{AccountDetailsResponse, AccountDetailsUseCase};
use tauri::State;

/// Returns the full account details view for the given account (ACD-012 to ACD-041).
#[tauri::command]
#[specta::specta]
pub async fn get_account_details(
    state: State<'_, AccountDetailsUseCase>,
    account_id: String,
) -> Result<AccountDetailsResponse, String> {
    state
        .get_account_details(&account_id)
        .await
        .map_err(|e| e.to_string())
}
