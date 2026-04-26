use crate::context::asset::AssetService;
use crate::context::transaction::TransactionRepository;
use anyhow::{bail, Result};
use std::sync::Arc;

/// Guards and delegates asset hard-deletion across the asset and transaction bounded contexts.
/// Blocks deletion if any transaction references the asset (preserves history integrity).
pub struct DeleteAssetUseCase {
    asset_service: Arc<AssetService>,
    transaction_repo: Arc<dyn TransactionRepository>,
}

impl DeleteAssetUseCase {
    /// Creates a new DeleteAssetUseCase.
    pub fn new(
        asset_service: Arc<AssetService>,
        transaction_repo: Arc<dyn TransactionRepository>,
    ) -> Self {
        Self {
            asset_service,
            transaction_repo,
        }
    }

    /// Deletes an asset, rejecting the request if any transaction references it.
    pub async fn delete_asset(&self, asset_id: &str) -> Result<()> {
        if self
            .transaction_repo
            .has_transactions_for_asset(asset_id)
            .await?
        {
            bail!("Cannot delete an asset with existing transactions");
        }
        self.asset_service.delete_asset(asset_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::asset::{
        AssetClass, CreateAssetDTO, SqliteAssetCategoryRepository, SqliteAssetRepository,
        SYSTEM_CATEGORY_ID,
    };
    use crate::context::transaction::{Transaction, TransactionRepository};
    use async_trait::async_trait;
    use sqlx::sqlite::SqlitePoolOptions;

    struct StubTransactionRepo {
        has_transactions: bool,
    }

    #[async_trait]
    impl TransactionRepository for StubTransactionRepo {
        async fn get_by_id(&self, _: &str) -> Result<Option<Transaction>> {
            unimplemented!()
        }
        async fn get_by_account_asset(&self, _: &str, _: &str) -> Result<Vec<Transaction>> {
            unimplemented!()
        }
        async fn get_asset_ids_for_account(&self, _: &str) -> Result<Vec<String>> {
            unimplemented!()
        }
        async fn get_realized_pnl_by_account(&self, _: &str) -> Result<Vec<(String, i64)>> {
            unimplemented!()
        }
        async fn create(&self, _: Transaction) -> Result<Transaction> {
            unimplemented!()
        }
        async fn update(&self, _: Transaction) -> Result<Transaction> {
            unimplemented!()
        }
        async fn delete(&self, _: &str) -> Result<()> {
            unimplemented!()
        }
        async fn has_transactions_for_asset(&self, _: &str) -> Result<bool> {
            Ok(self.has_transactions)
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
            Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
            Box::new(crate::context::asset::SqliteAssetPriceRepository::new(pool)),
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

    // delete blocked when transactions exist
    #[tokio::test]
    async fn delete_rejected_when_transactions_exist() {
        let (svc, asset_id) = setup_asset_service().await;
        let uc = DeleteAssetUseCase::new(
            svc,
            Arc::new(StubTransactionRepo {
                has_transactions: true,
            }),
        );
        let err = uc.delete_asset(&asset_id).await.unwrap_err();
        assert!(
            err.to_string().contains("existing transactions"),
            "got: {err}"
        );
    }

    // delete succeeds when no transactions exist
    #[tokio::test]
    async fn delete_succeeds_when_no_transactions() {
        let (svc, asset_id) = setup_asset_service().await;
        let uc = DeleteAssetUseCase::new(
            svc,
            Arc::new(StubTransactionRepo {
                has_transactions: false,
            }),
        );
        uc.delete_asset(&asset_id).await.unwrap();
    }
}
