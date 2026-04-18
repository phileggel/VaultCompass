use crate::context::account::AccountService;
use crate::context::asset::AssetService;
use anyhow::{anyhow, Result};
use serde::Serialize;
use specta::Type;
use std::sync::Arc;

/// Enriched view of a single holding with asset metadata and computed cost basis (ACD spec).
#[derive(Debug, Serialize, Clone, Type)]
pub struct HoldingDetail {
    /// ID of the held asset.
    pub asset_id: String,
    /// Display name of the asset.
    pub asset_name: String,
    /// Ticker or user-defined reference.
    pub asset_reference: String,
    /// Current units held (i64 micro-units, ADR-001).
    pub quantity: i64,
    /// VWAP purchase price in account currency (i64 micro-units, ADR-001).
    pub average_price: i64,
    /// Total cost of position: quantity × average_price / MICRO (i64 micro-units, ACD-023).
    pub cost_basis: i64,
}

/// Top-level response for the get_account_details command (ACD spec).
#[derive(Debug, Serialize, Clone, Type)]
pub struct AccountDetailsResponse {
    /// Display name of the account (ACD-032).
    pub account_name: String,
    /// Active holdings sorted by asset_name ascending (ACD-020, ACD-033).
    pub holdings: Vec<HoldingDetail>,
    /// Total holding count regardless of quantity (ACD-034).
    pub total_holding_count: usize,
    /// Sum of cost_basis across all active holdings, 0 if none (ACD-031).
    pub total_cost_basis: i64,
}

/// Orchestrates a cross-context read of account + asset data (ADR-003, ADR-004).
pub struct AccountDetailsUseCase {
    account_service: Arc<AccountService>,
    asset_service: Arc<AssetService>,
}

impl AccountDetailsUseCase {
    /// Creates a new use case instance.
    pub fn new(account_service: Arc<AccountService>, asset_service: Arc<AssetService>) -> Self {
        Self {
            account_service,
            asset_service,
        }
    }

    /// Builds an AccountDetailsResponse for the given account (ACD-012 to ACD-041).
    pub async fn get_account_details(&self, account_id: &str) -> Result<AccountDetailsResponse> {
        // ACD-032 — fetch account; bail with not-found if missing (ACD-012)
        let account = self
            .account_service
            .get_by_id(account_id)
            .await?
            .ok_or_else(|| anyhow!("Account not found: {account_id}"))?;

        // ACD-034 — total count before quantity filter
        let all_holdings = self
            .account_service
            .get_holdings_for_account(account_id)
            .await?;
        let total_holding_count = all_holdings.len();

        // ACD-020 — only active holdings (quantity > 0)
        let active_holdings: Vec<_> = all_holdings
            .into_iter()
            .filter(|h| h.quantity > 0)
            .collect();

        // ACD-022 — enrich each holding with asset metadata; ACD-021 — archived assets included
        let mut details: Vec<HoldingDetail> = Vec::with_capacity(active_holdings.len());
        for holding in active_holdings {
            let asset = self
                .asset_service
                .get_asset_by_id(&holding.asset_id)
                .await?
                .ok_or_else(|| anyhow!("Asset not found: {}", holding.asset_id))?;

            // ACD-023/024 — i128 intermediate to prevent overflow before scaling back to i64
            let cost_basis =
                (holding.quantity as i128 * holding.average_price as i128 / 1_000_000) as i64;

            details.push(HoldingDetail {
                asset_id: holding.asset_id,
                asset_name: asset.name,
                asset_reference: asset.reference,
                quantity: holding.quantity,
                average_price: holding.average_price,
                cost_basis,
            });
        }

        // ACD-033 — sort alphabetically by asset_name ascending
        details.sort_by(|a, b| a.asset_name.cmp(&b.asset_name));

        // ACD-031 — sum of cost_basis; 0 when no active holdings
        let total_cost_basis: i64 = details.iter().map(|d| d.cost_basis).sum();

        Ok(AccountDetailsResponse {
            account_name: account.name,
            holdings: details,
            total_holding_count,
            total_cost_basis,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::account::{
        AccountService, SqliteAccountRepository, SqliteHoldingRepository, UpdateFrequency,
    };
    use crate::context::asset::AssetService;
    use crate::context::asset::{
        AssetClass, CreateAssetDTO, SqliteAssetCategoryRepository, SqliteAssetRepository,
        SYSTEM_CATEGORY_ID,
    };
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup(pool: &sqlx::Pool<sqlx::Sqlite>) -> (Arc<AccountService>, Arc<AssetService>) {
        let account_svc = Arc::new(AccountService::new(
            Box::new(SqliteAccountRepository::new(pool.clone())),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
        ));
        let asset_svc = Arc::new(AssetService::new(
            Box::new(SqliteAssetRepository::new(pool.clone())),
            Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
        ));
        (account_svc, asset_svc)
    }

    async fn make_pool() -> sqlx::Pool<sqlx::Sqlite> {
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

    // ACD-012 — unknown account returns error
    #[tokio::test]
    async fn unknown_account_returns_error() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        let err = uc.get_account_details("nonexistent-id").await.unwrap_err();
        assert!(err.to_string().contains("Account not found"), "got: {err}");
    }

    // ACD-020 — holdings with quantity == 0 are excluded; ACD-034 — total_holding_count counts all
    #[tokio::test]
    async fn zero_quantity_holdings_excluded_from_active() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create("Test".to_string(), UpdateFrequency::ManualMonth)
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "AAPL".to_string(),
                reference: "AAPL".to_string(),
                class: AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 3,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
            })
            .await
            .unwrap();

        // Insert a zero-quantity holding directly via repo
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        use crate::context::account::Holding;
        use crate::context::account::HoldingRepository;
        holding_repo
            .upsert(Holding::new(account.id.clone(), asset.id.clone(), 0, 0).unwrap())
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        let resp = uc.get_account_details(&account.id).await.unwrap();

        assert_eq!(resp.holdings.len(), 0, "active holdings should be empty");
        assert_eq!(
            resp.total_holding_count, 1,
            "total count should include zero-qty holding"
        );
        assert_eq!(resp.total_cost_basis, 0);
    }

