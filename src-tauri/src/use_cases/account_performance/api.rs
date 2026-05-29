// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use super::orchestrator::{AccountPerformanceResponse, AccountPerformanceUseCase};
use crate::context::account::AccountApplicationError;
use tauri::State;

/// Returns per-period performance figures for a single account (PRF spec).
#[tauri::command]
#[specta::specta]
pub async fn get_account_performance(
    account_id: String,
    state: State<'_, AccountPerformanceUseCase>,
) -> Result<AccountPerformanceResponse, AccountApplicationError> {
    state.get_account_performance(&account_id).await
}
