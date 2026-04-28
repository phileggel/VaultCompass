// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use super::{ArchiveAssetError, ArchiveAssetUseCase};
use crate::context::asset::AssetDomainError;
use serde::Serialize;
use specta::Type;
use tauri::State;

/// Typed error returned to the frontend for the archive_asset command.
#[derive(Debug, Serialize, Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum ArchiveAssetCommandError {
    /// Asset still has non-zero holdings in at least one account.
    #[error("Cannot archive an asset with active holdings")]
    ActiveHoldings,
    /// No asset exists with the requested ID.
    #[error("Asset not found")]
    NotFound,
    /// An unexpected server-side error occurred.
    #[error("An unexpected error occurred")]
    Unknown,
}

fn to_archive_error(e: anyhow::Error) -> ArchiveAssetCommandError {
    if let Some(err) = e.downcast_ref::<ArchiveAssetError>() {
        return match err {
            ArchiveAssetError::ActiveHoldings => ArchiveAssetCommandError::ActiveHoldings,
        };
    }
    if let Some(err) = e.downcast_ref::<AssetDomainError>() {
        return match err {
            AssetDomainError::NotFound(_) => ArchiveAssetCommandError::NotFound,
            other => {
                tracing::error!(err = ?other, "unexpected asset error in archive_asset command");
                ArchiveAssetCommandError::Unknown
            }
        };
    }
    tracing::error!(err = ?e, "unexpected error in archive_asset command");
    ArchiveAssetCommandError::Unknown
}

/// Archives an asset, guarded against active holdings (OQ-6).
#[tauri::command]
#[specta::specta]
pub async fn archive_asset(
    uc: State<'_, ArchiveAssetUseCase>,
    id: String,
) -> Result<(), ArchiveAssetCommandError> {
    uc.archive_asset(&id).await.map_err(to_archive_error)
}
