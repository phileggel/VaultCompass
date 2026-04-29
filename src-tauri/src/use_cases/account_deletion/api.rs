// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use super::{AccountDeletionSummary, AccountDeletionUseCase};
use serde::Serialize;
use specta::Type;
use tauri::State;

/// Typed error returned to the frontend for the get_account_deletion_summary command.
#[derive(Debug, Serialize, Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum AccountDeletionCommandError {
    /// An unexpected server-side error occurred.
    #[error("An unexpected error occurred")]
    Unknown,
}

/// Returns the number of active holdings and transactions for an account (ACC-020).
///
/// Used by the frontend to decide whether to show the standard or reinforced
/// delete confirmation dialog (ACC-018 vs ACC-019).
#[tauri::command]
#[specta::specta]
pub async fn get_account_deletion_summary(
    uc: State<'_, AccountDeletionUseCase>,
    account_id: String,
) -> Result<AccountDeletionSummary, AccountDeletionCommandError> {
    uc.get_summary(&account_id).await.map_err(|e| {
        tracing::error!(err = ?e, "unexpected error in get_account_deletion_summary");
        AccountDeletionCommandError::Unknown
    })
}
