use crate::context::account::{AccountApplicationError, AccountService};
use crate::context::asset::{
    derive_yahoo_symbol_with_exchange, AssetApplicationError, AssetError, AssetPrice,
    AssetPriceRepository, AssetPriceSource, AssetService, HistoricalPriceProvider,
    HistoricalPriceRequest,
};
use crate::core::cash::system_cash_asset_id;
use crate::core::event_bus::Event;
use crate::core::logger::BACKEND;
use crate::core::SideEffectEventBus;
use chrono::NaiveDate;
use serde::Serialize;
use specta::Type;
use std::sync::Arc;

use super::error::{FetchAccountAssetPricesForDateError, FetchPriceForDateTask};

/// Summary of a date-scoped fetch run, surfaced to the modal so it can report how
/// many prices landed and which assets had no data at the chosen date.
#[derive(Debug, Serialize, Type, PartialEq, Eq)]
pub struct FetchForDateOutcome {
    /// Count of fetchable assets whose price was stored at the picked date.
    pub stored: u32,
    /// Names of fetchable assets the provider had no usable price for (sorted),
    /// e.g. the date predates the listing or the symbol was unknown.
    pub missing: Vec<String>,
}

/// Orchestrates the synchronous, per-account, per-date price fetch. Kept fully
/// separate from `AssetPriceFetchUseCase` (latest-price auto-fetch) so the two
/// paths never interfere (ADR-017): different provider trait, no shared in-flight
/// guard, and it awaits every fetch to return a concrete [`FetchForDateOutcome`].
pub struct AssetPriceFetchForDateUseCase {
    account_service: Arc<AccountService>,
    asset_service: Arc<AssetService>,
    provider: Arc<dyn HistoricalPriceProvider>,
    price_repo: Arc<dyn AssetPriceRepository>,
    event_bus: Arc<SideEffectEventBus>,
}

impl AssetPriceFetchForDateUseCase {
    /// Creates a new use case instance.
    pub fn new(
        account_service: Arc<AccountService>,
        asset_service: Arc<AssetService>,
        provider: Arc<dyn HistoricalPriceProvider>,
        price_repo: Arc<dyn AssetPriceRepository>,
        event_bus: Arc<SideEffectEventBus>,
    ) -> Self {
        Self {
            account_service,
            asset_service,
            provider,
            price_repo,
            event_bus,
        }
    }

    /// Fetches each fetchable active holding's close at (or carried back to) `date`
    /// and upserts it keyed to `date`:
    /// (a) reject a malformed or future `date`;
    /// (b) check the account exists, else `AccountNotFound`;
    /// (c) for each non-cash, non-locked active holding with a derivable symbol,
    ///     fetch the historical close — store on success, record the asset name in
    ///     `missing` on no-data / failure;
    /// (d) publish `AssetPriceUpdated` once if anything was stored.
    pub async fn fetch_for_account_on_date(
        &self,
        account_id: &str,
        date: &str,
    ) -> Result<FetchForDateOutcome, FetchAccountAssetPricesForDateError> {
        let parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| FetchPriceForDateTask::InvalidDate)?;
        if parsed > chrono::Local::now().date_naive() {
            return Err(FetchPriceForDateTask::DateInFuture.into());
        }

        let account = self
            .account_service
            .get_by_id(account_id)
            .await?
            .ok_or_else(|| AccountApplicationError::AccountNotFound {
                account_id: account_id.to_string(),
            })?;

        let holdings = self
            .account_service
            .get_holdings_for_account(&account.id)
            .await?;

