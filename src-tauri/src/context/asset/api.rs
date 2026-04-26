// Allow unreachable lint as tauri::command and specta::specta macros generate false positives
#![allow(clippy::unreachable)]

use crate::context::asset::domain::error::{
    AssetDomainError, AssetPriceDomainError, CategoryDomainError,
};
use crate::use_cases::archive_asset::{ArchiveAssetError, ArchiveAssetUseCase};
use crate::use_cases::delete_asset::{DeleteAssetError, DeleteAssetUseCase};
use crate::AppState;
use serde::{Deserialize, Serialize};
use specta::Type;

use tauri::State;

use super::domain::{Asset, AssetCategory, AssetClass};

// --- DTOs ---

/// Parameters for creating a new asset.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct CreateAssetDTO {
    /// Display name.
    pub name: String,
    /// Ticker, ISIN, or user-defined reference (mandatory — R1).
    pub reference: String,
    /// Classification type.
    pub class: AssetClass,
    /// ISO currency code.
    pub currency: String,
    /// 1-5 risk score.
    pub risk_level: u8,
    /// ID of the primary category.
    pub category_id: String,
}

/// Parameters for updating an existing asset.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct UpdateAssetDTO {
    /// Target asset ID.
    pub asset_id: String,
    /// New display name.
    pub name: String,
    /// New reference (mandatory — R1).
    pub reference: String,
    /// New classification.
    pub class: AssetClass,
    /// New currency.
    pub currency: String,
    /// New risk level.
    pub risk_level: u8,
    /// New category link.
    pub category_id: String,
}

// --- Boundary errors ---

/// Typed error returned to the frontend for asset CRUD commands.
#[derive(Debug, Serialize, Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum AssetCommandError {
    /// Asset name is empty or whitespace-only.
    #[error("Asset name cannot be empty")]
    NameEmpty,
    /// Asset reference (ticker/ISIN) is empty.
    #[error("Asset reference cannot be empty")]
    ReferenceEmpty,
    /// Risk level is outside the 1–5 range.
    #[error("Risk level must be between 1 and 5")]
    InvalidRiskLevel,
    /// Currency string is not a valid ISO 4217 code.
    #[error("Invalid currency code")]
    InvalidCurrency,
    /// Asset is archived and cannot be edited.
    #[error("Cannot edit an archived asset")]
    Archived,
    /// No asset exists with the requested ID.
    #[error("Asset not found")]
    NotFound,
    /// The category referenced in the DTO does not exist.
    #[error("Category not found")]
    CategoryNotFound,
    /// An unexpected server-side error occurred.
    #[error("An unexpected error occurred")]
    Unknown,
}

fn to_asset_error(e: anyhow::Error) -> AssetCommandError {
    if let Some(err) = e.downcast_ref::<AssetDomainError>() {
        return match err {
            AssetDomainError::NameEmpty => AssetCommandError::NameEmpty,
            AssetDomainError::ReferenceEmpty => AssetCommandError::ReferenceEmpty,
            AssetDomainError::InvalidRiskLevel(_) => AssetCommandError::InvalidRiskLevel,
            AssetDomainError::InvalidCurrency(_) => AssetCommandError::InvalidCurrency,
            AssetDomainError::Archived => AssetCommandError::Archived,
            AssetDomainError::NotFound(_) => AssetCommandError::NotFound,
        };
    }
    if let Some(err) = e.downcast_ref::<CategoryDomainError>() {
        return match err {
            CategoryDomainError::NotFound(_) => AssetCommandError::CategoryNotFound,
            other => {
                tracing::error!(err = ?other, "unexpected category error in asset command");
                AssetCommandError::Unknown
            }
        };
    }
    tracing::error!(err = ?e, "unexpected error in asset command");
    AssetCommandError::Unknown
}

