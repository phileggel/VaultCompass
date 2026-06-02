use crate::context::account::{AccountApplicationError, AccountService};
use crate::context::asset::{
    derive_stooq_symbol_with_exchange, Asset, AssetApplicationError, AssetError, AssetService,
};
use crate::context::currency::CurrencyPair;
use crate::core::cash::system_cash_asset_id;
use crate::core::logger::BACKEND;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::dispatcher::Dispatcher;
use super::error::{FetchAccountAssetPricesError, FetchAllAssetPricesError, FetchPriceTask};
use super::guard::FetchGuard;

/// Orchestrates the asset-price fetch tasks — `fetch_all` (MKT-122 / MKT-130) and
/// `fetch_for_account` (MKT-132 / MKT-131). Both methods share the same in-flight
/// guard, dispatcher, and scope-building logic.
pub struct AssetPriceFetchUseCase {
    account_service: Arc<AccountService>,
    asset_service: Arc<AssetService>,
    fetch_guard: Arc<FetchGuard>,
    dispatcher: Arc<Dispatcher>,
}

impl AssetPriceFetchUseCase {
    /// Creates a new use case instance.
    pub fn new(
        account_service: Arc<AccountService>,
        asset_service: Arc<AssetService>,
        fetch_guard: Arc<FetchGuard>,
        dispatcher: Arc<Dispatcher>,
    ) -> Self {
        Self {
            account_service,
            asset_service,
            fetch_guard,
            dispatcher,
        }
    }

    /// Runs the all-accounts fetch task:
    /// (a) acquire guard or return `FetchAlreadyRunning`;
    /// (b) load all active holdings across all accounts;
    /// (c) filter system cash assets (MKT-116);
    /// (d) derive Stooq symbols, discard non-derivable entries;
    /// (e) if empty scope → `NoFetchableHoldings` (MKT-111);
    /// (f) dispatch background task and return `Ok(())`.
    pub async fn fetch_all(&self) -> Result<(), FetchAllAssetPricesError> {
        let lease = self
            .fetch_guard
            .try_acquire()
            .ok_or(FetchPriceTask::FetchAlreadyRunning)?;

        let accounts = self.account_service.get_all().await?;

        let mut asset_ids: HashSet<String> = HashSet::new();
        // FXR-071 — collect (account_currency, asset_id) for every active holding so
        // foreign pairs can be derived, including assets with no fetchable price.
        let mut fx_inputs: Vec<(String, String)> = Vec::new();
        for account in &accounts {
            let holdings = self
                .account_service
                .get_holdings_for_account(&account.id)
                .await?;
            for holding in holdings {
                if holding.quantity > 0 {
                    fx_inputs.push((account.currency.clone(), holding.asset_id.clone()));
                    asset_ids.insert(holding.asset_id);
                }
            }
        }

        let scope = self.build_scope(asset_ids).await?;
        if scope.is_empty() {
            return Err(FetchPriceTask::NoFetchableHoldings.into());
        }

        let fx_pairs = self.build_fx_pairs(fx_inputs).await?;
        Arc::clone(&self.dispatcher).spawn(scope, fx_pairs, lease);
        Ok(())
    }

    /// Runs the per-account fetch task:
    /// (a) check account exists via `account_service.get_by_id`, else `AccountNotFound` (MKT-132);
    /// (b) acquire guard or return `FetchAlreadyRunning` (MKT-113);
    /// (c) load holdings for the account;
    /// (d) filter system cash assets (MKT-116);
    /// (e) derive Stooq symbols, discard non-derivable entries;
    /// (f) if empty scope → `NoFetchableHoldings` (MKT-111);
    /// (g) dispatch background task and return `Ok(())`.
    pub async fn fetch_for_account(
        &self,
        account_id: &str,
    ) -> Result<(), FetchAccountAssetPricesError> {
        let account = self
            .account_service
            .get_by_id(account_id)
            .await?
            .ok_or_else(|| {
                FetchAccountAssetPricesError::Account(AccountApplicationError::AccountNotFound {
                    account_id: account_id.to_string(),
                })
            })?;

        let lease = self
            .fetch_guard
            .try_acquire()
            .ok_or(FetchPriceTask::FetchAlreadyRunning)?;

        let holdings = self
            .account_service
            .get_holdings_for_account(&account.id)
            .await?;
        let mut asset_ids: HashSet<String> = HashSet::new();
        // FXR-071 — pairs for this account's active foreign holdings.
        let mut fx_inputs: Vec<(String, String)> = Vec::new();
        for holding in holdings.into_iter().filter(|holding| holding.quantity > 0) {
            fx_inputs.push((account.currency.clone(), holding.asset_id.clone()));
            asset_ids.insert(holding.asset_id);
        }

        let scope = self.build_scope(asset_ids).await?;
        if scope.is_empty() {
            return Err(FetchPriceTask::NoFetchableHoldings.into());
        }

        let fx_pairs = self.build_fx_pairs(fx_inputs).await?;
        Arc::clone(&self.dispatcher).spawn(scope, fx_pairs, lease);
        Ok(())
    }

