/// Integration tests for the account_performance use case (PRF spec).
///
/// Exercises the full stack: AccountPerformanceUseCase → AccountService →
/// AssetService → real in-memory SQLite. Covers happy-path end-to-end and the
/// AccountNotFound error propagation path (PRF-016, PRF-027).
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use vault_compass_lib::context::account::{
    AccountApplicationError, AccountService, SqliteAccountRepository, SqliteHoldingRepository,
    SqliteTransactionRepository, UpdateFrequency,
};
use vault_compass_lib::context::asset::{
    AssetService, CreateAssetDTO, SqliteAssetCategoryRepository, SqliteAssetPriceRepository,
    SqliteAssetRepository, SYSTEM_CATEGORY_ID,
};
use vault_compass_lib::use_cases::account_performance::{
    AccountPerformanceResponse, AccountPerformanceUseCase,
};

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

struct Ctx {
    use_case: AccountPerformanceUseCase,
    account_service: Arc<AccountService>,
    asset_service: Arc<AssetService>,
}

async fn build_ctx(pool: &sqlx::Pool<sqlx::Sqlite>) -> Ctx {
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
    let use_case = AccountPerformanceUseCase::new(account_service.clone(), asset_service.clone());
    Ctx {
        use_case,
        account_service,
        asset_service,
    }
}

// Happy-path end-to-end: deposit → year row with correct end_value and gain (PRF-020, PRF-031)
#[tokio::test]
async fn get_account_performance_deposit_end_to_end() {
    let pool = make_pool().await;
    let ctx = build_ctx(&pool).await;
    ctx.asset_service.seed_cash_asset("EUR").await.unwrap();
    let account = ctx
        .account_service
        .create(
            "E2E Account".to_string(),
            "EUR".to_string(),
            UpdateFrequency::Automatic,
        )
        .await
        .unwrap();
    ctx.account_service
        .record_deposit(&account.id, "2024-06-01".to_string(), 2_000_000_000, None)
        .await
        .unwrap();
    let resp: AccountPerformanceResponse = ctx
        .use_case
        .get_account_performance(&account.id)
        .await
        .unwrap();
    assert_eq!(resp.account_name, "E2E Account");
    assert_eq!(resp.currency, "EUR");
    assert!(
        resp.month_view_available,
        "Automatic → month_view_available"
    );
    let year_2024 = resp
        .yearly
        .iter()
        .find(|p| p.year == 2024)
        .expect("2024 year row");
    assert_eq!(
        year_2024.end_value, 2_000_000_000,
        "end_value for 2024 must equal deposit amount"
    );
    // gain for the first period (no prior): since_inception carries it (PRF-035).
    // end − 0 − 2000 = 0
    let since_inception = year_2024
        .since_inception
        .as_ref()
        .expect("since_inception present for first period");
    assert_eq!(
        since_inception.gain, 0,
        "gain = end − start(0) − deposit = 0"
    );
}

// Error propagation: AccountNotFound surfaces correctly through the full stack (PRF-016)
#[tokio::test]
async fn get_account_performance_not_found_propagates() {
    let pool = make_pool().await;
    let ctx = build_ctx(&pool).await;
    let err = ctx
        .use_case
        .get_account_performance("does-not-exist")
        .await
        .unwrap_err();
    assert!(
        matches!(
            &err,
            AccountApplicationError::AccountNotFound { account_id }
                if account_id == "does-not-exist"
        ),
        "AccountNotFound must propagate with the supplied account_id; got: {err:?}"
    );
}

// End-to-end with a priced EUR stock: end_value = cash + market_value (PRF-020, PRF-022)
#[tokio::test]
async fn get_account_performance_priced_stock_included_in_end_value() {
    let pool = make_pool().await;
    let ctx = build_ctx(&pool).await;
    ctx.asset_service.seed_cash_asset("EUR").await.unwrap();
    let account = ctx
        .account_service
        .create(
            "Stock Account".to_string(),
            "EUR".to_string(),
            UpdateFrequency::Automatic,
        )
        .await
        .unwrap();
    let stock = ctx
        .asset_service
        .create_asset(CreateAssetDTO {
            name: "Blue Chip".to_string(),
            reference: "BLU".to_string(),
            isin: None,
            class: vault_compass_lib::context::asset::AssetClass::Stocks,
            currency: "EUR".to_string(),
            risk_level: 2,
            category_id: SYSTEM_CATEGORY_ID.to_string(),
            exchange: None,
        })
        .await
        .unwrap();
    // Deposit 3000 EUR; buy 2 units at 1000 EUR each → cash residual = 1000 EUR
    ctx.account_service
        .record_deposit(&account.id, "2024-01-02".to_string(), 3_000_000_000, None)
        .await
        .unwrap();
    ctx.account_service
        .buy_holding(
            &account.id,
            stock.id.clone(),
            "2024-01-10".to_string(),
            2_000_000,
            1_000_000_000,
            1_000_000,
            0,
            None,
        )
        .await
        .unwrap();
    // Price at year end: 1200 EUR → market_value = 2 × 1200 = 2400 EUR
    ctx.asset_service
        .record_asset_price(&stock.id, "2024-12-31", 1200.0)
        .await
        .unwrap();
    let resp = ctx
        .use_case
        .get_account_performance(&account.id)
        .await
        .unwrap();
    let year_2024 = resp
        .yearly
        .iter()
        .find(|p| p.year == 2024)
        .expect("2024 row");
    // cash = 1000 EUR, stock = 2400 EUR → end_value = 3400 EUR
    assert_eq!(
        year_2024.end_value, 3_400_000_000,
        "end_value = cash(1000) + stock(2400) = 3400 EUR"
    );
    // gain for the first period (no prior): since_inception carries it (PRF-035).
    // gain = 3400 − 0 − 3000 (deposit) = 400 EUR
    let since_inception = year_2024
        .since_inception
        .as_ref()
        .expect("since_inception present for first period");
    assert_eq!(
        since_inception.gain, 400_000_000,
        "gain must be 400 EUR (stock appreciation)"
    );
}