        let cash_prefix = system_cash_asset_id("");
        let mut stored: u32 = 0;
        let mut missing: Vec<String> = Vec::new();
        for holding in holdings.into_iter().filter(|holding| holding.quantity > 0) {
            if holding.asset_id.starts_with(&cash_prefix) {
                continue;
            }
            let asset = match self.asset_service.get_asset_by_id(&holding.asset_id).await {
                Ok(Some(asset)) => asset,
                Ok(None) => continue,
                Err(application_error) => {
                    tracing::error!(
                        target: BACKEND,
                        asset_id = %holding.asset_id,
                        err = ?application_error,
                        "fetch_for_date: get_asset_by_id failed"
                    );
                    return Err(translate_asset_application_error(application_error).into());
                }
            };
            // ADR-014 / MKT-151 — a locked asset is excluded from fetch scope, same as
            // the latest-price path; its recorded prices are left untouched.
            if asset.price_refresh_blocked {
                continue;
            }
            let Some(symbol) =
                derive_yahoo_symbol_with_exchange(&asset.reference, asset.exchange.as_ref())
            else {
                continue;
            };
            let request = HistoricalPriceRequest {
                symbol: symbol.clone(),
                date: date.to_string(),
            };
            match self.provider.fetch_price_on_date(request).await {
                Ok(Some(quote)) => {
                    // Store under the user-picked date (a carry-back candle is stamped to
                    // the chosen valuation date, not its own earlier date).
                    let record = AssetPrice::restore(
                        asset.id.clone(),
                        date.to_string(),
                        quote.price,
                        AssetPriceSource::YahooFinance,
                    );
                    if let Err(e) = self.price_repo.upsert(record).await {
                        tracing::warn!(
                            target: BACKEND,
                            asset_id = %asset.id,
                            symbol = %symbol,
                            err = ?e,
                            "fetch_for_date: upsert failed; reporting as missing"
                        );
                        missing.push(asset.name.clone());
                        continue;
                    }
                    stored += 1;
                }
                Ok(None) => {
                    tracing::debug!(
                        target: BACKEND,
                        asset_id = %asset.id,
                        symbol = %symbol,
                        date = %date,
                        "fetch_for_date: provider has no data at or before date"
                    );
                    missing.push(asset.name.clone());
                }
                Err(e) => {
                    tracing::warn!(
                        target: BACKEND,
                        asset_id = %asset.id,
                        symbol = %symbol,
                        err = ?e,
                        "fetch_for_date: provider fetch failed; reporting as missing"
                    );
                    missing.push(asset.name.clone());
                }
            }
        }

        if stored > 0 {
            self.event_bus.publish(Event::AssetPriceUpdated);
        }
        missing.sort();
        Ok(FetchForDateOutcome { stored, missing })
    }
}