    async fn build_scope(
        &self,
        asset_ids: HashSet<String>,
    ) -> Result<Vec<(Asset, String)>, AssetError> {
        let cash_prefix = system_cash_asset_id("");
        let mut scope: Vec<(Asset, String)> = Vec::new();
        for asset_id in asset_ids {
            if asset_id.starts_with(&cash_prefix) {
                continue;
            }
            let asset = match self.asset_service.get_asset_by_id(&asset_id).await {
                Ok(Some(asset)) => asset,
                Ok(None) => continue,
                Err(application_error) => {
                    tracing::error!(
                        target: BACKEND,
                        asset_id = %asset_id,
                        err = ?application_error,
                        "fetch_scope: get_asset_by_id failed"
                    );
                    return Err(translate_asset_application_error(application_error));
                }
            };
            // MKT-151 / ADR-014 — a locked asset is excluded from fetch scope,
            // preserving its most recently recorded price (same shape as the
            // system-cash exclusion above).
            if asset.price_refresh_blocked {
                continue;
            }
            let Some(symbol) =
                derive_stooq_symbol_with_exchange(&asset.reference, asset.exchange.as_ref())
            else {
                continue;
            };
            scope.push((asset, symbol));
        }
        Ok(scope)
    }

    /// Derives the distinct foreign-currency `CurrencyPair`s (`asset_currency →
    /// account_currency`) for the active holdings in `inputs` (FXR-071/013). Cash
    /// holdings and same-currency holdings are excluded; a missing asset is skipped.
    /// Independent of `build_scope` because a foreign holding still needs its pair
    /// followed even when its price is not auto-fetchable.
    async fn build_fx_pairs(
        &self,
        inputs: Vec<(String, String)>,
    ) -> Result<Vec<CurrencyPair>, AssetError> {
        let cash_prefix = system_cash_asset_id("");
        let mut currency_by_asset: HashMap<String, Option<String>> = HashMap::new();
        let mut seen: HashSet<(String, String)> = HashSet::new();
        let mut pairs: Vec<CurrencyPair> = Vec::new();

        for (account_currency, asset_id) in inputs {
            if asset_id.starts_with(&cash_prefix) {
                continue;
            }
            let asset_currency = match currency_by_asset.get(&asset_id) {
                Some(cached) => cached.clone(),
                None => {
                    let loaded = match self.asset_service.get_asset_by_id(&asset_id).await {
                        Ok(Some(asset)) => Some(asset.currency),
                        Ok(None) => None,
                        Err(application_error) => {
                            tracing::error!(
                                target: BACKEND,
                                asset_id = %asset_id,
                                err = ?application_error,
                                "build_fx_pairs: get_asset_by_id failed"
                            );
                            return Err(translate_asset_application_error(application_error));
                        }
                    };
                    currency_by_asset.insert(asset_id.clone(), loaded.clone());
                    loaded
                }
            };
            let Some(asset_currency) = asset_currency else {
                continue;
            };
            if asset_currency == account_currency {
                continue;
            }
            if seen.insert((asset_currency.clone(), account_currency.clone())) {
                pairs.push(CurrencyPair::from_storage(asset_currency, account_currency));
            }
        }
        Ok(pairs)
    }
}

