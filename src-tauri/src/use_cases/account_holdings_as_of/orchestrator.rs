use crate::context::account::{
    Account, AccountError, AccountService, Transaction, TransactionType,
};
use crate::context::asset::AssetService;
use crate::context::currency::CurrencyService;
use crate::core::cash::{is_cash_asset, system_cash_asset_id};
use crate::core::logger::BACKEND;
use crate::use_cases::account_performance::load_priced_assets;
use chrono::{Local, NaiveDate};
use serde::Serialize;
use specta::Type;
use std::collections::HashSet;
use std::result::Result as StdResult;
use std::sync::Arc;

/// Micro-unit scale shared by every monetary field (ADR-001).
const MICRO: i128 = 1_000_000;

/// Narrows an i128 monetary intermediate back to i64 micro-units. Overflow is
/// unrealistic for any real portfolio; the debug assertion guards regressions
/// (matches the truncation guards in `account_performance`).
fn narrow(value: i128) -> i64 {
    debug_assert!(
        value <= i64::MAX as i128 && value >= i64::MIN as i128,
        "monetary value {value} overflows i64 micro-units"
    );
    value as i64
}

/// One reconstructed holding as it stood on the as-of date. All monetary fields
/// are i64 micro-units (ADR-001); foreign holdings are valued in the account
/// currency using the FX rate as of the date.
#[derive(Debug, Serialize, Clone, Type)]
pub struct HoldingAsOf {
    /// ID of the held asset.
    pub asset_id: String,
    /// Display name of the asset.
    pub asset_name: String,
    /// ISO 4217 currency code of the asset's native currency.
    pub asset_currency: String,
    /// Units held on the as-of date (> 0; the Cash Holding carries its balance).
    pub quantity: i64,
    /// VWAP average cost as of the date, account currency (1.0 for the Cash Holding).
    pub average_price: i64,
    /// Cost of the position: quantity × average_price / MICRO (0 for the Cash Holding).
    pub cost_basis: i64,
    /// Market value in account currency. `None` when no price is recorded on or
    /// before the date, or a foreign holding has no usable FX rate as of the date.
    pub market_value: Option<i64>,
    /// Most recent recorded price on or before the date, in the asset's native
    /// currency. `None` when no such price exists (and for the Cash Holding).
    pub price: Option<i64>,
    /// ISO date of the price observation; `None` when `price` is `None`.
    pub price_date: Option<String>,
    /// Unrealized P&L in account currency. `None` under the same conditions as
    /// `market_value` (no price, or no usable FX rate).
    pub unrealized_pnl: Option<i64>,
}

/// Top-level response for `get_account_holdings_as_of` — a read-only valuation of
/// the account's holdings reconstructed as of a past date.
#[derive(Debug, Serialize, Clone, Type)]
pub struct HoldingsAsOfResponse {
    /// Display name of the account.
    pub account_name: String,
    /// The as-of date echoed back ("YYYY-MM-DD").
    pub as_of_date: String,
    /// ISO 4217 currency code of the account.
    pub account_currency: String,
    /// Holdings with quantity > 0 on the date, sorted by asset_name ascending.
    pub holdings: Vec<HoldingAsOf>,
    /// Sum of cost_basis across all holdings (cash contributes 0).
    pub total_cost_basis: i64,
    /// Sum of present market values plus the cash balance.
    pub total_market_value: i64,
}

/// Orchestrates a read-only cross-context reconstruction of an account's holdings
/// as of a past date (ADR-003). Does not touch the live-view orchestrator.
pub struct AccountHoldingsAsOfUseCase {
    account_service: Arc<AccountService>,
    asset_service: Arc<AssetService>,
    currency_service: Arc<CurrencyService>,
}

impl AccountHoldingsAsOfUseCase {
    /// Creates a new use case instance. The currency service is the valuation
    /// read port for foreign-currency holdings.
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

