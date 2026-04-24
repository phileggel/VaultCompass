use super::domain::{
    Asset, AssetCategory, AssetCategoryRepository, AssetRepository, SYSTEM_CATEGORY_ID,
};
use crate::{
    context::asset::{CreateAssetDTO, UpdateAssetDTO},
    core::{Event, SideEffectEventBus},
};
use anyhow::Result;
use std::sync::Arc;

/// Orchestrates business logic for assets and categories.
pub struct AssetService {
    asset_repo: Box<dyn AssetRepository>,
    category_repo: Box<dyn AssetCategoryRepository>,
    event_bus: Option<Arc<SideEffectEventBus>>,
}

impl AssetService {
    /// Creates a new AssetService.
    pub fn new(
        asset_repo: Box<dyn AssetRepository>,
        category_repo: Box<dyn AssetCategoryRepository>,
    ) -> Self {
        Self {
            asset_repo,
            category_repo,
            event_bus: None,
        }
    }

    /// Attaches an event bus for side-effect notifications.
    pub fn with_event_bus(mut self, bus: Arc<SideEffectEventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    // --- Asset Methods ---

    /// Retrieves all active (non-archived) assets.
    pub async fn get_all_assets(&self) -> Result<Vec<Asset>> {
        self.asset_repo.get_all().await
    }

    /// Retrieves all assets including archived ones.
    pub async fn get_all_assets_with_archived(&self) -> Result<Vec<Asset>> {
        self.asset_repo.get_all_including_archived().await
    }

    /// Retrieves a single asset by ID.
    pub async fn get_asset_by_id(&self, asset_id: &str) -> Result<Option<Asset>> {
        self.asset_repo.get_by_id(asset_id).await
    }

    /// Creates a new asset and publishes an AssetUpdated event.
    pub async fn create_asset(&self, dto: CreateAssetDTO) -> Result<Asset> {
        let category = self
            .get_category_by_id(&dto.category_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Category not found: {}", dto.category_id))?;

        let asset = Asset::new(
            dto.name,
            dto.class,
            category,
            dto.currency,
            dto.risk_level,
            dto.reference,
        )?;

        let asset = self.asset_repo.create(asset).await?;
        tracing::info!(asset_id = %asset.id, name = %asset.name, "Asset created");

        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AssetUpdated);
        }

        Ok(asset)
    }

    /// Updates an existing asset. Rejects if the asset is archived (R6).
    pub async fn update_asset(&self, dto: UpdateAssetDTO) -> Result<Asset> {
        let existing = self
            .asset_repo
            .get_by_id(&dto.asset_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Asset not found: {}", dto.asset_id))?;

        if existing.is_archived {
            anyhow::bail!("Cannot edit an archived asset");
        }

        let category = self
            .get_category_by_id(&dto.category_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Category not found: {}", dto.category_id))?;

        let asset = Asset::with_id(
            dto.asset_id,
            dto.name,
            dto.class,
            category,
            dto.currency,
            dto.risk_level,
            dto.reference,
            false,
        )?;

        let asset = self.asset_repo.update(asset).await?;
        tracing::info!(asset_id = %asset.id, name = %asset.name, "Asset updated");

        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AssetUpdated);
        }

        Ok(asset)
    }

