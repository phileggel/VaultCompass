use crate::context::account::{Account, AccountApplicationError, AccountService, UpdateFrequency};
use crate::context::asset::{AssetClass, AssetService};
use crate::context::currency::CurrencyService;
use crate::core::logger::BACKEND;
use serde::Serialize;
use specta::Type;
use std::result::Result as StdResult;
use std::sync::Arc;

/// Row returned by `get_account_summaries` (ACC-021). Pairs each `Account` with its
/// computed `total_global_value` so the Accounts list can render the value column
/// without each row calling `get_account_details` separately.
#[derive(Debug, Serialize, Clone, Type)]
pub struct AccountSummary {
    /// Account identifier (UUID).
    pub id: String,
    /// User-defined display name.
    pub name: String,
    /// ISO 4217 currency code; matches `Account.currency`.
    pub currency: String,
    /// Manual / Automatic update cadence (purely informational, ACC-004).
    pub update_frequency: UpdateFrequency,
    /// Total economic value in account-currency micros (CSH-094 / FXR-041): cash
    /// quantity plus the sum of `quantity × latest_price` over priced active non-cash
    /// holdings, with foreign holdings converted to account currency. Unpriced holdings,
    /// or foreign holdings with no usable rate (FXR-034), contribute 0.
    pub total_global_value: i64,
}

/// Orchestrates a cross-context read of account + asset data to build the
/// Accounts-list view (ACC-021, ADR-003).
pub struct AccountSummaryUseCase {
    account_service: Arc<AccountService>,
    asset_service: Arc<AssetService>,
    currency_service: Arc<CurrencyService>,
}

impl AccountSummaryUseCase {
    /// Creates a new use case instance. The currency service is the valuation
    /// read port for foreign-currency holdings (FXR-041/035).
    pub fn new(
        account_service: Arc<AccountService>,
        asset_service: Arc<AssetService>,
        currency_service: Arc<CurrencyService>,
    ) -> Self {
        Self {
            account_service,
            asset_service,
            currency_service,
        }
    }

    /// Builds a summary row for every non-deleted account.
    pub async fn get_account_summaries(
        &self,
    ) -> StdResult<Vec<AccountSummary>, AccountApplicationError> {
        let accounts = self.account_service.get_all().await?;
        let mut summaries = Vec::with_capacity(accounts.len());
        for account in accounts {
            let total_global_value = self.compute_global_value(&account).await?;
            summaries.push(AccountSummary {
                id: account.id,
                name: account.name,
                currency: account.currency,
                update_frequency: account.update_frequency,
                total_global_value,
            });
        }
        Ok(summaries)
    }