/// The fetch wire-surface (`AssetError`) exposes only `DatabaseError`; a holding
/// referencing a missing asset mid-fetch is an internal inconsistency surfaced
/// generically, so every variant maps to it.
fn translate_asset_application_error(error: AssetApplicationError) -> AssetError {
    match error {
        AssetApplicationError::NotFound { .. } => AssetError::DatabaseError,
        AssetApplicationError::DatabaseError => AssetError::DatabaseError,
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
        AssetClass, CreateAssetDTO, MockHistoricalPriceProvider, Quote,
        SqliteAssetCategoryRepository, SqliteAssetPriceRepository, SqliteAssetRepository,
        SYSTEM_CATEGORY_ID,
    };
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

    fn services(pool: &sqlx::Pool<sqlx::Sqlite>) -> (Arc<AccountService>, Arc<AssetService>) {
        let account_service = Arc::new(AccountService::new(
            Box::new(SqliteAccountRepository::new(pool.clone())),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
            Box::new(SqliteTransactionRepository::new(pool.clone())),
        ));
        let asset_service = Arc::new(AssetService::new(
            Box::new(SqliteAssetRepository::new(pool.clone())),
            Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
            Box::new(SqliteAssetPriceRepository::new(pool.clone())),
        ));
        (account_service, asset_service)
    }

    fn use_case(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        account_service: Arc<AccountService>,
        asset_service: Arc<AssetService>,
        provider: MockHistoricalPriceProvider,
    ) -> AssetPriceFetchForDateUseCase {
        AssetPriceFetchForDateUseCase::new(
            account_service,
            asset_service,
            Arc::new(provider),
            Arc::new(SqliteAssetPriceRepository::new(pool.clone())),
            Arc::new(SideEffectEventBus::new()),
        )
    }

    async fn seed_account_with_holding(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        account_service: &AccountService,
        asset_service: &AssetService,
    ) -> (String, String) {
        let account = account_service
            .create(
                "Acc".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_service
            .create_asset(CreateAssetDTO {
                name: "Apple".to_string(),
                reference: "AAPL".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(account.id.clone(), asset.id.clone(), 1_000_000, 0, 0, None).unwrap(),
            )
            .await
            .unwrap();
        (account.id, asset.id)
    }

    #[tokio::test]
    async fn rejects_malformed_date() {
        let pool = make_pool().await;
        let (account_service, asset_service) = services(&pool);
        let uc = use_case(
            &pool,
            account_service,
            asset_service,
            MockHistoricalPriceProvider::new(),
        );
        let err = uc
            .fetch_for_account_on_date("any", "not-a-date")
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            FetchAccountAssetPricesForDateError::Failure(FetchPriceForDateTask::InvalidDate)
        ));
    }

    #[tokio::test]
    async fn rejects_future_date() {
        let pool = make_pool().await;
        let (account_service, asset_service) = services(&pool);
        let uc = use_case(
            &pool,
            account_service,
            asset_service,
            MockHistoricalPriceProvider::new(),
        );
        let err = uc
            .fetch_for_account_on_date("any", "2099-12-31")
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            FetchAccountAssetPricesForDateError::Failure(FetchPriceForDateTask::DateInFuture)
        ));
    }

    #[tokio::test]
    async fn unknown_account_returns_account_not_found() {
        let pool = make_pool().await;
        let (account_service, asset_service) = services(&pool);
        let uc = use_case(
            &pool,
            account_service,
            asset_service,
            MockHistoricalPriceProvider::new(),
        );
        let err = uc
            .fetch_for_account_on_date("ghost", "2024-06-10")
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            FetchAccountAssetPricesForDateError::Account(
                AccountApplicationError::AccountNotFound { .. }
            )
        ));
    }

    #[tokio::test]
    async fn stores_fetched_price_at_picked_date() {
        let pool = make_pool().await;
        let (account_service, asset_service) = services(&pool);
        let (account_id, asset_id) =
            seed_account_with_holding(&pool, &account_service, &asset_service).await;
        let mut provider = MockHistoricalPriceProvider::new();
        provider.expect_fetch_price_on_date().returning(|_| {
            Ok(Some(Quote {
                price: 100_000_000,
                date: Some("2024-06-07".to_string()),
            }))
        });
        let uc = use_case(&pool, account_service, asset_service, provider);

        let outcome = uc
            .fetch_for_account_on_date(&account_id, "2024-06-10")
            .await
            .unwrap();
        assert_eq!(outcome.stored, 1);
        assert!(outcome.missing.is_empty());

        // Persisted under the user-picked date, not the carry-back candle date.
        let price_repo = SqliteAssetPriceRepository::new(pool.clone());
        let stored = price_repo
            .get_by_asset_and_date(&asset_id, "2024-06-10")
            .await
            .unwrap()
            .expect("price persisted at picked date");
        assert_eq!(stored.price, 100_000_000);
        assert_eq!(stored.source, AssetPriceSource::YahooFinance);
    }

    #[tokio::test]
    async fn reports_asset_with_no_data_as_missing() {
        let pool = make_pool().await;
        let (account_service, asset_service) = services(&pool);
        let (account_id, _asset_id) =
            seed_account_with_holding(&pool, &account_service, &asset_service).await;
        let mut provider = MockHistoricalPriceProvider::new();
        provider
            .expect_fetch_price_on_date()
            .returning(|_| Ok(None));
        let uc = use_case(&pool, account_service, asset_service, provider);

        let outcome = uc
            .fetch_for_account_on_date(&account_id, "2024-06-10")
            .await
            .unwrap();
        assert_eq!(outcome.stored, 0);
        assert_eq!(outcome.missing, vec!["Apple".to_string()]);
    }
}