    // ACD-023/024 — cost basis uses i128 intermediates; ACD-031 — total_cost_basis is sum
    #[tokio::test]
    async fn cost_basis_and_total_computed_correctly() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create("Portfolio".to_string(), UpdateFrequency::ManualMonth)
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Bond".to_string(),
                reference: "BOND".to_string(),
                class: AssetClass::Bonds,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
            })
            .await
            .unwrap();

        // 2.0 units at 100.00 → cost_basis = 200_000_000 micros = 200.00
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        use crate::context::account::Holding;
        use crate::context::account::HoldingRepository;
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    2_000_000,   // 2.0 units
                    100_000_000, // 100.00
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        let resp = uc.get_account_details(&account.id).await.unwrap();

        assert_eq!(resp.holdings.len(), 1);
        assert_eq!(resp.holdings[0].cost_basis, 200_000_000);
        assert_eq!(resp.total_cost_basis, 200_000_000);
    }

    // ACD-021 — holdings for archived assets are included (quantity > 0)
    #[tokio::test]
    async fn archived_asset_holding_included() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create("Archived Test".to_string(), UpdateFrequency::ManualMonth)
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Archived Stock".to_string(),
                reference: "ARCH".to_string(),
                class: AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 2,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
            })
            .await
            .unwrap();

        // Archive the asset
        asset_svc.archive_asset(&asset.id).await.unwrap();

        // Insert a positive-quantity holding for the archived asset
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        use crate::context::account::Holding;
        use crate::context::account::HoldingRepository;
        holding_repo
            .upsert(
                Holding::new(account.id.clone(), asset.id.clone(), 1_000_000, 50_000_000).unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        let resp = uc.get_account_details(&account.id).await.unwrap();

        assert_eq!(
            resp.holdings.len(),
            1,
            "archived asset holding should be included"
        );
        assert_eq!(resp.holdings[0].asset_reference, "ARCH");
    }

    // ACD-033 — holdings sorted by asset_name ascending
    #[tokio::test]
    async fn holdings_sorted_by_asset_name_ascending() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create("Alpha".to_string(), UpdateFrequency::ManualMonth)
            .await
            .unwrap();

        for (name, reference) in [
            ("Zebra Fund", "ZBR"),
            ("Apple Inc", "AAPL"),
            ("Microsoft", "MSFT"),
        ] {
            let asset = asset_svc
                .create_asset(CreateAssetDTO {
                    name: name.to_string(),
                    reference: reference.to_string(),
                    class: AssetClass::Stocks,
                    currency: "USD".to_string(),
                    risk_level: 2,
                    category_id: SYSTEM_CATEGORY_ID.to_string(),
                })
                .await
                .unwrap();
            let holding_repo = SqliteHoldingRepository::new(pool.clone());
            use crate::context::account::Holding;
            use crate::context::account::HoldingRepository;
            holding_repo
                .upsert(Holding::new(account.id.clone(), asset.id, 1_000_000, 50_000_000).unwrap())
                .await
                .unwrap();
        }

        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        let resp = uc.get_account_details(&account.id).await.unwrap();

        let names: Vec<&str> = resp
            .holdings
            .iter()
            .map(|h| h.asset_name.as_str())
            .collect();
        assert_eq!(names, vec!["Apple Inc", "Microsoft", "Zebra Fund"]);
    }
}