    /// CSH-094 — per-account economic value in account currency. Mirrors the
    /// inlined accumulator in `account_details::orchestrator::get_account_details`;
    /// kept duplicated to avoid double-iterating the holdings loop on the detail
    /// path. Tracked as tech debt for a future shared helper.
    async fn compute_global_value(
        &self,
        account: &Account,
    ) -> StdResult<i64, AccountApplicationError> {
        let holdings = self
            .account_service
            .get_holdings_for_account(&account.id)
            .await?;
        // FXR-035 — valuation date is "today"; future-dated rates are forbidden
        // (FXR-022), so the latest rate on or before today is the latest rate.
        let today = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let mut total: i64 = 0;
        for holding in holdings.into_iter().filter(|h| h.quantity > 0) {
            let asset = self
                .asset_service
                .get_asset_by_id(&holding.asset_id)
                .await
                .map_err(|e| {
                    tracing::error!(target: BACKEND, asset_id = %holding.asset_id, err = ?e, "get_account_summaries: get_asset_by_id failed");
                    AccountApplicationError::DatabaseError
                })?
                .ok_or_else(|| {
                    tracing::error!(target: BACKEND, asset_id = %holding.asset_id, "get_account_summaries: holding references missing asset");
                    AccountApplicationError::DatabaseError
                })?;

            if asset.class == AssetClass::Cash {
                total = total.saturating_add(holding.quantity);
                continue;
            }
            // FXR-041/035 — resolve the conversion rate (identity → 1.0). A foreign
            // pair with no usable rate contributes 0 to the Global Value (FXR-034).
            let Some(rate) = self
                .currency_service
                .resolve_rate_micros(&asset.currency, &account.currency, &today)
                .await
                .map_err(|e| {
                    tracing::error!(target: BACKEND, asset_id = %holding.asset_id, err = ?e, "get_account_summaries: resolve_rate_micros failed");
                    AccountApplicationError::DatabaseError
                })?
            else {
                continue;
            };
            if let Some(latest) = self
                .asset_service
                .get_latest_price(&holding.asset_id)
                .await
                .ok()
                .flatten()
            {
                let converted_price = (latest.price as i128 * rate as i128 / 1_000_000) as i64;
                let market_value =
                    (holding.quantity as i128 * converted_price as i128 / 1_000_000) as i64;
                total = total.saturating_add(market_value);
            }
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::account::{
        AccountService, Holding, HoldingRepository, SqliteAccountRepository,
        SqliteHoldingRepository, SqliteTransactionRepository, UpdateFrequency,
    };
    use crate::context::asset::{
        AssetService, CreateAssetDTO, SqliteAssetCategoryRepository, SqliteAssetPriceRepository,
        SqliteAssetRepository, SYSTEM_CATEGORY_ID,
    };
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup(pool: &sqlx::Pool<sqlx::Sqlite>) -> (Arc<AccountService>, Arc<AssetService>) {
        let account_svc = Arc::new(AccountService::new(
            Box::new(SqliteAccountRepository::new(pool.clone())),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
            Box::new(SqliteTransactionRepository::new(pool.clone())),
        ));
        let asset_svc = Arc::new(AssetService::new(
            Box::new(SqliteAssetRepository::new(pool.clone())),
            Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
            Box::new(SqliteAssetPriceRepository::new(pool.clone())),
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

    // ACC-021 — empty account returns total_global_value = 0
    #[tokio::test]
    async fn empty_account_has_zero_global_value() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Empty".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let uc = AccountSummaryUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let summaries = uc.get_account_summaries().await.unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, account.id);
        assert_eq!(summaries[0].total_global_value, 0);
    }

    // ACC-021 / CSH-094 — cash + same-currency priced non-cash holding aggregate per account
    #[tokio::test]
    async fn aggregates_cash_and_priced_holdings_per_account() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        // Account A: 250 EUR cash + 2 units of a 110-EUR bond → 250 + 220 = 470 EUR
        let acc_a = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&acc_a.id, "2020-01-01".to_string(), 250_000_000, None)
            .await
            .unwrap();
        let bond = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Bond".to_string(),
                reference: "BOND".to_string(),
                isin: None,
                class: crate::context::asset::AssetClass::Bonds,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    acc_a.id.clone(),
                    bond.id.clone(),
                    2_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        asset_svc
            .record_asset_price(&bond.id, "2026-01-01", 110.0)
            .await
            .unwrap();

        // Account B: empty USD account
        let acc_b = account_svc
            .create(
                "B".to_string(),
                "USD".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();

        let uc = AccountSummaryUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let summaries = uc.get_account_summaries().await.unwrap();
        assert_eq!(summaries.len(), 2);

        let row_a = summaries.iter().find(|s| s.id == acc_a.id).unwrap();
        let row_b = summaries.iter().find(|s| s.id == acc_b.id).unwrap();
        assert_eq!(row_a.total_global_value, 470_000_000);
        assert_eq!(row_a.currency, "EUR");
        assert_eq!(row_b.total_global_value, 0);
        assert_eq!(row_b.currency, "USD");
    }

    // FXR-034 — foreign-currency non-cash holding with no usable rate contributes 0
    #[tokio::test]
    async fn foreign_currency_holding_contributes_zero() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "EUR Account".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        // USD-denominated asset held in EUR account; no rate recorded → contributes 0
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "US Stock".to_string(),
                reference: "USX".to_string(),
                isin: None,
                class: crate::context::asset::AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        SqliteHoldingRepository::new(pool.clone())
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
        asset_svc
            .record_asset_price(&asset.id, "2026-01-01", 60.0)
            .await
            .unwrap();

        let uc = AccountSummaryUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let summaries = uc.get_account_summaries().await.unwrap();
        assert_eq!(summaries[0].total_global_value, 0);
    }

    // CSH-094 — unpriced non-cash holding contributes 0 (no fallback to average_price)
    #[tokio::test]
    async fn unpriced_holding_contributes_zero() {
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
                name: "Unpriced".to_string(),
                reference: "UNP".to_string(),
                isin: None,
                class: crate::context::asset::AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    2_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        // No record_asset_price → unpriced

        let uc = AccountSummaryUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let summaries = uc.get_account_summaries().await.unwrap();
        assert_eq!(summaries[0].total_global_value, 0);
    }

    // -------------------------------------------------------------------------
    // FXR-041/ACC-021 — multi-currency global value lift
    // -------------------------------------------------------------------------
    //
    // Setup (mirrors account_details tests):
    //   account currency = EUR, asset currency = USD
    //   quantity = 1_000_000 (1.0 unit), current_price = 110_000_000 USD
    //   rate (USD→EUR) = 1_080_000
    //
    // converted market value = (1_000_000 × 118_800_000) / 1_000_000 = 118_800_000
    //   (where converted_price = (110_000_000 * 1_080_000) / 1_000_000 = 118_800_000)

    use crate::context::currency::{
        application::service::CurrencyService,
        domain::{MockCurrencyPairRepository, MockCurrencyRateRepository},
    };

    fn make_currency_service_with_fixed_rate(rate_micros: i64) -> Arc<CurrencyService> {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo
            .expect_latest_rate_on_or_before()
            .returning(move |_, _, _| {
                Ok(Some(
                    crate::context::currency::domain::CurrencyRate::from_storage(
                        "USD".to_string(),
                        "EUR".to_string(),
                        "2026-01-01".to_string(),
                        rate_micros,
                        crate::context::currency::domain::CurrencyRateSource::Manual,
                    ),
                ))
            });
        Arc::new(CurrencyService::new(
            Box::new(pair_repo),
            Box::new(rate_repo),
        ))
    }

    fn make_currency_service_with_no_rate() -> Arc<CurrencyService> {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo
            .expect_latest_rate_on_or_before()
            .times(0..)
            .returning(|_, _, _| Ok(None));
        Arc::new(CurrencyService::new(
            Box::new(pair_repo),
            Box::new(rate_repo),
        ))
    }

    // FXR-041/ACC-021 — a foreign holding's converted market value is included in
    // total_global_value when a rate is available.
    #[tokio::test]
    async fn foreign_holding_with_rate_included_in_total_global_value() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create(
                "FX Summary".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();

        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "US Stock".to_string(),
                reference: "USX".to_string(),
                isin: None,
                class: crate::context::asset::AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 3,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();

        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    1_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();

        // current_price = 110.00 USD
        asset_svc
            .record_asset_price(&asset.id, "2026-01-01", 110.0)
            .await
            .unwrap();

        let currency_svc = make_currency_service_with_fixed_rate(1_080_000);
        let uc = AccountSummaryUseCase::new(account_svc, asset_svc, currency_svc);
        let summaries = uc.get_account_summaries().await.unwrap();

        // converted_price = (110_000_000 * 1_080_000) / 1_000_000 = 118_800_000
        // market_value = (1_000_000 * 118_800_000) / 1_000_000 = 118_800_000
        assert_eq!(
            summaries[0].total_global_value, 118_800_000,
            "got {}",
            summaries[0].total_global_value
        );
    }

    // Regression — same-currency holding contributes unchanged after FXR lift.
    #[tokio::test]
    async fn same_currency_holding_unchanged_in_summary() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create(
                "Same CCY".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();

        let bond = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Bond".to_string(),
                reference: "BOND".to_string(),
                isin: None,
                class: crate::context::asset::AssetClass::Bonds,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();

        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    bond.id.clone(),
                    2_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();

        asset_svc
            .record_asset_price(&bond.id, "2026-01-01", 110.0)
            .await
            .unwrap();

        // currency service: no call expected for EUR→EUR
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo
            .expect_latest_rate_on_or_before()
            .times(0)
            .returning(|_, _, _| Ok(None));
        let currency_svc = Arc::new(CurrencyService::new(
            Box::new(pair_repo),
            Box::new(rate_repo),
        ));

        let uc = AccountSummaryUseCase::new(account_svc, asset_svc, currency_svc);
        let summaries = uc.get_account_summaries().await.unwrap();

        // 2 units × 110 EUR = 220 EUR = 220_000_000 micros (same as before FXR lift)
        assert_eq!(summaries[0].total_global_value, 220_000_000);
    }

    // FXR-034 — foreign holding with no rate contributes 0 to total_global_value.
    #[tokio::test]
    async fn foreign_holding_without_rate_contributes_zero_to_global_value() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create(
                "No Rate".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();

        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "US Stock".to_string(),
                reference: "USX".to_string(),
                isin: None,
                class: crate::context::asset::AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 3,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();

        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    1_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();

        asset_svc
            .record_asset_price(&asset.id, "2026-01-01", 110.0)
            .await
            .unwrap();

        let currency_svc = make_currency_service_with_no_rate();
        let uc = AccountSummaryUseCase::new(account_svc, asset_svc, currency_svc);
        let summaries = uc.get_account_summaries().await.unwrap();

        assert_eq!(
            summaries[0].total_global_value, 0,
            "foreign holding with no rate must contribute 0; got {}",
            summaries[0].total_global_value
        );
    }

    // CSH-094 — closed holdings (quantity == 0) are excluded
    #[tokio::test]
    async fn closed_holdings_excluded() {
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
                name: "Closed".to_string(),
                reference: "CLS".to_string(),
                isin: None,
                class: crate::context::asset::AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        // quantity = 0 → closed holding
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    0,
                    100_000_000,
                    0,
                    Some("2025-12-01".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        asset_svc
            .record_asset_price(&asset.id, "2026-01-01", 200.0)
            .await
            .unwrap();

        let uc = AccountSummaryUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let summaries = uc.get_account_summaries().await.unwrap();
        assert_eq!(
            summaries[0].total_global_value, 0,
            "closed holdings (quantity = 0) must not contribute"
        );
    }
}