    /// Archives an asset (reversible — R6).
    pub async fn archive_asset(&self, asset_id: &str) -> Result<()> {
        self.asset_repo.archive(asset_id).await?;
        tracing::info!(asset_id = %asset_id, "Asset archived");
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AssetUpdated);
        }
        Ok(())
    }

    /// Unarchives an asset (R18).
    pub async fn unarchive_asset(&self, asset_id: &str) -> Result<()> {
        self.asset_repo.unarchive(asset_id).await?;
        tracing::info!(asset_id = %asset_id, "Asset unarchived");
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AssetUpdated);
        }
        Ok(())
    }

    /// Soft-deletes an asset and publishes an AssetUpdated event.
    pub async fn delete_asset(&self, asset_id: &str) -> Result<()> {
        self.asset_repo.delete(asset_id).await?;
        tracing::info!(asset_id = %asset_id, "Asset deleted");
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AssetUpdated);
        }
        Ok(())
    }

    // --- Category Methods ---

    /// Retrieves all non-deleted categories.
    pub async fn get_all_categories(&self) -> Result<Vec<AssetCategory>> {
        self.category_repo.get_all().await
    }

    /// Retrieves a category by ID.
    pub async fn get_category_by_id(&self, id: &str) -> Result<Option<AssetCategory>> {
        self.category_repo.get_by_id(id).await
    }

    /// Creates a category and publishes a CategoryUpdated event.
    pub async fn create_category(&self, label: &str) -> Result<AssetCategory> {
        if self.category_repo.find_by_name(label).await?.is_some() {
            anyhow::bail!("error.category.duplicate_name");
        }
        let category = AssetCategory::new(label.to_string())?;
        let category = self.category_repo.create(category).await?;
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::CategoryUpdated);
        }
        Ok(category)
    }

    /// Updates a category and publishes a CategoryUpdated event.
    pub async fn update_category(&self, id: &str, label: &str) -> Result<AssetCategory> {
        if id == SYSTEM_CATEGORY_ID {
            anyhow::bail!("error.category.system_readonly");
        }
        if let Some(existing) = self.category_repo.find_by_name(label).await? {
            if existing.id != id {
                anyhow::bail!("error.category.duplicate_name");
            }
        }
        let category = AssetCategory::update_from(id.to_string(), label.to_string())?;
        let category = self.category_repo.update(category).await?;
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::CategoryUpdated);
        }
        Ok(category)
    }

    /// Reassigns assets to default category, then deletes the category.
    pub async fn delete_category(&self, category_id: &str) -> Result<()> {
        if category_id == SYSTEM_CATEGORY_ID {
            anyhow::bail!("error.category.system_protected");
        }
        self.category_repo
            .reassign_assets_and_delete(category_id, SYSTEM_CATEGORY_ID)
            .await?;
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::CategoryUpdated);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::asset::{
        AssetClass, CreateAssetDTO, SqliteAssetCategoryRepository, SqliteAssetRepository,
    };
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_service() -> AssetService {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test pool");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");
        AssetService::new(
            Box::new(SqliteAssetRepository::new(pool.clone())),
            Box::new(SqliteAssetCategoryRepository::new(pool)),
        )
    }

    fn base_dto(name: &str) -> CreateAssetDTO {
        CreateAssetDTO {
            name: name.to_string(),
            reference: "REF-001".to_string(),
            class: AssetClass::Cash,
            currency: "USD".to_string(),
            risk_level: 1,
            category_id: SYSTEM_CATEGORY_ID.to_string(),
        }
    }

    // R1 — empty name is rejected
    #[tokio::test]
    async fn create_asset_rejects_empty_name() {
        let svc = setup_service().await;
        let err = svc
            .create_asset(CreateAssetDTO {
                name: "".to_string(),
                ..base_dto("ignored")
            })
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("name cannot be empty"),
            "got: {err}"
        );
    }

    // R1 — empty reference is rejected
    #[tokio::test]
    async fn create_asset_rejects_empty_reference() {
        let svc = setup_service().await;
        let err = svc
            .create_asset(CreateAssetDTO {
                reference: "".to_string(),
                ..base_dto("Bond")
            })
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("reference cannot be empty"),
            "got: {err}"
        );
    }

    // R1 — invalid currency is rejected
    #[tokio::test]
    async fn create_asset_rejects_invalid_currency() {
        let svc = setup_service().await;
        let err = svc
            .create_asset(CreateAssetDTO {
                currency: "INVALID".to_string(),
                ..base_dto("Bond")
            })
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Invalid currency"), "got: {err}");
    }

    // R1 — risk level out of range is rejected
    #[tokio::test]
    async fn create_asset_rejects_invalid_risk_level() {
        let svc = setup_service().await;
        let err = svc
            .create_asset(CreateAssetDTO {
                risk_level: 6,
                ..base_dto("Bond")
            })
            .await
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("Risk level must be between 1 and 5"),
            "got: {err}"
        );
    }

    // R4 — reference is normalized to uppercase
    #[tokio::test]
    async fn create_asset_normalizes_reference_to_uppercase() {
        let svc = setup_service().await;
        let asset = svc
            .create_asset(CreateAssetDTO {
                reference: "aapl".to_string(),
                ..base_dto("Apple")
            })
            .await
            .unwrap();
        assert_eq!(asset.reference, "AAPL");
    }

    // R4 — reference leading/trailing spaces are trimmed
    #[tokio::test]
    async fn create_asset_normalizes_reference_trims_spaces() {
        let svc = setup_service().await;
        let asset = svc
            .create_asset(CreateAssetDTO {
                reference: "  AAPL  ".to_string(),
                ..base_dto("Apple")
            })
            .await
            .unwrap();
        assert_eq!(asset.reference, "AAPL");
    }

    // R5/R6 — updating an archived asset is rejected
    #[tokio::test]
    async fn update_archived_asset_is_rejected() {
        let svc = setup_service().await;
        let asset = svc.create_asset(base_dto("Apple")).await.unwrap();
        svc.archive_asset(&asset.id).await.unwrap();
        let err = svc
            .update_asset(crate::context::asset::UpdateAssetDTO {
                asset_id: asset.id.clone(),
                name: "Apple Updated".to_string(),
                reference: "AAPL".to_string(),
                class: AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 4,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
            })
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("Cannot edit an archived asset"),
            "got: {err}"
        );
    }

    // R6 — archiving sets is_archived = true
    #[tokio::test]
    async fn archive_asset_sets_flag() {
        let svc = setup_service().await;
        let asset = svc.create_asset(base_dto("Apple")).await.unwrap();
        svc.archive_asset(&asset.id).await.unwrap();
        let all = svc.get_all_assets_with_archived().await.unwrap();
        let found = all.iter().find(|a| a.id == asset.id).unwrap();
        assert!(found.is_archived);
    }

    // R18 — unarchiving clears is_archived
    #[tokio::test]
    async fn unarchive_asset_clears_flag() {
        let svc = setup_service().await;
        let asset = svc.create_asset(base_dto("Apple")).await.unwrap();
        svc.archive_asset(&asset.id).await.unwrap();
        svc.unarchive_asset(&asset.id).await.unwrap();
        let all = svc.get_all_assets().await.unwrap();
        let found = all.iter().find(|a| a.id == asset.id).unwrap();
        assert!(!found.is_archived);
    }

    // R7 — get_all excludes archived assets
    #[tokio::test]
    async fn get_all_assets_excludes_archived() {
        let svc = setup_service().await;
        let asset = svc.create_asset(base_dto("Apple")).await.unwrap();
        svc.archive_asset(&asset.id).await.unwrap();
        let active = svc.get_all_assets().await.unwrap();
        assert!(!active.iter().any(|a| a.id == asset.id));
    }

    // R19 — get_all_with_archived includes both active and archived
    #[tokio::test]
    async fn get_all_assets_with_archived_includes_both() {
        let svc = setup_service().await;
        let active = svc
            .create_asset(CreateAssetDTO {
                reference: "ACT".to_string(),
                ..base_dto("Active Asset")
            })
            .await
            .unwrap();
        let archived = svc
            .create_asset(CreateAssetDTO {
                reference: "ARC".to_string(),
                ..base_dto("Archived Asset")
            })
            .await
            .unwrap();
        svc.archive_asset(&archived.id).await.unwrap();
        let all = svc.get_all_assets_with_archived().await.unwrap();
        assert!(all.iter().any(|a| a.id == active.id));
        assert!(all.iter().any(|a| a.id == archived.id));
    }

    // Category tests

    // R1 — duplicate name, same case
    #[tokio::test]
    async fn create_category_rejects_duplicate_same_case() {
        let svc = setup_service().await;
        svc.create_category("Bonds").await.unwrap();
        let err = svc.create_category("Bonds").await.unwrap_err();
        assert!(err.to_string().contains("duplicate_name"), "got: {err}");
    }

    // R1 — duplicate name, different case
    #[tokio::test]
    async fn create_category_rejects_duplicate_different_case() {
        let svc = setup_service().await;
        svc.create_category("Bonds").await.unwrap();
        let err = svc.create_category("bonds").await.unwrap_err();
        assert!(err.to_string().contains("duplicate_name"), "got: {err}");
    }

    // R2 — system category cannot be renamed
    #[tokio::test]
    async fn update_category_rejects_system_category() {
        let svc = setup_service().await;
        let err = svc
            .update_category(SYSTEM_CATEGORY_ID, "Renamed")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("system_readonly"), "got: {err}");
    }

    // R1 — update with name already taken by another category
    #[tokio::test]
    async fn update_category_rejects_duplicate_name() {
        let svc = setup_service().await;
        svc.create_category("Bonds").await.unwrap();
        let cat2 = svc.create_category("Equities").await.unwrap();
        let err = svc.update_category(&cat2.id, "bonds").await.unwrap_err();
        assert!(err.to_string().contains("duplicate_name"), "got: {err}");
    }

    // R2 — system category cannot be deleted
    #[tokio::test]
    async fn delete_category_rejects_system_category() {
        let svc = setup_service().await;
        let err = svc.delete_category(SYSTEM_CATEGORY_ID).await.unwrap_err();
        assert!(err.to_string().contains("system_protected"), "got: {err}");
    }

    // R3 — deleting a category reassigns its assets to the default category
    #[tokio::test]
    async fn delete_category_reassigns_assets_to_default() {
        let svc = setup_service().await;
        let cat = svc.create_category("Bonds").await.unwrap();
        let asset = svc
            .create_asset(CreateAssetDTO {
                category_id: cat.id.clone(),
                ..base_dto("Test Bond")
            })
            .await
            .unwrap();
        svc.delete_category(&cat.id).await.unwrap();
        let assets = svc.get_all_assets().await.unwrap();
        let updated = assets.iter().find(|a| a.id == asset.id).unwrap();
        assert_eq!(updated.category.id, SYSTEM_CATEGORY_ID);
    }
}
