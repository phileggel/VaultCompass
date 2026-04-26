use super::error::CategoryDomainError;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

/// The fixed ID of the system default category used as a fallback.
pub const SYSTEM_CATEGORY_ID: &str = "default-uncategorized";

/// A user-defined grouping for assets.
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
pub struct AssetCategory {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
}

impl Default for AssetCategory {
    fn default() -> Self {
        Self {
            id: SYSTEM_CATEGORY_ID.to_string(),
            name: "generic.uncategorized".to_string(),
        }
    }
}

impl AssetCategory {
    /// Creates a new AssetCategory.
    pub fn new(label: String) -> Result<Self> {
        if label.trim().is_empty() {
            return Err(CategoryDomainError::LabelEmpty.into());
        }
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            name: label,
        })
    }

    /// Updates an existing AssetCategory.
    pub fn update_from(id: String, label: String) -> Result<Self> {
        if label.trim().is_empty() {
            return Err(CategoryDomainError::LabelEmpty.into());
        }
        Ok(Self { id, name: label })
    }

    /// Creates a new AssetCategory from storage.
    pub fn from_storage(category_id: String, label: String) -> Self {
        Self {
            id: category_id,
            name: label,
        }
    }
}

/// Interface for category persistence.
#[async_trait]
pub trait AssetCategoryRepository: Send + Sync {
    /// Fetches all active categories.
    async fn get_all(&self) -> Result<Vec<AssetCategory>>;
    /// Fetches a category by its ID.
    async fn get_by_id(&self, id: &str) -> Result<Option<AssetCategory>>;
    /// Finds a category by name (case-insensitive).
    async fn find_by_name(&self, name: &str) -> Result<Option<AssetCategory>>;
    /// Persists a new category.
    async fn create(&self, category: AssetCategory) -> Result<AssetCategory>;
    /// Updates an existing category.
    async fn update(&self, category: AssetCategory) -> Result<AssetCategory>;
    /// Reassigns all assets from category_id to fallback_id, then soft-deletes the category.
    /// Both operations run in a single atomic transaction.
    async fn reassign_assets_and_delete(&self, category_id: &str, fallback_id: &str) -> Result<()>;
}