fn translate_asset_application_error(error: AssetApplicationError) -> AssetError {
    match error {
        AssetApplicationError::DatabaseError => AssetError::DatabaseError,
        _ => AssetError::DatabaseError,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::account::{
        AccountService, SqliteAccountRepository, SqliteHoldingRepository,
        SqliteTransactionRepository,
    };
    use crate::context::asset::{
        AssetCategory, AssetClass, MockAssetRepository, MockPriceProvider,
        SqliteAssetCategoryRepository, SqliteAssetPriceRepository,
    };
    use crate::context::currency::{
        CurrencyService, SqliteCurrencyPairRepository, SqliteCurrencyRateRepository,
    };
    use crate::core::SideEffectEventBus;
    use chrono::NaiveDate;
    use sqlx::sqlite::SqlitePoolOptions;

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

    fn stock_asset(currency: &str) -> Asset {
        Asset::new(
            "Test Asset".to_string(),
            AssetClass::Stocks,
            AssetCategory::default(),
            currency.to_string(),
            2,
            "TST".to_string(),
            None,
            None,
        )
        .expect("valid test asset")
    }

    /// Builds a use case whose asset lookups are driven by `asset_repo`. The account
    /// service and dispatcher are real but inert — `build_fx_pairs` only touches the
    /// asset service, so a mocked asset repository fully controls every branch.
    fn build_use_case(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        asset_repo: MockAssetRepository,
    ) -> AssetPriceFetchUseCase {
        let bus = Arc::new(SideEffectEventBus::new());
        let account_service = Arc::new(AccountService::new(
            Box::new(SqliteAccountRepository::new(pool.clone())),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
            Box::new(SqliteTransactionRepository::new(pool.clone())),
        ));
        let asset_service = Arc::new(AssetService::new(
            Box::new(asset_repo),
            Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
            Box::new(SqliteAssetPriceRepository::new(pool.clone())),
        ));
        let dispatcher = Arc::new(Dispatcher::new(
            Arc::new(MockPriceProvider::new()),
            Arc::new(SqliteAssetPriceRepository::new(pool.clone())),
            Arc::clone(&bus),
            Arc::new(CurrencyService::new(
                Box::new(SqliteCurrencyPairRepository::new(pool.clone())),
                Box::new(SqliteCurrencyRateRepository::new(pool.clone())),
            )),
            Arc::new(|| NaiveDate::from_ymd_opt(2026, 6, 1).expect("valid date")),
        ));
        AssetPriceFetchUseCase::new(
            account_service,
            asset_service,
            Arc::new(FetchGuard::new()),
            dispatcher,
        )
    }

    fn pairs_as_tuples(pairs: &[CurrencyPair]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|pair| (pair.from_currency.clone(), pair.to_currency.clone()))
            .collect()
    }

    // FXR-071 — a foreign holding yields one (asset_currency → account_currency) pair.
    #[tokio::test]
    async fn build_fx_pairs_creates_pair_for_foreign_holding() {
        let pool = make_pool().await;
        let mut asset_repo = MockAssetRepository::new();
        asset_repo
            .expect_get_by_id()
            .returning(|_| Ok(Some(stock_asset("USD"))));
        let use_case = build_use_case(&pool, asset_repo);

        let pairs = use_case
            .build_fx_pairs(vec![("EUR".to_string(), "asset-usd".to_string())])
            .await
            .expect("ok");

        assert_eq!(
            pairs_as_tuples(&pairs),
            vec![("USD".to_string(), "EUR".to_string())]
        );
    }

    // FXR-013 — a holding whose currency equals the account currency yields no pair.
    #[tokio::test]
    async fn build_fx_pairs_skips_same_currency_holding() {
        let pool = make_pool().await;
        let mut asset_repo = MockAssetRepository::new();
        asset_repo
            .expect_get_by_id()
            .returning(|_| Ok(Some(stock_asset("EUR"))));
        let use_case = build_use_case(&pool, asset_repo);

        let pairs = use_case
            .build_fx_pairs(vec![("EUR".to_string(), "asset-eur".to_string())])
            .await
            .expect("ok");

        assert!(
            pairs.is_empty(),
            "same-currency holding must not yield a pair"
        );
    }

    // FXR-071 — a cash holding is filtered before any asset lookup occurs.
    #[tokio::test]
    async fn build_fx_pairs_skips_cash_holding() {
        let pool = make_pool().await;
        let mut asset_repo = MockAssetRepository::new();
        asset_repo.expect_get_by_id().never();
        let use_case = build_use_case(&pool, asset_repo);

        let cash_id = system_cash_asset_id("USD");
        let pairs = use_case
            .build_fx_pairs(vec![("EUR".to_string(), cash_id)])
            .await
            .expect("ok");

        assert!(pairs.is_empty(), "cash holding must not yield a pair");
    }

    // FXR-071 — a holding whose asset no longer exists is skipped without error.
    #[tokio::test]
    async fn build_fx_pairs_skips_missing_asset() {
        let pool = make_pool().await;
        let mut asset_repo = MockAssetRepository::new();
        asset_repo.expect_get_by_id().returning(|_| Ok(None));
        let use_case = build_use_case(&pool, asset_repo);

        let pairs = use_case
            .build_fx_pairs(vec![("EUR".to_string(), "ghost".to_string())])
            .await
            .expect("ok");

        assert!(pairs.is_empty(), "missing asset must not yield a pair");
    }

    // FXR-071 — two foreign holdings resolving to the same pair are de-duplicated.
    #[tokio::test]
    async fn build_fx_pairs_dedups_repeated_pair() {
        let pool = make_pool().await;
        let mut asset_repo = MockAssetRepository::new();
        asset_repo
            .expect_get_by_id()
            .times(2)
            .returning(|_| Ok(Some(stock_asset("USD"))));
        let use_case = build_use_case(&pool, asset_repo);

        let pairs = use_case
            .build_fx_pairs(vec![
                ("EUR".to_string(), "asset-a".to_string()),
                ("EUR".to_string(), "asset-b".to_string()),
            ])
            .await
            .expect("ok");

        assert_eq!(
            pairs_as_tuples(&pairs),
            vec![("USD".to_string(), "EUR".to_string())],
            "the same pair from two assets must appear once"
        );
    }

    // FXR-071 — a repeated asset_id reuses the cached currency (a single lookup).
    #[tokio::test]
    async fn build_fx_pairs_reuses_cached_currency() {
        let pool = make_pool().await;
        let mut asset_repo = MockAssetRepository::new();
        asset_repo
            .expect_get_by_id()
            .times(1)
            .returning(|_| Ok(Some(stock_asset("USD"))));
        let use_case = build_use_case(&pool, asset_repo);

        let pairs = use_case
            .build_fx_pairs(vec![
                ("EUR".to_string(), "asset-a".to_string()),
                ("EUR".to_string(), "asset-a".to_string()),
            ])
            .await
            .expect("ok");

        assert_eq!(
            pairs_as_tuples(&pairs),
            vec![("USD".to_string(), "EUR".to_string())],
            "a repeated asset_id must produce exactly one lookup and one pair"
        );
    }

    // MKT-116 — build_scope surfaces a repository failure as a typed DatabaseError
    // (the asset-load error arm shared with the FX-pair path).
    #[tokio::test]
    async fn build_scope_returns_database_error_when_load_fails() {
        let pool = make_pool().await;
        let mut asset_repo = MockAssetRepository::new();
        asset_repo
            .expect_get_by_id()
            .returning(|_| Err(anyhow::anyhow!("simulated repository failure")));
        let use_case = build_use_case(&pool, asset_repo);

        let mut asset_ids = HashSet::new();
        asset_ids.insert("asset-x".to_string());
        let error = use_case
            .build_scope(asset_ids)
            .await
            .expect_err("must surface a typed error");

        assert!(
            matches!(error, AssetError::DatabaseError),
            "repository failure must map to DatabaseError, got: {error:?}"
        );
    }

    // FXR-071 — a repository failure surfaces as a typed DatabaseError.
    #[tokio::test]
    async fn build_fx_pairs_returns_database_error_when_load_fails() {
        let pool = make_pool().await;
        let mut asset_repo = MockAssetRepository::new();
        asset_repo
            .expect_get_by_id()
            .returning(|_| Err(anyhow::anyhow!("simulated repository failure")));
        let use_case = build_use_case(&pool, asset_repo);

        let error = use_case
            .build_fx_pairs(vec![("EUR".to_string(), "asset-a".to_string())])
            .await
            .expect_err("must surface a typed error");

        assert!(
            matches!(error, AssetError::DatabaseError),
            "repository failure must map to DatabaseError, got: {error:?}"
        );
    }
}
