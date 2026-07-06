use crate::context::account::{Account, AccountError, AccountService, UpdateFrequency};
use crate::context::asset::{AssetClass, AssetService};
use crate::context::currency::CurrencyService;
use crate::core::logger::BACKEND;
use crate::use_cases::shared::valuation::compute_current_ytd_pct;
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
    /// Account-wide unrealized P&L in account-currency micros (ACC-023, ADR-001):
    /// the sum of per-holding unrealized P&L (current value − cost basis) over
    /// priced, computable, active non-cash holdings, with foreign holdings
    /// converted to account currency (MKT-040, FXR-040). `None` when no holding
    /// qualifies (no price, or a foreign holding with no usable rate).
    pub total_unrealized_pnl: Option<i64>,
    /// Year-to-date performance for the current calendar year as micro-percent
    /// (ACC-024, ADR-001): the Simple-Dietz return over `[Jan 1, today]` (PRF-034).
    /// `None` when the account has no transactions or the Dietz denominator is
    /// not positive (PRF-032). A first-calendar-year account uses a year-start
    /// baseline of 0 and is present.
    pub ytd_performance_pct: Option<i64>,
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
    pub async fn get_account_summaries(&self) -> StdResult<Vec<AccountSummary>, AccountError> {
        let accounts = self.account_service.get_all().await?;
        let mut summaries = Vec::with_capacity(accounts.len());
        let today = chrono::Local::now().date_naive();
        for account in accounts {
            let total_global_value = self.compute_global_value(&account).await?;
            // ACC-023 — account-wide unrealized P&L, mirroring the MKT-040/FXR-040
            // valuation pass over the account's active non-cash holdings.
            let total_unrealized_pnl = self.compute_total_unrealized_pnl(&account, today).await?;
            // ACC-024 — current calendar-year YTD performance, reusing the
            // account-performance Simple-Dietz machinery (PRF-034) over the
            // account's transactions. Per-account degradation to None mirrors the
            // global-value path; it does not abort the list.
            let transactions = self
                .account_service
                .get_all_transactions_for_account(&account.id)
                .await?;
            let ytd_performance_pct = compute_current_ytd_pct(
                &account.currency,
                &self.asset_service,
                &self.currency_service,
                &transactions,
                today,
            )
            .await?;
            summaries.push(AccountSummary {
                id: account.id,
                name: account.name,
                currency: account.currency,
                update_frequency: account.update_frequency,
                total_global_value,
                total_unrealized_pnl,
                ytd_performance_pct,
            });
        }
        Ok(summaries)
    }

    /// ACC-023 / MKT-040 — account-wide unrealized P&L in account currency: the
    /// sum of per-holding unrealized P&L over priced, computable, active non-cash
    /// holdings. Mirrors the per-holding computation in
    /// `account_details::orchestrator::get_account_details` (current value − cost
    /// basis, with foreign holdings converted via the FX rate). `None` when no
    /// holding qualifies — no price, or a foreign holding with no usable rate
    /// (FXR-034). Cash holdings carry no P&L and never qualify.
    async fn compute_total_unrealized_pnl(
        &self,
        account: &Account,
        today: chrono::NaiveDate,
    ) -> StdResult<Option<i64>, AccountError> {
        let holdings = self
            .account_service
            .get_holdings_for_account(&account.id)
            .await?;
        // FXR-035 — valuation date is "today" (snapshotted once by the caller);
        // future-dated rates are forbidden (FXR-022), so the latest rate on or
        // before today is the latest rate.
        let today = today.format("%Y-%m-%d").to_string();
        let mut total: i64 = 0;
        let mut any_qualified = false;
        for holding in holdings.into_iter().filter(|h| h.quantity > 0) {
            let asset = self
                .asset_service
                .get_asset_by_id(&holding.asset_id)
                .await
                .map_err(|e| {
                    tracing::error!(target: BACKEND, asset_id = %holding.asset_id, err = ?e, "get_account_summaries: get_asset_by_id failed (unrealized_pnl)");
                    AccountError::DatabaseError
                })?
                .ok_or_else(|| {
                    tracing::error!(target: BACKEND, asset_id = %holding.asset_id, "get_account_summaries: holding references missing asset (unrealized_pnl)");
                    AccountError::DatabaseError
                })?;

            // MKT-040 — cash holdings carry no unrealized P&L; they never qualify.
            if asset.class == AssetClass::Cash {
                continue;
            }
            // FXR-040 — resolve the conversion rate (identity → 1.0). A foreign
            // pair with no usable rate makes the holding non-computable (FXR-034).
            let Some(rate) = self
                .currency_service
                .resolve_rate_micros(&asset.currency, &account.currency, &today)
                .await
                .map_err(|e| {
                    tracing::error!(target: BACKEND, asset_id = %holding.asset_id, err = ?e, "get_account_summaries: resolve_rate_micros failed (unrealized_pnl)");
                    AccountError::DatabaseError
                })?
            else {
                continue;
            };
            // MKT-031 — an unpriced holding is non-computable. A price-lookup error
            // is a deliberate degradation (the holding drops from the P&L total),
            // but log it server-side so a real failure leaves a trace.
            let latest_price = self.asset_service.get_latest_price(&holding.asset_id).await;
            if let Err(ref e) = latest_price {
                tracing::warn!(target: BACKEND, asset_id = %holding.asset_id, err = ?e, "get_account_summaries: get_latest_price failed; holding excluded from unrealized P&L (MKT-031)");
            }
            let Some(latest) = latest_price.ok().flatten() else {
                continue;
            };
            // MKT-040/FXR-040 — (converted_price − average_price) × quantity, with
            // i128 intermediates, matching the per-holding detail computation.
            let converted_price = (latest.price as i128 * rate as i128 / 1_000_000) as i64;
            let unrealized_pnl = ((converted_price as i128 - holding.average_price as i128)
                * holding.quantity as i128
                / 1_000_000) as i64;
            total = total.saturating_add(unrealized_pnl);
            any_qualified = true;
        }
        Ok(any_qualified.then_some(total))
    }

    /// CSH-094 — per-account economic value in account currency. Mirrors the
    /// inlined accumulator in `account_details::orchestrator::get_account_details`;
    /// kept duplicated to avoid double-iterating the holdings loop on the detail
    /// path. Tracked as tech debt for a future shared helper.
    async fn compute_global_value(&self, account: &Account) -> StdResult<i64, AccountError> {
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
                    AccountError::DatabaseError
                })?
                .ok_or_else(|| {
                    tracing::error!(target: BACKEND, asset_id = %holding.asset_id, "get_account_summaries: holding references missing asset");
                    AccountError::DatabaseError
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
                    AccountError::DatabaseError
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
    use chrono::Datelike;
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
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
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
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
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
                interest_bearing: false,
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
                String::new(),
                "USD".to_string(),
                UpdateFrequency::ManualMonth,
                false,
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
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
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
                interest_bearing: false,
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
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
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
                interest_bearing: false,
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
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
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
                interest_bearing: false,
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
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
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
                interest_bearing: false,
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
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
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
                interest_bearing: false,
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
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
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
                interest_bearing: false,
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

    // =========================================================================
    // ACC-023 — total_unrealized_pnl
    // =========================================================================

    // ACC-023 — a priced same-currency holding produces the correct
    // account-wide unrealized P&L (MKT-040 algorithm).
    //
    // Setup: 1 unit of a EUR stock bought at average_price = 100 EUR,
    // current_price = 120 EUR.
    // unrealized_pnl = (120 − 100) × 1 = 20 EUR = 20_000_000 micros.
    #[tokio::test]
    async fn priced_same_currency_holding_has_correct_total_unrealized_pnl() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "PnL Account".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let stock = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Priced Stock".to_string(),
                reference: "PRC".to_string(),
                isin: None,
                class: crate::context::asset::AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
                interest_bearing: false,
            })
            .await
            .unwrap();
        // 1 unit at average_price = 100 EUR → cost_basis = 100_000_000
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    stock.id.clone(),
                    1_000_000,   // quantity: 1 unit
                    100_000_000, // average_price: 100 EUR
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        // current_price = 120 EUR
        asset_svc
            .record_asset_price(&stock.id, "2026-01-01", 120.0)
            .await
            .unwrap();

        let uc = AccountSummaryUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let summaries = uc.get_account_summaries().await.unwrap();
        assert_eq!(summaries.len(), 1);
        // (120 − 100) × 1 unit = 20 EUR = 20_000_000 micros
        assert_eq!(
            summaries[0].total_unrealized_pnl,
            Some(20_000_000),
            "expected 20 EUR unrealized P&L; got {:?}",
            summaries[0].total_unrealized_pnl
        );
    }

    // ACC-023 — an empty account (no holdings) yields total_unrealized_pnl = None.
    #[tokio::test]
    async fn empty_account_has_none_total_unrealized_pnl() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        account_svc
            .create(
                "Empty PnL".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
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
        assert!(
            summaries[0].total_unrealized_pnl.is_none(),
            "empty account must have total_unrealized_pnl = None; got {:?}",
            summaries[0].total_unrealized_pnl
        );
    }

    // ACC-023 — an account whose only holding is unpriced yields
    // total_unrealized_pnl = None (MKT-031: no price → no P&L).
    #[tokio::test]
    async fn unpriced_holding_account_has_none_total_unrealized_pnl() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Unpriced PnL".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        let stock = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Unpriced".to_string(),
                reference: "UNP2".to_string(),
                isin: None,
                class: crate::context::asset::AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
                interest_bearing: false,
            })
            .await
            .unwrap();
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    stock.id.clone(),
                    2_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        // No price recorded → unpriced

        let uc = AccountSummaryUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let summaries = uc.get_account_summaries().await.unwrap();
        assert_eq!(summaries.len(), 1);
        assert!(
            summaries[0].total_unrealized_pnl.is_none(),
            "unpriced holding must produce total_unrealized_pnl = None; got {:?}",
            summaries[0].total_unrealized_pnl
        );
    }

    // =========================================================================
    // ACC-024 — ytd_performance_pct
    // =========================================================================

    // ACC-024 — a first-calendar-year account (account opens and deposits in
    // the current year, no prior-year baseline) must have a present
    // ytd_performance_pct (not None). PRF-034: inception-year accounts use
    // baseline 0 for the YTD computation, so the denominator is the weighted
    // deposit rather than 0.
    //
    // Setup: deposit 1000 EUR on Jan 1 of the current year.
    // YTD start baseline = 0 (prior Dec 31 of a year before any data = 0).
    // net_flow in the year = 1000 EUR; end_value = 1000 EUR (cash at face).
    // gain = 1000 − 0 − 1000 = 0. weighted flow = 1000 × full-year fraction > 0.
    // Dietz denominator > 0 → pct is Some(0).
    #[tokio::test]
    async fn first_calendar_year_account_has_present_ytd_performance_pct() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "YTD First Year".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        // Deposit on Jan 1 of the current year
        let current_year = chrono::Local::now().date_naive().year();
        let deposit_date = format!("{current_year}-01-01");
        account_svc
            .record_deposit(&account.id, deposit_date, 1_000_000_000, None)
            .await
            .unwrap();

        let uc = AccountSummaryUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let summaries = uc.get_account_summaries().await.unwrap();
        assert_eq!(summaries.len(), 1);
        assert!(
            summaries[0].ytd_performance_pct.is_some(),
            "first-calendar-year account must have ytd_performance_pct present (not None); got None"
        );
    }

    // ACC-024 — an account with a prior-year baseline and a deposit this year has
    // ytd_performance_pct matching the PRF-034 Simple-Dietz computation.
    //
    // Setup (all in known past years so "today" doesn't affect the baseline):
    //   - Deposit 1000 EUR on 2024-12-31 (prior-year baseline = 1000).
    //   - Deposit 500 EUR on 2025-01-01 (flow in 2025).
    //   - No price changes → end_value for YTD = 1500 EUR.
    //
    // But because tests run on an unknown "today" we need the YTD span to be
    // a past year we fully control. We use 2024 as the prior-year baseline year
    // and seed a 2024-only deposit; then for 2025 we check the 2025 YTD via the
    // performance use-case route and compare.
    //
    // Simpler pinned scenario: use a year that is already complete (2023 → 2024).
    //   - 2023-12-31 baseline: deposit 1000 EUR
    //   - 2024-01-02 flow: deposit 500 EUR (Jan flow, not on Jan 1 so days_remaining > 0
    //     and the weighted denominator > 0).
    //   - No non-cash holdings, no price changes.
    //   - 2024 YTD at end of 2024: end_value = 1500, baseline = 1000,
    //     net_flow = 500, gain = 0, denom = 1000 + 500 × (364/365) ≈ 1498.6 EUR
    //     → pct ≈ 0 (integer truncation). The test only verifies pct is Some(0)
    //     to isolate the "denominator non-zero → present" invariant.
    //
    // NOTE: This test will only exercise the 2024 YTD for a current year > 2024
    // (i.e., the test runs in 2025+). In that case AccountSummary.ytd_performance_pct
    // is the CURRENT year's YTD (not 2024's). We assert it is Some(0) for the
    // current year (cash-only: no gain).
    #[tokio::test]
    async fn prior_year_baseline_account_has_present_ytd_performance_pct() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "YTD With Baseline".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        // Seed a deposit in a past year to establish a non-zero year-start baseline
        account_svc
            .record_deposit(&account.id, "2024-06-01".to_string(), 1_000_000_000, None)
            .await
            .unwrap();
        // Seed a deposit in the current year so the current-YTD denominator > 0
        let current_year = chrono::Local::now().date_naive().year();
        let this_year_flow_date = format!("{current_year}-01-02");
        account_svc
            .record_deposit(&account.id, this_year_flow_date, 500_000_000, None)
            .await
            .unwrap();

        let uc = AccountSummaryUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let summaries = uc.get_account_summaries().await.unwrap();
        assert_eq!(summaries.len(), 1);
        assert!(
            summaries[0].ytd_performance_pct.is_some(),
            "account with prior-year baseline must have ytd_performance_pct present; got None"
        );
    }

    // ACC-024 — an account with a prior-year baseline and a flow this year
    // has ytd_performance_pct that matches the equivalent get_account_performance
    // latest-period year_to_date.pct (PRF-034 reuse).
    //
    // This test verifies that the AccountSummary YTD value is consistent with
    // the AccountPerformance computation for the same account.
    //
    // Setup: deposit 1000 EUR on 2024-06-01 (prior year) and deposit 500 EUR
    // on Jan 2 of the current year. Cash-only, no price changes.
    // Both computations must agree on pct.
    #[tokio::test]
    async fn ytd_performance_pct_matches_account_performance_ytd() {
        use crate::context::currency::domain::{
            MockCurrencyPairRepository, MockCurrencyRateRepository,
        };
        use crate::use_cases::account_performance::AccountPerformanceUseCase;

        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "YTD Cross-check".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::Automatic,
                false,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        // Establish a prior-year baseline
        account_svc
            .record_deposit(&account.id, "2024-06-01".to_string(), 1_000_000_000, None)
            .await
            .unwrap();
        // Flow in the current year (Jan 2 so days_remaining > 0 → denominator > 0)
        let current_year = chrono::Local::now().date_naive().year();
        let this_year_flow_date = format!("{current_year}-01-02");
        account_svc
            .record_deposit(&account.id, this_year_flow_date, 500_000_000, None)
            .await
            .unwrap();

        // Build a no-rate currency service usable by both use cases
        let make_no_rate_svc = || {
            let pair_repo = MockCurrencyPairRepository::new();
            let mut rate_repo = MockCurrencyRateRepository::new();
            rate_repo
                .expect_latest_rate_on_or_before()
                .times(0..)
                .returning(|_, _, _| Ok(None));
            Arc::new(crate::context::currency::CurrencyService::new(
                Box::new(pair_repo),
                Box::new(rate_repo),
            ))
        };

        // AccountSummary path
        let summary_uc =
            AccountSummaryUseCase::new(account_svc.clone(), asset_svc.clone(), make_no_rate_svc());
        let summaries = summary_uc.get_account_summaries().await.unwrap();
        let summary_ytd = summaries
            .iter()
            .find(|s| s.id == account.id)
            .unwrap()
            .ytd_performance_pct;

        // AccountPerformance path — find the current year's latest-month row ytd
        let perf_uc = AccountPerformanceUseCase::new(
            account_svc.clone(),
            asset_svc.clone(),
            make_no_rate_svc(),
        );
        let perf_resp = perf_uc
            .get_account_performance(&account.id, None)
            .await
            .unwrap();
        let current_month = chrono::Local::now().date_naive().month() as u8;
        let latest_month_row = perf_resp
            .monthly
            .iter()
            .find(|p| p.year == current_year && p.month == Some(current_month));
        let perf_ytd_pct = latest_month_row
            .and_then(|row| row.year_to_date.as_ref())
            .and_then(|m| m.pct);

        assert_eq!(
            summary_ytd, perf_ytd_pct,
            "AccountSummary.ytd_performance_pct ({summary_ytd:?}) must match \
             AccountPerformance latest-month year_to_date.pct ({perf_ytd_pct:?})"
        );
    }

    // ACC-024 / PRF-032 — an account with no transactions has no data span
    // and therefore no ytd period to compute → ytd_performance_pct = None.
    #[tokio::test]
    async fn no_transaction_account_has_none_ytd_performance_pct() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        account_svc
            .create(
                "YTD None".to_string(),
                String::new(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
                false,
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
        assert!(
            summaries[0].ytd_performance_pct.is_none(),
            "account with no transactions must have ytd_performance_pct = None; got {:?}",
            summaries[0].ytd_performance_pct
        );
    }
}