    /// Reconstructs the account's holdings as they stood on `as_of_date`.
    pub async fn get_account_holdings_as_of(
        &self,
        account_id: &str,
        as_of_date: &str,
    ) -> StdResult<HoldingsAsOfResponse, AccountError> {
        // Validate the date: must be ISO YYYY-MM-DD and not in the future.
        let as_of = NaiveDate::parse_from_str(as_of_date, "%Y-%m-%d")
            .map_err(|_| AccountError::InvalidDate)?;
        if as_of > Local::now().date_naive() {
            return Err(AccountError::DateInFuture);
        }

        let account = self
            .account_service
            .get_by_id(account_id)
            .await?
            .ok_or_else(|| AccountError::AccountNotFound {
                account_id: account_id.to_string(),
            })?;

        let transactions = self
            .account_service
            .get_all_transactions_for_account(account_id)
            .await?;

        // Reuse the performance loader for per-asset price history; the as-of
        // carry-forward search runs over the preloaded prices (PricedAsset::price_as_of).
        let priced_assets = load_priced_assets(&self.asset_service, &transactions).await?;

        let mut holdings: Vec<HoldingAsOf> = Vec::new();
        let mut total_cost_basis: i64 = 0;
        let mut total_market_value: i64 = 0;

        // Distinct non-cash asset ids referenced by this account's transactions.
        let mut seen: HashSet<&str> = HashSet::new();
        for transaction in &transactions {
            let asset_id = transaction.asset_id.as_str();
            if is_cash_asset(asset_id) || !seen.insert(asset_id) {
                continue;
            }

            // Reconstruct quantity + VWAP as of the date (TDI-010). Only holdings
            // with a positive quantity on the date are reported.
            let snapshot = Account::holding_snapshot_as_of(&transactions, asset_id, as_of_date);
            if snapshot.quantity <= 0 {
                continue;
            }

            let asset = self
                .asset_service
                .get_asset_by_id(asset_id)
                .await
                .map_err(|e| {
                    tracing::error!(target: BACKEND, asset_id = %asset_id, err = ?e, "get_account_holdings_as_of: get_asset_by_id failed");
                    AccountError::DatabaseError
                })?
                .ok_or_else(|| {
                    tracing::error!(target: BACKEND, asset_id = %asset_id, "get_account_holdings_as_of: transaction references missing asset");
                    AccountError::DatabaseError
                })?;

            let cost_basis =
                narrow(snapshot.quantity as i128 * snapshot.average_price as i128 / MICRO);
            total_cost_basis = total_cost_basis.saturating_add(cost_basis);

            // Carry-forward price as of the date (asset's native currency).
            let priced = priced_assets
                .get(asset_id)
                .and_then(|p| p.price_as_of(as_of));
            let price = priced.as_ref().map(|(value, _)| *value);
            let price_date = priced.as_ref().map(|(_, date)| date.clone());

            // Value in account currency. A same-currency holding resolves to rate
            // 1.0; a foreign holding with no usable rate as of the date yields
            // None, so both market_value and unrealized_pnl stay None.
            let (market_value, unrealized_pnl) = match price {
                Some(price) => {
                    let rate = self
                        .currency_service
                        .resolve_rate_micros(&asset.currency, &account.currency, as_of_date)
                        .await
                        .map_err(|e| {
                            tracing::error!(target: BACKEND, asset_id = %asset_id, err = ?e, "get_account_holdings_as_of: resolve_rate failed");
                            AccountError::DatabaseError
                        })?;
                    match rate {
                        Some(rate) => {
                            let converted_price = narrow(price as i128 * rate as i128 / MICRO);
                            let market_value =
                                narrow(snapshot.quantity as i128 * converted_price as i128 / MICRO);
                            let unrealized_pnl = narrow(
                                (converted_price as i128 - snapshot.average_price as i128)
                                    * snapshot.quantity as i128
                                    / MICRO,
                            );
                            total_market_value = total_market_value.saturating_add(market_value);
                            (Some(market_value), Some(unrealized_pnl))
                        }
                        None => (None, None),
                    }
                }
                None => (None, None),
            };

            holdings.push(HoldingAsOf {
                asset_id: asset_id.to_string(),
                asset_name: asset.name,
                asset_currency: asset.currency,
                quantity: snapshot.quantity,
                average_price: snapshot.average_price,
                cost_basis,
                market_value,
                price,
                price_date,
                unrealized_pnl,
            });
        }

        // Include the system Cash Holding when it carried a balance on the date.
        let cash_balance = cash_balance_as_of(&transactions, as_of_date);
        if cash_balance > 0 {
            let cash_asset_id = system_cash_asset_id(&account.currency);
            let cash_asset = self
                .asset_service
                .get_asset_by_id(&cash_asset_id)
                .await
                .map_err(|e| {
                    tracing::error!(target: BACKEND, asset_id = %cash_asset_id, err = ?e, "get_account_holdings_as_of: get cash asset failed");
                    AccountError::DatabaseError
                })?
                .ok_or_else(|| {
                    tracing::error!(target: BACKEND, asset_id = %cash_asset_id, "get_account_holdings_as_of: cash asset missing");
                    AccountError::DatabaseError
                })?;
            total_market_value = total_market_value.saturating_add(cash_balance);
            holdings.push(HoldingAsOf {
                asset_id: cash_asset_id,
                asset_name: cash_asset.name,
                asset_currency: account.currency.clone(),
                quantity: cash_balance,
                average_price: MICRO as i64,
                cost_basis: 0,
                market_value: Some(cash_balance),
                price: None,
                price_date: None,
                unrealized_pnl: None,
            });
        }

        holdings.sort_by(|a, b| a.asset_name.cmp(&b.asset_name));

        Ok(HoldingsAsOfResponse {
            account_name: account.name,
            as_of_date: as_of_date.to_string(),
            account_currency: account.currency,
            holdings,
            total_cost_basis,
            total_market_value,
        })
    }
}

