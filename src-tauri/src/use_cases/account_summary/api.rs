// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use super::orchestrator::{AccountSummary, AccountSummaryUseCase};
use crate::context::account::AccountApplicationError;
use tauri::State;

/// Returns one `AccountSummary` per non-deleted account (ACC-021).
#[tauri::command]
#[specta::specta]
pub async fn get_account_summaries(
    state: State<'_, AccountSummaryUseCase>,
) -> Result<Vec<AccountSummary>, AccountApplicationError> {
    state.get_account_summaries().await
}
