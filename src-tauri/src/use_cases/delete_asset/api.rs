// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use super::{DeleteAssetError, DeleteAssetUseCase};
use crate::context::asset::AssetDomainError;
use serde::Serialize;
use specta::Type;
use tauri::State;

/// Typed error returned to the frontend for the delete_asset command.
#[derive(Debug, Serialize, Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum DeleteAssetCommandError {
    /// At least one transaction references this asset.
    #[error("Cannot delete an asset with existing transactions")]
    ExistingTransactions,
    /// Target asset is a system Cash Asset and cannot be deleted (CSH-016).
    #[error("Cannot delete a system Cash Asset")]
    CashAssetNotEditable,
    /// No asset exists with the requested ID.
    #[error("Asset not found")]
    NotFound,
    /// An unexpected server-side error occurred.
    #[error("An unexpected error occurred")]
    Unknown,
}

fn to_delete_error(e: anyhow::Error) -> DeleteAssetCommandError {
    if let Some(err) = e.downcast_ref::<DeleteAssetError>() {
        return match err {
            DeleteAssetError::ExistingTransactions => DeleteAssetCommandError::ExistingTransactions,
        };
    }
    if let Some(err) = e.downcast_ref::<AssetDomainError>() {
        return match err {
            AssetDomainError::NotFound(_) => DeleteAssetCommandError::NotFound,
            AssetDomainError::CashAssetNotEditable => DeleteAssetCommandError::CashAssetNotEditable,
            other => {
                tracing::error!(err = ?other, "unexpected asset error in delete_asset command");
                DeleteAssetCommandError::Unknown
            }
        };
    }
    tracing::error!(err = ?e, "unexpected error in delete_asset command");
    DeleteAssetCommandError::Unknown
}

/// Deletes an asset, guarded against existing transactions.
#[tauri::command]
#[specta::specta]
pub async fn delete_asset(
    uc: State<'_, DeleteAssetUseCase>,
    id: String,
) -> Result<(), DeleteAssetCommandError> {
    uc.delete_asset(&id).await.map_err(to_delete_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    // CSH-016 — to_delete_error maps AssetDomainError::CashAssetNotEditable
    #[test]
    fn to_delete_error_maps_cash_asset_not_editable() {
        let domain_err = AssetDomainError::CashAssetNotEditable;
        let cmd_err = to_delete_error(anyhow::anyhow!(domain_err));
        assert!(
            matches!(cmd_err, DeleteAssetCommandError::CashAssetNotEditable),
            "got: {cmd_err:?}"
        );
    }
}
