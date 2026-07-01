/// Integration tests for the `record_management_fee` use-case (FEE spec).
///
/// Exercises the full stack through the public `vault_compass_lib` API:
/// `HoldingTransactionUseCase` → `AccountService` / `AssetService` →
/// `AccountPerformanceUseCase` over real in-memory SQLite. No mocks — per
/// test_convention.md Tier 3 constraint. Mirrors `free_shares_crud.rs`, the
/// quantity-adding sibling of the quantity-reducing fee deduction.
use std::sync::Arc;
use vault_compass_lib::context::account::{
    AccountService, SqliteAccountRepository, SqliteHoldingRepository, SqliteTransactionRepository,
    UpdateFrequency,
};
use vault_compass_lib::context::asset::{
    AssetClass, AssetService, CreateAssetDTO, SqliteAssetCategoryRepository,
    SqliteAssetPriceRepository, SqliteAssetRepository, SYSTEM_CATEGORY_ID,
};
use vault_compass_lib::context::currency::{
    CurrencyService, SqliteCurrencyPairRepository, SqliteCurrencyRateRepository,
};
use vault_compass_lib::core::SideEffectEventBus;
use vault_compass_lib::use_cases::account_performance::AccountPerformanceUseCase;
use vault_compass_lib::use_cases::holding_transaction::HoldingTransactionUseCase;

fn micro(v: i64) -> i64 {
    v * 1_000_000
}

async fn make_pool() -> sqlx::Pool<sqlx::Sqlite> {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
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
    use_case: HoldingTransactionUseCase,
    account_service: Arc<AccountService>,
    asset_service: Arc<AssetService>,
}

async fn build_ctx() -> Ctx {
    let pool = make_pool().await;
    let bus = Arc::new(SideEffectEventBus::new());

    let account_service = Arc::new(
        AccountService::new(
            Box::new(SqliteAccountRepository::new(pool.clone())),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
            Box::new(SqliteTransactionRepository::new(pool.clone())),
        )
        .with_event_bus(Arc::clone(&bus)),
    );
    let asset_service = Arc::new(
        AssetService::new(
            Box::new(SqliteAssetRepository::new(pool.clone())),
            Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
            Box::new(SqliteAssetPriceRepository::new(pool.clone())),
        )
        .with_event_bus(Arc::clone(&bus)),
    );

    let use_case =
        HoldingTransactionUseCase::new(Arc::clone(&account_service), Arc::clone(&asset_service));

    Ctx {
        use_case,
        account_service,
        asset_service,
    }
}

fn stocks_asset_dto(name: &str, reference: &str, currency: &str) -> CreateAssetDTO {
    CreateAssetDTO {
        name: name.to_string(),
        reference: reference.to_string(),
        isin: None,
        class: AssetClass::Stocks,
        currency: currency.to_string(),
        risk_level: 2,
        category_id: SYSTEM_CATEGORY_ID.to_string(),
        exchange: None,
    }
}

// -------------------------------------------------------------------------
// FEE-024 — recording a deduction creates no AssetPrice row
// -------------------------------------------------------------------------

/// FEE-024 — recording a management-fee deduction must not create or modify any
/// AssetPrice record for the charged asset (the deduction is not a price
/// observation) — mirroring FSD-024.
#[tokio::test]
async fn record_management_fee_does_not_create_asset_price_row() {
    // FEE-024 — negative-space test: no AssetPrice write on a fee deduction
    let ctx = build_ctx().await;
    let asset = ctx
        .asset_service
        .create_asset(stocks_asset_dto("AAPL", "AAPL", "USD"))
        .await
        .unwrap();
    let account = ctx
        .account_service
        .create(
            "Portfolio".to_string(),
            "USD".to_string(),
            UpdateFrequency::ManualMonth,
        )
        .await
        .unwrap();

    ctx.use_case
        .record_deposit(&account.id, "2024-01-01".to_string(), micro(1_000), None)
        .await
        .unwrap();
    ctx.use_case
        .buy_holding(
            &account.id,
            asset.id.clone(),
            "2024-01-15".to_string(),
            micro(10),
            micro(50),
            micro(1),
            0,
            None,
        )
        .await
        .unwrap();

    // Record a 1% management fee (removes floor(10 × 1%) = 0.1 units).
    ctx.use_case
        .record_management_fee(
            &account.id,
            asset.id.clone(),
            "2024-06-15".to_string(),
            micro(1),
            None,
        )
        .await
        .unwrap();

    let latest_price = ctx.asset_service.get_latest_price(&asset.id).await.unwrap();
    assert!(
        latest_price.is_none(),
        "recording a management-fee deduction must not create any AssetPrice row (FEE-024)"
    );
}

// -------------------------------------------------------------------------
// FEE-071 — performance treatment: a fee is not a flow nor a dividend
// -------------------------------------------------------------------------

/// FEE-071 — a management-fee deduction is neither an external cash flow nor
/// dividend income: the performance bridge must record cash_flow from the
/// deposit only, asset_flow = 0 (it is not an in-kind contribution like free
/// shares), and dividends = 0. Its drag surfaces through the position's reduced
/// value, the inverse of FSD-070. Mirrors `record_free_shares_performance_neutrality`.
#[tokio::test]
async fn record_management_fee_performance_neutrality() {
    // FEE-071 — fee excluded from cash flows AND dividend totals
    let pool = make_pool().await;

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
    let currency_service = Arc::new(CurrencyService::new(
        Box::new(SqliteCurrencyPairRepository::new(pool.clone())),
        Box::new(SqliteCurrencyRateRepository::new(pool.clone())),
    ));
    let perf_use_case = AccountPerformanceUseCase::new(
        Arc::clone(&account_service),
        Arc::clone(&asset_service),
        currency_service,
    );
    let uc =
        HoldingTransactionUseCase::new(Arc::clone(&account_service), Arc::clone(&asset_service));

    let asset = asset_service
        .create_asset(stocks_asset_dto("AAPL", "AAPL", "USD"))
        .await
        .unwrap();
    let account = account_service
        .create(
            "Portfolio".to_string(),
            "USD".to_string(),
            UpdateFrequency::Automatic,
        )
        .await
        .unwrap();

    uc.record_deposit(&account.id, "2024-01-01".to_string(), micro(1_000), None)
        .await
        .unwrap();
    uc.buy_holding(
        &account.id,
        asset.id.clone(),
        "2024-01-15".to_string(),
        micro(10),
        micro(50),
        micro(1),
        0,
        None,
    )
    .await
    .unwrap();
    uc.record_management_fee(
        &account.id,
        asset.id.clone(),
        "2024-06-01".to_string(),
        micro(1), // 1% of the holding
        None,
    )
    .await
    .unwrap();

    let resp = perf_use_case
        .get_account_performance(&account.id)
        .await
        .unwrap();

    let year_2024 = resp
        .yearly
        .iter()
        .find(|p| p.year == 2024)
        .expect("a 2024 year row must exist in the performance response");

    // FEE-071 — the fee must NOT register as a cash flow: cash_flow reflects only
    // the deposit (deposits − withdrawals = 1000).
    assert_eq!(
        year_2024.cash_flow,
        micro(1_000),
        "management fee must not appear in cash_flow — only the deposit counts (FEE-071)"
    );
    // FEE-071 — the fee is not an in-kind contribution (unlike free shares).
    assert_eq!(
        year_2024.asset_flow, 0,
        "management fee must not appear in asset_flow (FEE-071)"
    );
    // FEE-071 — the fee is not dividend income.
    assert_eq!(
        year_2024.dividends, 0,
        "management fee must not appear in dividend totals (FEE-071)"
    );
}
