use crate::context::account::AccountService;
use crate::context::asset::AssetService;
use anyhow::{anyhow, Result};
use serde::Serialize;
use specta::Type;
use std::sync::Arc;

/// Enriched view of a single active holding (quantity > 0) with asset metadata (ACD-020).
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
    /// Sum of realized P&L from all Sell transactions for this asset (i64 micro-units, SEL-042).
    pub realized_pnl: i64,
}

/// Enriched view of a fully-closed position (quantity == 0, ACD-044).
#[derive(Debug, Serialize, Clone, Type)]
pub struct ClosedHoldingDetail {
    /// ID of the previously held asset.
    pub asset_id: String,
    /// Display name of the asset.
    pub asset_name: String,
    /// Ticker or user-defined reference.
    pub asset_reference: String,
    /// Total realized P&L for this position (micro-units, ACD-045).
    pub realized_pnl: i64,
    /// ISO date of the most recent sell for this position ("YYYY-MM-DD", ACD-043).
    pub last_sold_date: String,
}

/// Top-level response for the get_account_details command (ACD spec).
#[derive(Debug, Serialize, Clone, Type)]
pub struct AccountDetailsResponse {
    /// Display name of the account (ACD-032).
    pub account_name: String,
    /// Active holdings (quantity > 0), sorted by asset_name asc (ACD-020, ACD-033).
    pub holdings: Vec<HoldingDetail>,
    /// Closed positions (quantity == 0), sorted by asset_name asc (ACD-044, ACD-046).
    pub closed_holdings: Vec<ClosedHoldingDetail>,
    /// Total holding count regardless of quantity (ACD-034).
    pub total_holding_count: i64,
    /// Sum of cost_basis across all active holdings, 0 if none (ACD-031).
    pub total_cost_basis: i64,
    /// Sum of total_realized_pnl across ALL holdings (active + closed), 0 if none (ACD-047).
    pub total_realized_pnl: i64,
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

    /// Builds an AccountDetailsResponse for the given account (ACD-012 to ACD-050).
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
        let total_holding_count = all_holdings.len() as i64;

        // ACD-047 — total realized pnl from ALL holdings (active + closed)
        let total_realized_pnl: i64 = all_holdings.iter().map(|h| h.total_realized_pnl).sum();

        // ACD-020 — active holdings (quantity > 0); ACD-044 — closed (quantity == 0, last_sold_date set)
        let (active_holdings, closed_holdings_raw): (Vec<_>, Vec<_>) =
            all_holdings.into_iter().partition(|h| h.quantity > 0);