/// Typed error returned to the frontend for category commands.
#[derive(Debug, Serialize, Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum CategoryCommandError {
    /// Category label is empty or whitespace-only.
    #[error("Category label cannot be empty")]
    LabelEmpty,
    /// A category with the same name already exists.
    #[error("A category with this name already exists")]
    DuplicateName,
    /// Attempt to rename the system default category.
    #[error("The system category cannot be renamed")]
    SystemReadonly,
    /// Attempt to delete the system default category.
    #[error("The system category cannot be deleted")]
    SystemProtected,
    /// An unexpected server-side error occurred.
    #[error("An unexpected error occurred")]
    Unknown,
}

fn to_category_error(e: anyhow::Error) -> CategoryCommandError {
    if let Some(err) = e.downcast_ref::<CategoryDomainError>() {
        match err {
            CategoryDomainError::LabelEmpty => CategoryCommandError::LabelEmpty,
            CategoryDomainError::DuplicateName => CategoryCommandError::DuplicateName,
            CategoryDomainError::SystemReadonly => CategoryCommandError::SystemReadonly,
            CategoryDomainError::SystemProtected => CategoryCommandError::SystemProtected,
            other => {
                tracing::error!(err = ?other, "unexpected category error in category command");
                CategoryCommandError::Unknown
            }
        }
    } else {
        tracing::error!(err = ?e, "unexpected error in category command");
        CategoryCommandError::Unknown
    }
}

/// Typed error returned to the frontend for the record_asset_price command.
#[derive(Debug, Serialize, Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum AssetPriceCommandError {
    /// Price must be strictly positive.
    #[error("Price must be strictly positive")]
    NotPositive,
    /// Price value is not a finite number.
    #[error("Price must be a finite number")]
    NonFinite,
    /// Price date is in the future.
    #[error("Date cannot be in the future")]
    DateInFuture,
    /// An unexpected server-side error occurred.
    #[error("An unexpected error occurred")]
    Unknown,
}

fn to_asset_price_error(e: anyhow::Error) -> AssetPriceCommandError {
    if let Some(err) = e.downcast_ref::<AssetPriceDomainError>() {
        match err {
            AssetPriceDomainError::NotPositive => AssetPriceCommandError::NotPositive,
            AssetPriceDomainError::NonFinite => AssetPriceCommandError::NonFinite,
            AssetPriceDomainError::DateInFuture => AssetPriceCommandError::DateInFuture,
        }
    } else {
        tracing::error!(err = ?e, "unexpected error in asset price command");
        AssetPriceCommandError::Unknown
    }
}

/// Typed error returned to the frontend for the archive_asset command.
#[derive(Debug, Serialize, Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum ArchiveAssetCommandError {
    /// Asset still has non-zero holdings in at least one account.
    #[error("Cannot archive an asset with active holdings")]
    ActiveHoldings,
    /// An unexpected server-side error occurred.
    #[error("An unexpected error occurred")]
    Unknown,
}

fn to_archive_error(e: anyhow::Error) -> ArchiveAssetCommandError {
    if let Some(err) = e.downcast_ref::<ArchiveAssetError>() {
        match err {
            ArchiveAssetError::ActiveHoldings => ArchiveAssetCommandError::ActiveHoldings,
        }
    } else {
        tracing::error!(err = ?e, "unexpected error in archive_asset command");
        ArchiveAssetCommandError::Unknown
    }
}

/// Typed error returned to the frontend for the delete_asset command.
#[derive(Debug, Serialize, Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum DeleteAssetCommandError {
    /// At least one transaction references this asset.
    #[error("Cannot delete an asset with existing transactions")]
    ExistingTransactions,
    /// An unexpected server-side error occurred.
    #[error("An unexpected error occurred")]
    Unknown,
}

fn to_delete_error(e: anyhow::Error) -> DeleteAssetCommandError {
    if let Some(err) = e.downcast_ref::<DeleteAssetError>() {
        match err {
            DeleteAssetError::ExistingTransactions => DeleteAssetCommandError::ExistingTransactions,
        }
    } else {
        tracing::error!(err = ?e, "unexpected error in delete_asset command");
        DeleteAssetCommandError::Unknown
    }
}

