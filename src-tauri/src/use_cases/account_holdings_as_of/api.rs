// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use super::orchestrator::{AccountHoldingsAsOfUseCase, HoldingsAsOfResponse};
use crate::context::account::AccountError;
use tauri::State;

/// Returns the account's holdings reconstructed as of a past date (read-only).
#[tauri::command]
#[specta::specta]
pub async fn get_account_holdings_as_of(
    state: State<'_, AccountHoldingsAsOfUseCase>,
    account_id: String,
    as_of_date: String,
) -> Result<HoldingsAsOfResponse, AccountError> {
    state
        .get_account_holdings_as_of(&account_id, &as_of_date)
        .await
}