        // ACD-022 — enrich each active holding with asset metadata; ACD-021 — archived assets included
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
                realized_pnl: holding.total_realized_pnl,
            });
        }

        // ACD-033 — sort alphabetically by asset_name ascending
        details.sort_by(|a, b| a.asset_name.cmp(&b.asset_name));

        // ACD-031 — sum of cost_basis; 0 when no active holdings
        let total_cost_basis: i64 = details.iter().map(|d| d.cost_basis).sum();

        // ACD-044/ACD-045 — enrich closed positions with asset metadata
        // Only holdings with last_sold_date set are shown (they're genuinely closed)
        let mut closed_details: Vec<ClosedHoldingDetail> =
            Vec::with_capacity(closed_holdings_raw.len());
        for holding in closed_holdings_raw {
            let Some(last_sold_date) = holding.last_sold_date else {
                continue; // ACD-045: skip qty=0 holdings without a sell date
            };
            let asset = self
                .asset_service
                .get_asset_by_id(&holding.asset_id)
                .await?
                .ok_or_else(|| anyhow!("Asset not found: {}", holding.asset_id))?;
            closed_details.push(ClosedHoldingDetail {
                asset_id: holding.asset_id,
                asset_name: asset.name,
                asset_reference: asset.reference,
                realized_pnl: holding.total_realized_pnl,
                last_sold_date,
            });
        }

        // ACD-046 — sort closed holdings by asset_name ascending
        closed_details.sort_by(|a, b| a.asset_name.cmp(&b.asset_name));

        Ok(AccountDetailsResponse {
            account_name: account.name,
            holdings: details,
            closed_holdings: closed_details,
            total_holding_count,
            total_cost_basis,
            total_realized_pnl,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::account::{
        AccountService, Holding, HoldingRepository, SqliteAccountRepository,
        SqliteHoldingRepository, UpdateFrequency,
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
            .create(
                "Test".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
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
        holding_repo
            .upsert(Holding::new(account.id.clone(), asset.id.clone(), 0, 0, 0, None).unwrap())
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
            .create(
                "Portfolio".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
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
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    2_000_000,   // 2.0 units
                    100_000_000, // 100.00
                    0,
                    None,
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
            .create(
                "Archived Test".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
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
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    1_000_000,
                    50_000_000,
                    0,
                    None,
                )
                .unwrap(),
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

    // ACD-032 — account_name is present in the response
    #[tokio::test]
    async fn account_name_present_in_response() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "My Account".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        let resp = uc.get_account_details(&account.id).await.unwrap();
        assert_eq!(resp.account_name, "My Account");
    }

    // ACD-038 — DB or service failure surfaces as an error (not a panic)
    #[tokio::test]
    async fn service_failure_returns_error() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        // nonexistent account triggers anyhow error path, not a panic
        let result = uc.get_account_details("no-such-id").await;
        assert!(result.is_err());
    }

    // ACD-043 — Holding entity exposes last_sold_date: Option<String> and total_realized_pnl: i64
    #[tokio::test]
    async fn holding_entity_carries_last_sold_date_and_total_realized_pnl() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "X".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "A".to_string(),
                reference: "A".to_string(),
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
            })
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    0,
                    50_000_000,
                    15_000_000, // total_realized_pnl
                    Some("2026-01-15".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let holdings = holding_repo.get_by_account(&account.id).await.unwrap();
        assert_eq!(holdings.len(), 1);
        assert_eq!(holdings[0].total_realized_pnl, 15_000_000);
        assert_eq!(holdings[0].last_sold_date.as_deref(), Some("2026-01-15"));
    }

    // ACD-044 — closed_holdings contains holdings with quantity == 0; active holdings do not
    #[tokio::test]
    async fn closed_holdings_contains_zero_qty_holdings() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Acct".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Closed Co".to_string(),
                reference: "CC".to_string(),
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
            })
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    0,
                    50_000_000,
                    5_000_000,
                    Some("2025-12-01".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        let resp = uc.get_account_details(&account.id).await.unwrap();
        assert_eq!(resp.holdings.len(), 0);
        assert_eq!(resp.closed_holdings.len(), 1);
        assert_eq!(resp.closed_holdings[0].asset_reference, "CC");
    }

    // ACD-044 — closed holdings are enriched with asset_name and asset_reference
    #[tokio::test]
    async fn closed_holdings_enriched_with_asset_metadata() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Acct".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Meta Inc".to_string(),
                reference: "META".to_string(),
                class: AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 3,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
            })
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id,
                    0,
                    0,
                    1_000_000,
                    Some("2026-03-10".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        let resp = uc.get_account_details(&account.id).await.unwrap();
        assert_eq!(resp.closed_holdings[0].asset_name, "Meta Inc");
        assert_eq!(resp.closed_holdings[0].asset_reference, "META");
    }

    // ACD-045 — ClosedHoldingDetail.realized_pnl equals Holding.total_realized_pnl
    #[tokio::test]
    async fn closed_holding_detail_realized_pnl_matches_holding() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "P".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Q".to_string(),
                reference: "Q".to_string(),
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
            })
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id,
                    0,
                    0,
                    42_000_000,
                    Some("2026-02-01".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        let resp = uc.get_account_details(&account.id).await.unwrap();
        assert_eq!(resp.closed_holdings[0].realized_pnl, 42_000_000);
    }

    // ACD-045 — last_sold_date on ClosedHoldingDetail is non-optional String from Holding
    #[tokio::test]
    async fn closed_holding_detail_last_sold_date_is_non_optional() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "D".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "E".to_string(),
                reference: "E".to_string(),
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
            })
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id,
                    0,
                    0,
                    0,
                    Some("2025-11-30".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        let resp = uc.get_account_details(&account.id).await.unwrap();
        assert_eq!(resp.closed_holdings[0].last_sold_date, "2025-11-30");
    }

    // ACD-045 — holdings with last_sold_date == None are excluded from closed_holdings
    #[tokio::test]
    async fn holding_without_last_sold_date_excluded_from_closed_holdings() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "F".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "G".to_string(),
                reference: "G".to_string(),
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
            })
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        // qty=0 but no last_sold_date — should not appear in closed_holdings
        holding_repo
            .upsert(Holding::new(account.id.clone(), asset.id, 0, 0, 0, None).unwrap())
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        let resp = uc.get_account_details(&account.id).await.unwrap();
        assert_eq!(resp.closed_holdings.len(), 0);
    }

    // ACD-046 — closed_holdings sorted by asset_name ascending
    #[tokio::test]
    async fn closed_holdings_sorted_by_asset_name_ascending() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "H".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        for (name, reference) in [("Zebra", "ZBR"), ("Alpha", "ALP"), ("Mango", "MNG")] {
            let asset = asset_svc
                .create_asset(CreateAssetDTO {
                    name: name.to_string(),
                    reference: reference.to_string(),
                    class: AssetClass::Stocks,
                    currency: "EUR".to_string(),
                    risk_level: 1,
                    category_id: SYSTEM_CATEGORY_ID.to_string(),
                })
                .await
                .unwrap();
            holding_repo
                .upsert(
                    Holding::new(
                        account.id.clone(),
                        asset.id,
                        0,
                        0,
                        0,
                        Some("2026-01-01".to_string()),
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
        }

        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        let resp = uc.get_account_details(&account.id).await.unwrap();
        let names: Vec<&str> = resp
            .closed_holdings
            .iter()
            .map(|h| h.asset_name.as_str())
            .collect();
        assert_eq!(names, vec!["Alpha", "Mango", "Zebra"]);
    }

    // ACD-047 — total_realized_pnl is sum across ALL holdings (active + closed)
    #[tokio::test]
    async fn total_realized_pnl_sums_active_and_closed_holdings() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "I".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());

        // Active holding with partial sells (pnl = 10)
        let asset1 = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Active".to_string(),
                reference: "ACT".to_string(),
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
            })
            .await
            .unwrap();
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset1.id,
                    1_000_000,
                    50_000_000,
                    10_000_000,
                    Some("2025-06-01".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        // Closed holding (pnl = 25)
        let asset2 = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Closed".to_string(),
                reference: "CLO".to_string(),
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
            })
            .await
            .unwrap();
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset2.id,
                    0,
                    0,
                    25_000_000,
                    Some("2026-01-10".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        let resp = uc.get_account_details(&account.id).await.unwrap();
        assert_eq!(resp.total_realized_pnl, 35_000_000);
    }

    // ACD-047 — total_realized_pnl is 0 when no holdings have realized P&L
    #[tokio::test]
    async fn total_realized_pnl_is_zero_when_no_sells() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "J".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        let resp = uc.get_account_details(&account.id).await.unwrap();
        assert_eq!(resp.total_realized_pnl, 0);
    }

    // ACD-050 — closed_holdings is empty list when no closed positions exist
    #[tokio::test]
    async fn closed_holdings_empty_when_no_closed_positions() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "K".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let uc = AccountDetailsUseCase::new(account_svc, asset_svc);
        let resp = uc.get_account_details(&account.id).await.unwrap();
        assert!(resp.closed_holdings.is_empty());
    }

    // ACD-033 — holdings sorted by asset_name ascending
    #[tokio::test]
    async fn holdings_sorted_by_asset_name_ascending() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create(
                "Alpha".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
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
            holding_repo
                .upsert(
                    Holding::new(account.id.clone(), asset.id, 1_000_000, 50_000_000, 0, None)
                        .unwrap(),
                )
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