// --- Assets ---

/// Fetches all active (non-archived) assets.
#[tauri::command]
#[specta::specta]
pub async fn get_assets(state: State<'_, AppState>) -> Result<Vec<Asset>, AssetCommandError> {
    state
        .asset_service
        .get_all_assets()
        .await
        .map_err(to_asset_error)
}

/// Fetches all assets including archived ones.
#[tauri::command]
#[specta::specta]
pub async fn get_assets_with_archived(
    state: State<'_, AppState>,
) -> Result<Vec<Asset>, AssetCommandError> {
    state
        .asset_service
        .get_all_assets_with_archived()
        .await
        .map_err(to_asset_error)
}

/// Adds a new asset.
#[tauri::command]
#[specta::specta]
pub async fn add_asset(
    state: State<'_, AppState>,
    dto: CreateAssetDTO,
) -> Result<Asset, AssetCommandError> {
    state
        .asset_service
        .create_asset(dto)
        .await
        .map_err(to_asset_error)
}

/// Updates an existing asset.
#[tauri::command]
#[specta::specta]
pub async fn update_asset(
    state: State<'_, AppState>,
    dto: UpdateAssetDTO,
) -> Result<Asset, AssetCommandError> {
    state
        .asset_service
        .update_asset(dto)
        .await
        .map_err(to_asset_error)
}

/// Archives an asset, guarded against active holdings (R6, OQ-6).
#[tauri::command]
#[specta::specta]
pub async fn archive_asset(
    uc: State<'_, ArchiveAssetUseCase>,
    id: String,
) -> Result<(), ArchiveAssetCommandError> {
    uc.archive_asset(&id).await.map_err(to_archive_error)
}

/// Unarchives an asset (R18).
#[tauri::command]
#[specta::specta]
pub async fn unarchive_asset(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AssetCommandError> {
    state
        .asset_service
        .unarchive_asset(&id)
        .await
        .map_err(to_asset_error)
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

// --- Categories ---

/// Fetches all active categories.
#[tauri::command]
#[specta::specta]
pub async fn get_categories(
    state: State<'_, AppState>,
) -> Result<Vec<AssetCategory>, CategoryCommandError> {
    state
        .asset_service
        .get_all_categories()
        .await
        .map_err(to_category_error)
}

/// Creates a new category.
#[tauri::command]
#[specta::specta]
pub async fn add_category(
    label: String,
    state: State<'_, AppState>,
) -> Result<AssetCategory, CategoryCommandError> {
    state
        .asset_service
        .create_category(&label)
        .await
        .map_err(to_category_error)
}

/// Updates an existing category.
#[tauri::command]
#[specta::specta]
pub async fn update_category(
    id: String,
    label: String,
    state: State<'_, AppState>,
) -> Result<AssetCategory, CategoryCommandError> {
    state
        .asset_service
        .update_category(&id, &label)
        .await
        .map_err(to_category_error)
}

/// Deletes a category.
#[tauri::command]
#[specta::specta]
pub async fn delete_category(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), CategoryCommandError> {
    state
        .asset_service
        .delete_category(&id)
        .await
        .map_err(to_category_error)
}

// --- Market Price ---

/// Records (or overwrites) a market price for an asset on a given date (MKT-024/025).
/// price is a human-readable decimal; the backend converts to i64 micros at this boundary (MKT-024).
#[tauri::command]
#[specta::specta]
pub async fn record_asset_price(
    state: State<'_, AppState>,
    asset_id: String,
    date: String,
    price: f64,
) -> Result<(), AssetPriceCommandError> {
    state
        .asset_service
        .record_price(&asset_id, &date, price)
        .await
        .map_err(to_asset_price_error)
}