/// Cash balance as of `as_of_date` (inclusive), reconstructed from the cash-
/// affecting transactions: Deposit / Sell / Dividend credit, Withdrawal /
/// Purchase debit. ISO `YYYY-MM-DD` dates compare lexicographically, so a string
/// cut-off matches the chronological one (TDI-011). Clamped at 0.
fn cash_balance_as_of(transactions: &[Transaction], as_of_date: &str) -> i64 {
    let mut balance: i128 = 0;
    for transaction in transactions {
        if transaction.date.as_str() > as_of_date {
            continue;
        }
        match transaction.transaction_type {
            TransactionType::Deposit | TransactionType::Sell | TransactionType::Dividend => {
                balance += transaction.total_amount as i128;
            }
            TransactionType::Withdrawal | TransactionType::Purchase => {
                balance -= transaction.total_amount as i128;
            }
            TransactionType::OpeningBalance | TransactionType::FreeShares => {}
        }
    }
    narrow(balance.max(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::account::{
        SqliteAccountRepository, SqliteHoldingRepository, SqliteTransactionRepository,
        UpdateFrequency,
    };
    use crate::context::asset::{
        AssetClass, CreateAssetDTO, SqliteAssetCategoryRepository, SqliteAssetPriceRepository,
        SqliteAssetRepository, SYSTEM_CATEGORY_ID,
    };
    use crate::context::currency::{
        application::service::CurrencyService,
        domain::{MockCurrencyPairRepository, MockCurrencyRateRepository},
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
                        "2024-01-01".to_string(),
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

    async fn make_stock(asset_svc: &AssetService, name: &str, currency: &str) -> String {
        asset_svc
            .create_asset(CreateAssetDTO {
                name: name.to_string(),
                reference: name.to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: currency.to_string(),
                risk_level: 3,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap()
            .id
    }

    // Malformed date string is rejected with InvalidDate before any lookup.
    #[tokio::test]
    async fn malformed_date_returns_invalid_date() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let uc = AccountHoldingsAsOfUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let err = uc
            .get_account_holdings_as_of("any-id", "not-a-date")
            .await
            .unwrap_err();
        assert!(matches!(err, AccountError::InvalidDate), "got: {err:?}");
    }

    // A future as-of date is rejected with DateInFuture.
    #[tokio::test]
    async fn future_date_returns_date_in_future() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let uc = AccountHoldingsAsOfUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let err = uc
            .get_account_holdings_as_of("any-id", "2999-12-31")
            .await
            .unwrap_err();
        assert!(matches!(err, AccountError::DateInFuture), "got: {err:?}");
    }

    // Unknown account returns AccountNotFound (after date validation passes).
    #[tokio::test]
    async fn unknown_account_returns_not_found() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let uc = AccountHoldingsAsOfUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let err = uc
            .get_account_holdings_as_of("nonexistent", "2024-06-01")
            .await
            .unwrap_err();
        assert!(
            matches!(&err, AccountError::AccountNotFound { account_id } if account_id == "nonexistent"),
            "got: {err:?}"
        );
    }

    // A holding opened AFTER the as-of date is excluded; one opened before is
    // reconstructed with the as-of-date quantity + VWAP and priced as of the date.
    #[tokio::test]
    async fn reconstructs_holdings_as_of_date_excluding_later_openings() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Acc".to_string(),
                "EUR".to_string(),
                UpdateFrequency::Automatic,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2024-01-01".to_string(), 1_000_000_000, None)
            .await
            .unwrap();

        let early = make_stock(&asset_svc, "Early", "EUR").await;
        let late = make_stock(&asset_svc, "Late", "EUR").await;

        // Early: buy 2 @ 100 on 2024-02-01 (cost 200).
        account_svc
            .buy_holding(
                &account.id,
                early.clone(),
                "2024-02-01".to_string(),
                2_000_000,
                100_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();
        // Late: buy 5 @ 50 on 2024-08-01 — after the as-of date.
        account_svc
            .buy_holding(
                &account.id,
                late.clone(),
                "2024-08-01".to_string(),
                5_000_000,
                50_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();

        // Price for Early: 120 on 2024-03-01 (carry-forward to the as-of date).
        asset_svc
            .record_asset_price(&early, "2024-03-01", 120.0)
            .await
            .unwrap();

        let uc = AccountHoldingsAsOfUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc
            .get_account_holdings_as_of(&account.id, "2024-06-01")
            .await
            .unwrap();

        // "Late" excluded; "Early" and the cash row present.
        let early_row = resp
            .holdings
            .iter()
            .find(|h| h.asset_name == "Early")
            .expect("Early present");
        assert!(
            resp.holdings.iter().all(|h| h.asset_name != "Late"),
            "Late opened after the date must be excluded"
        );
        assert_eq!(early_row.quantity, 2_000_000);
        assert_eq!(early_row.average_price, 100_000_000);
        assert_eq!(early_row.cost_basis, 200_000_000);
        assert_eq!(early_row.price, Some(120_000_000));
        assert_eq!(early_row.price_date.as_deref(), Some("2024-03-01"));
        // Same-currency market value = 2 × 120 = 240; unrealized = (120-100) × 2 = 40.
        assert_eq!(early_row.market_value, Some(240_000_000));
        assert_eq!(early_row.unrealized_pnl, Some(40_000_000));

        // Cash on the date = 1000 deposit − 200 buy = 800.
        let cash = resp
            .holdings
            .iter()
            .find(|h| is_cash_asset(&h.asset_id))
            .expect("cash row present");
        assert_eq!(cash.quantity, 800_000_000);
        assert_eq!(cash.market_value, Some(800_000_000));

        assert_eq!(resp.total_cost_basis, 200_000_000);
        // total market value = 240 (Early) + 800 (cash) = 1040.
        assert_eq!(resp.total_market_value, 1_040_000_000);
        assert_eq!(resp.as_of_date, "2024-06-01");
    }

    // A partial sell BEFORE the as-of date lowers the quantity but preserves VWAP.
    #[tokio::test]
    async fn partial_sell_before_date_reduces_quantity_preserving_vwap() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Acc".to_string(),
                "EUR".to_string(),
                UpdateFrequency::Automatic,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2024-01-01".to_string(), 1_000_000_000, None)
            .await
            .unwrap();
        let stock = make_stock(&asset_svc, "Stock", "EUR").await;
        account_svc
            .buy_holding(
                &account.id,
                stock.clone(),
                "2024-02-01".to_string(),
                4_000_000,
                100_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();
        // Sell 1 @ 150 on 2024-03-01 — before the as-of date.
        account_svc
            .sell_holding(
                &account.id,
                stock.clone(),
                "2024-03-01".to_string(),
                1_000_000,
                150_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();
        asset_svc
            .record_asset_price(&stock, "2024-02-15", 100.0)
            .await
            .unwrap();

        let uc = AccountHoldingsAsOfUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc
            .get_account_holdings_as_of(&account.id, "2024-06-01")
            .await
            .unwrap();
        let row = resp
            .holdings
            .iter()
            .find(|h| h.asset_name == "Stock")
            .expect("Stock present");
        assert_eq!(row.quantity, 3_000_000, "4 bought − 1 sold = 3");
        assert_eq!(
            row.average_price, 100_000_000,
            "VWAP preserved across the sell"
        );
    }

    // A foreign holding is valued using the FX rate as of the date.
    #[tokio::test]
    async fn foreign_holding_valued_with_fx_rate() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Acc".to_string(),
                "EUR".to_string(),
                UpdateFrequency::Automatic,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2024-01-01".to_string(), 1_000_000_000, None)
            .await
            .unwrap();
        let stock = make_stock(&asset_svc, "US Co", "USD").await;
        // 2 units, avg 100.00 EUR (cost basis already in account currency).
        account_svc
            .buy_holding(
                &account.id,
                stock.clone(),
                "2024-02-01".to_string(),
                2_000_000,
                100_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();
        asset_svc
            .record_asset_price(&stock, "2024-03-01", 110.0)
            .await
            .unwrap();

        let uc = AccountHoldingsAsOfUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_fixed_rate(1_080_000),
        );
        let resp = uc
            .get_account_holdings_as_of(&account.id, "2024-06-01")
            .await
            .unwrap();
        let row = resp
            .holdings
            .iter()
            .find(|h| h.asset_name == "US Co")
            .expect("US Co present");
        // converted_price = 110 × 1.08 = 118.8; market value = 2 × 118.8 = 237.6.
        assert_eq!(row.price, Some(110_000_000));
        assert_eq!(row.market_value, Some(237_600_000));
        // unrealized = (118.8 − 100) × 2 = 37.6.
        assert_eq!(row.unrealized_pnl, Some(37_600_000));
    }

    // A foreign holding with no usable rate has None market value + None P&L,
    // but the raw asset-currency price is still reported.
    #[tokio::test]
    async fn foreign_holding_without_rate_has_none_value() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Acc".to_string(),
                "EUR".to_string(),
                UpdateFrequency::Automatic,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2024-01-01".to_string(), 1_000_000_000, None)
            .await
            .unwrap();
        let stock = make_stock(&asset_svc, "US Co", "USD").await;
        account_svc
            .buy_holding(
                &account.id,
                stock.clone(),
                "2024-02-01".to_string(),
                2_000_000,
                100_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();
        asset_svc
            .record_asset_price(&stock, "2024-03-01", 110.0)
            .await
            .unwrap();

        let uc = AccountHoldingsAsOfUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc
            .get_account_holdings_as_of(&account.id, "2024-06-01")
            .await
            .unwrap();
        let row = resp
            .holdings
            .iter()
            .find(|h| h.asset_name == "US Co")
            .expect("US Co present");
        assert_eq!(row.price, Some(110_000_000));
        assert!(row.market_value.is_none());
        assert!(row.unrealized_pnl.is_none());
    }
}
