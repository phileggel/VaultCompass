// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use super::AccountCreationUseCase;
use crate::context::account::{Account, AccountCrudError, UpdateFrequency};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

/// Parameters for creating a new account.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct CreateAccountDTO {
    /// Display name.
    pub name: String,
    /// ISO 4217 currency code.
    pub currency: String,
    /// Update frequency.
    pub update_frequency: UpdateFrequency,
}

/// Adds a new account, eagerly seeding its Cash Asset + 0-balance Cash Holding
/// (ACC-025, CSH-010 / CSH-012).
#[tauri::command]
#[specta::specta]
pub async fn add_account(
    uc: State<'_, AccountCreationUseCase>,
    dto: CreateAccountDTO,
) -> Result<Account, AccountCrudError> {
    uc.create(dto.name, dto.currency, dto.update_frequency)
        .await
}
