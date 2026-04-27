use super::domain::{
    Asset, AssetCategory, AssetCategoryRepository, AssetDomainError, AssetPrice,
    AssetPriceDomainError, AssetPriceRepository, AssetRepository, CategoryDomainError,
    SYSTEM_CATEGORY_ID,
};
use crate::{
    context::asset::{CreateAssetDTO, UpdateAssetDTO},
    core::{Event, SideEffectEventBus, BACKEND},
};
use anyhow::Result;
use std::sync::Arc;

/// Orchestrates business logic for assets, categories, and market prices.
pub struct AssetService {
    asset_repo: Box<dyn AssetRepository>,
    category_repo: Box<dyn AssetCategoryRepository>,
    price_repo: Box<dyn AssetPriceRepository>,
    event_bus: Option<Arc<SideEffectEventBus>>,
}

impl AssetService {
    /// Creates a new AssetService.
    pub fn new(
        asset_repo: Box<dyn AssetRepository>,
        category_repo: Box<dyn AssetCategoryRepository>,
        price_repo: Box<dyn AssetPriceRepository>,
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
            .ok_or_else(|| CategoryDomainError::NotFound(dto.category_id.clone()))?;

        let asset = Asset::new(
            dto.name,
            dto.class,
            category,
            dto.currency,
            dto.risk_level,
            dto.reference,
        )?;

        let asset = self.asset_repo.create(asset).await?;
        tracing::info!(target: BACKEND, asset_id = %asset.id, name = %asset.name, "Asset created");

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
            .ok_or_else(|| AssetDomainError::NotFound(dto.asset_id.clone()))?;

        if existing.is_archived {
            return Err(AssetDomainError::Archived.into());
        }

        let category = self
            .get_category_by_id(&dto.category_id)
            .await?
            .ok_or_else(|| CategoryDomainError::NotFound(dto.category_id.clone()))?;

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
        tracing::info!(target: BACKEND, asset_id = %asset.id, name = %asset.name, "Asset updated");

        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AssetUpdated);
        }

        Ok(asset)
    }

    /// Archives an asset (reversible — R6).
    pub async fn archive_asset(&self, asset_id: &str) -> Result<()> {
        self.asset_repo.archive(asset_id).await?;
        tracing::info!(target: BACKEND, asset_id = %asset_id, "Asset archived");
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AssetUpdated);
        }
        Ok(())
    }

    /// Unarchives an asset (R18).
    pub async fn unarchive_asset(&self, asset_id: &str) -> Result<()> {
        self.asset_repo.unarchive(asset_id).await?;
        tracing::info!(target: BACKEND, asset_id = %asset_id, "Asset unarchived");
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AssetUpdated);
        }
        Ok(())
    }

    /// Soft-deletes an asset and publishes an AssetUpdated event.
    pub async fn delete_asset(&self, asset_id: &str) -> Result<()> {
        self.asset_repo.delete(asset_id).await?;
        tracing::info!(target: BACKEND, asset_id = %asset_id, "Asset deleted");
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
            return Err(CategoryDomainError::DuplicateName.into());
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
            return Err(CategoryDomainError::SystemReadonly.into());
        }
        if let Some(existing) = self.category_repo.find_by_name(label).await? {
            if existing.id != id {
                return Err(CategoryDomainError::DuplicateName.into());
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
            return Err(CategoryDomainError::SystemProtected.into());
        }
        self.category_repo
            .reassign_assets_and_delete(category_id, SYSTEM_CATEGORY_ID)
            .await?;
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::CategoryUpdated);
        }
        Ok(())
    }

    // --- Market Price Methods ---

    /// Records (or overwrites) a market price for an asset on a given date (MKT-025).
    /// Validates asset exists (MKT-043), price > 0 (MKT-021), date not in future (MKT-022).
    /// Publishes AssetPriceUpdated on success (MKT-026).
    pub async fn record_price(&self, asset_id: &str, date: &str, price_f64: f64) -> Result<()> {
        // MKT-043 — reject unknown asset
        if self.asset_repo.get_by_id(asset_id).await?.is_none() {
            return Err(AssetDomainError::NotFound(asset_id.to_string()).into());
        }
        // MKT-024 — convert f64 decimal to i64 micros at the IPC boundary
        if !price_f64.is_finite() {
            return Err(AssetPriceDomainError::NonFinite.into());
        }
        let price_micros = (price_f64 * 1_000_000.0).round() as i64;
        // MKT-021, MKT-022 — validate via domain entity factory
        let price = AssetPrice::new(asset_id.to_string(), date.to_string(), price_micros)?;
        // MKT-025 — upsert
        self.price_repo.upsert(price).await?;
        tracing::info!(target: BACKEND, asset_id = %asset_id, date = %date, "Asset price recorded");
        // MKT-026 — publish bare signal event
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AssetPriceUpdated);
        }
        Ok(())
    }

    /// Returns the most recently dated market price for the given asset, or None (MKT-031).
    pub async fn get_latest_price(&self, asset_id: &str) -> Result<Option<AssetPrice>> {
        self.price_repo.get_latest(asset_id).await
    }

    /// Publishes AssetPriceUpdated without performing any write.
    /// Called by the record_transaction use case after an atomic DB commit (MKT-057, B8).
    pub fn notify_asset_price_updated(&self) {
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::AssetPriceUpdated);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::asset::{
        AssetClass, CreateAssetDTO, SqliteAssetCategoryRepository, SqliteAssetPriceRepository,
        SqliteAssetRepository,
    };
    use crate::core::SideEffectEventBus;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_pool() -> sqlx::Pool<sqlx::Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test pool");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");
        pool
    }

    async fn setup_service() -> AssetService {
        let pool = setup_pool().await;
        AssetService::new(
            Box::new(SqliteAssetRepository::new(pool.clone())),
            Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
            Box::new(SqliteAssetPriceRepository::new(pool)),
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
            matches!(
                err.downcast_ref::<AssetDomainError>(),
                Some(AssetDomainError::NameEmpty)
            ),
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
            matches!(
                err.downcast_ref::<AssetDomainError>(),
                Some(AssetDomainError::ReferenceEmpty)
            ),
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
        assert!(
            matches!(
                err.downcast_ref::<AssetDomainError>(),
                Some(AssetDomainError::InvalidCurrency(_))
            ),
            "got: {err}"
        );
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
            matches!(
                err.downcast_ref::<AssetDomainError>(),
                Some(AssetDomainError::InvalidRiskLevel(_))
            ),
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
            matches!(
                err.downcast_ref::<AssetDomainError>(),
                Some(AssetDomainError::Archived)
            ),
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
        assert!(
            matches!(
                err.downcast_ref::<CategoryDomainError>(),
                Some(CategoryDomainError::DuplicateName)
            ),
            "got: {err}"
        );
    }

    // R1 — duplicate name, different case
    #[tokio::test]
    async fn create_category_rejects_duplicate_different_case() {
        let svc = setup_service().await;
        svc.create_category("Bonds").await.unwrap();
        let err = svc.create_category("bonds").await.unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<CategoryDomainError>(),
                Some(CategoryDomainError::DuplicateName)
            ),
            "got: {err}"
        );
    }

    // R2 — system category cannot be renamed
    #[tokio::test]
    async fn update_category_rejects_system_category() {
        let svc = setup_service().await;
        let err = svc
            .update_category(SYSTEM_CATEGORY_ID, "Renamed")
            .await
            .unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<CategoryDomainError>(),
                Some(CategoryDomainError::SystemReadonly)
            ),
            "got: {err}"
        );
    }

    // R1 — update with name already taken by another category
    #[tokio::test]
    async fn update_category_rejects_duplicate_name() {
        let svc = setup_service().await;
        svc.create_category("Bonds").await.unwrap();
        let cat2 = svc.create_category("Equities").await.unwrap();
        let err = svc.update_category(&cat2.id, "bonds").await.unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<CategoryDomainError>(),
                Some(CategoryDomainError::DuplicateName)
            ),
            "got: {err}"
        );
    }

    // R2 — system category cannot be deleted
    #[tokio::test]
    async fn delete_category_rejects_system_category() {
        let svc = setup_service().await;
        let err = svc.delete_category(SYSTEM_CATEGORY_ID).await.unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<CategoryDomainError>(),
                Some(CategoryDomainError::SystemProtected)
            ),
            "got: {err}"
        );
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

    // MKT-043 — record_price rejects unknown asset
    #[tokio::test]
    async fn record_price_rejects_unknown_asset() {
        let svc = setup_service().await;
        let err = svc
            .record_price("nonexistent-id", "2026-01-01", 100.0)
            .await
            .unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<AssetDomainError>(),
                Some(AssetDomainError::NotFound(_))
            ),
            "got: {err}"
        );
    }

    // MKT-021 — record_price rejects price <= 0
    #[tokio::test]
    async fn record_price_rejects_non_positive_price() {
        let svc = setup_service().await;
        let asset = svc.create_asset(base_dto("Apple")).await.unwrap();
        let err = svc
            .record_price(&asset.id, "2026-01-01", 0.0)
            .await
            .unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<AssetPriceDomainError>(),
                Some(AssetPriceDomainError::NotPositive)
            ),
            "got: {err}"
        );
    }

    // MKT-022 — record_price rejects a future date
    #[tokio::test]
    async fn record_price_rejects_future_date() {
        let svc = setup_service().await;
        let asset = svc.create_asset(base_dto("Apple")).await.unwrap();
        let err = svc
            .record_price(&asset.id, "2099-12-31", 100.0)
            .await
            .unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<AssetPriceDomainError>(),
                Some(AssetPriceDomainError::DateInFuture)
            ),
            "got: {err}"
        );
    }

    // MKT-025, MKT-026 — record_price upserts the price and publishes AssetPriceUpdated on success
    #[tokio::test]
    async fn record_price_upserts_and_publishes_event_on_success() {
        let pool = setup_pool().await;
        let bus = Arc::new(SideEffectEventBus::new());
        let mut rx = bus.subscribe();
        let svc = AssetService::new(
            Box::new(SqliteAssetRepository::new(pool.clone())),
            Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
            Box::new(SqliteAssetPriceRepository::new(pool.clone())),
        )
        .with_event_bus(bus);

        let asset = svc.create_asset(base_dto("Apple")).await.unwrap();
        // First record — insert
        svc.record_price(&asset.id, "2026-01-01", 150.5)
            .await
            .unwrap();
        rx.changed().await.unwrap();
        assert_eq!(*rx.borrow(), Event::AssetPriceUpdated);

        // Second record for same date — should overwrite (MKT-025)
        svc.record_price(&asset.id, "2026-01-01", 160.0)
            .await
            .unwrap();
        let latest = svc.get_latest_price(&asset.id).await.unwrap().unwrap();
        assert_eq!(latest.price, 160_000_000); // 160.0 → micros
        assert_eq!(latest.date, "2026-01-01");
    }

    // MKT-057 — notify_asset_price_updated publishes AssetPriceUpdated when a bus is configured.
    // Used by the record_transaction use case after committing an auto-recorded price (B8 — the
    // orchestrator does not publish events directly).
    #[tokio::test]
    async fn notify_asset_price_updated_publishes_event() {
        let pool = setup_pool().await;
        let bus = Arc::new(SideEffectEventBus::new());
        let mut rx = bus.subscribe();
        let svc = AssetService::new(
            Box::new(SqliteAssetRepository::new(pool.clone())),
            Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
            Box::new(SqliteAssetPriceRepository::new(pool.clone())),
        )
        .with_event_bus(bus);

        svc.notify_asset_price_updated();

        rx.changed().await.unwrap();
        assert_eq!(*rx.borrow(), Event::AssetPriceUpdated);
    }

    // MKT-031 — get_latest_price returns None when no price has been recorded for the asset
    #[tokio::test]
    async fn get_latest_price_returns_none_when_no_price_recorded() {
        let svc = setup_service().await;
        let asset = svc.create_asset(base_dto("Apple")).await.unwrap();
        let result = svc.get_latest_price(&asset.id).await.unwrap();
        assert!(result.is_none());
    }

    // MKT-031 — get_latest_price returns the most recently dated price when multiple exist
    #[tokio::test]
    async fn get_latest_price_returns_latest_price_when_one_exists() {
        let svc = setup_service().await;
        let asset = svc.create_asset(base_dto("Apple")).await.unwrap();
        svc.record_price(&asset.id, "2026-01-01", 100.0)
            .await
            .unwrap();
        svc.record_price(&asset.id, "2026-01-03", 120.0)
            .await
            .unwrap();
        svc.record_price(&asset.id, "2026-01-02", 110.0)
            .await
            .unwrap();
        let latest = svc.get_latest_price(&asset.id).await.unwrap().unwrap();
        // Most recent by date is 2026-01-03
        assert_eq!(latest.date, "2026-01-03");
        assert_eq!(latest.price, 120_000_000);
    }
}
