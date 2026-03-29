use super::domain::{
    Asset, AssetCategory, AssetCategoryRepository, AssetPrice, AssetRepository, PriceRepository,
    SYSTEM_CATEGORY_ID,
};
use crate::{
    context::asset::{CreateAssetDTO, CreatePriceDTO, UpdateAssetDTO},
    core::{Event, SideEffectEventBus},
};
use anyhow::Result;
use std::sync::Arc;

/// Orchestrates business logic for assets, categories, and prices.
pub struct AssetService {
    asset_repo: Box<dyn AssetRepository>,
    category_repo: Box<dyn AssetCategoryRepository>,
    price_repo: Arc<dyn PriceRepository>,
    event_bus: Option<Arc<SideEffectEventBus>>,
}

impl AssetService {
    /// Creates a new AssetService.
    pub fn new(
        asset_repo: Box<dyn AssetRepository>,
        category_repo: Box<dyn AssetCategoryRepository>,
        price_repo: Arc<dyn PriceRepository>,
    ) -> Self {
        Self {
            asset_repo,
            category_repo,
            price_repo,
            event_bus: None,
        }
    }

    /// Attaches an event bus for side-effect notifications.
    pub fn with_event_bus(mut self, bus: Arc<SideEffectEventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    // --- Asset Methods ---

    /// Retrieves all non-deleted assets.
    pub async fn get_all_assets(&self) -> Result<Vec<Asset>> {
        self.asset_repo.get_all().await
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

        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AssetUpdated);
        }

        Ok(asset)
    }

    /// Updates an existing asset and publishes an AssetUpdated event.
    pub async fn update_asset(&self, dto: UpdateAssetDTO) -> Result<Asset> {
        let category = self
            .get_category_by_id(&dto.category_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Category not found: {}", dto.category_id))?;

        let asset = Asset::update_from(
            dto.asset_id,
            dto.name,
            dto.class,
            category,
            dto.currency,
            dto.risk_level,
            dto.reference,
        )?;

        let asset = self.asset_repo.update(asset).await?;

        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AssetUpdated);
        }

        Ok(asset)
    }

    /// Soft-deletes an asset and publishes an AssetUpdated event.
    pub async fn delete_asset(&self, asset_id: &str) -> Result<()> {
        self.asset_repo.delete(asset_id).await?;
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

    // --- Price Methods ---

    /// Retrieves all price history for an asset.
    pub async fn get_all_prices_by_asset(&self, asset_id: &str) -> Result<Vec<AssetPrice>> {
        self.price_repo.get_by_asset(asset_id).await
    }

    /// Retrieves price history within a specific range.
    pub async fn get_prices_by_asset_and_date_range(
        &self,
        asset_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<AssetPrice>> {
        self.price_repo
            .get_by_asset_and_date_range(asset_id, start_date, end_date)
            .await
    }

    /// Creates a new price snapshot.
    pub async fn create_price(&self, dto: CreatePriceDTO) -> Result<AssetPrice> {
        let price = AssetPrice::new(dto.asset_id, dto.price, dto.date)?;
        self.price_repo.create(price).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::asset::{
        AssetClass, CreateAssetDTO, SqliteAssetCategoryRepository, SqliteAssetRepository,
        SqlitePriceRepository,
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
            Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
            Arc::new(SqlitePriceRepository::new(pool)),
        )
    }

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
                name: "Test Bond".to_string(),
                reference: None,
                class: AssetClass::Bonds,
                currency: "USD".to_string(),
                risk_level: 2,
                category_id: cat.id.clone(),
            })
            .await
            .unwrap();
        svc.delete_category(&cat.id).await.unwrap();
        let assets = svc.get_all_assets().await.unwrap();
        let updated = assets.iter().find(|a| a.id == asset.id).unwrap();
        assert_eq!(updated.category.id, SYSTEM_CATEGORY_ID);
    }
}
