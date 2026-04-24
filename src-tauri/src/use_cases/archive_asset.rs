use crate::context::account::HoldingRepository;
use crate::context::asset::AssetService;
use anyhow::{bail, Result};
use std::sync::Arc;

/// Guards and delegates asset archiving across the asset and account bounded contexts (OQ-6).
pub struct ArchiveAssetUseCase {
    asset_service: Arc<AssetService>,
    holding_repo: Arc<dyn HoldingRepository>,
}

impl ArchiveAssetUseCase {
    /// Creates a new ArchiveAssetUseCase.
    pub fn new(asset_service: Arc<AssetService>, holding_repo: Arc<dyn HoldingRepository>) -> Self {
        Self {
            asset_service,
            holding_repo,
        }
    }

    /// Archives an asset, rejecting the request if any account holds an active position (OQ-6).
    pub async fn archive_asset(&self, asset_id: &str) -> Result<()> {
        if self
            .holding_repo
            .has_active_holdings_for_asset(asset_id)
            .await?
        {
            bail!("Cannot archive an asset with active holdings");
        }
        self.asset_service.archive_asset(asset_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::account::{Holding, HoldingRepository};
    use crate::context::asset::{
        AssetClass, CreateAssetDTO, SqliteAssetCategoryRepository, SqliteAssetRepository,
        SYSTEM_CATEGORY_ID,
    };
    use async_trait::async_trait;
    use sqlx::sqlite::SqlitePoolOptions;

    struct StubHoldingRepo {
        has_active: bool,
    }

    #[async_trait]
    impl HoldingRepository for StubHoldingRepo {
        async fn get_by_account(&self, _: &str) -> Result<Vec<Holding>> {
            unimplemented!()
        }
        async fn get_by_account_asset(&self, _: &str, _: &str) -> Result<Option<Holding>> {
            unimplemented!()
        }
        async fn upsert(&self, _: Holding) -> Result<Holding> {
            unimplemented!()
        }
        async fn delete(&self, _: &str) -> Result<()> {
            unimplemented!()
        }
        async fn delete_by_account_asset(&self, _: &str, _: &str) -> Result<()> {
            unimplemented!()
        }
        async fn has_active_holdings_for_asset(&self, _: &str) -> Result<bool> {
            Ok(self.has_active)
        }
    }

    async fn setup_asset_service() -> (Arc<AssetService>, String) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test pool");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");
        let svc = Arc::new(AssetService::new(
            Box::new(SqliteAssetRepository::new(pool.clone())),
            Box::new(SqliteAssetCategoryRepository::new(pool)),
        ));
        let asset = svc
            .create_asset(CreateAssetDTO {
                name: "Test Asset".to_string(),
                reference: "TST".to_string(),
                class: AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
            })
            .await
            .unwrap();
        (svc, asset.id)
    }

    // OQ-6 — archive rejected when active holding exists
    #[tokio::test]
    async fn archive_rejected_when_active_holding() {
        let (svc, asset_id) = setup_asset_service().await;
        let uc = ArchiveAssetUseCase::new(svc, Arc::new(StubHoldingRepo { has_active: true }));
        let err = uc.archive_asset(&asset_id).await.unwrap_err();
        assert!(err.to_string().contains("active holdings"), "got: {err}");
    }

    // OQ-6 — archive succeeds when no active holdings
    #[tokio::test]
    async fn archive_succeeds_when_no_active_holdings() {
        let (svc, asset_id) = setup_asset_service().await;
        let uc = ArchiveAssetUseCase::new(svc, Arc::new(StubHoldingRepo { has_active: false }));
        uc.archive_asset(&asset_id).await.unwrap();
    }
}
