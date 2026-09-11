/// Integration tests for the Price Movement report (PMV spec) — the wiring
/// between `fetch_all_asset_prices`'s `trigger` argument and the
/// `AssetPriceFetchCompleted.movement` payload (PMV-010/011/015).
///
/// Exercises the full stack through the public API: orchestrator constructor →
/// AccountService / AssetService → real in-memory SQLite, mirroring
/// `asset_price_fetch_crud.rs`. No mocks — per test_convention.md Tier 3.
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
use vault_compass_lib::core::event_bus::Event;
use vault_compass_lib::core::SideEffectEventBus;
use vault_compass_lib::use_cases::asset_price_fetch::dispatcher::Dispatcher;
use vault_compass_lib::use_cases::asset_price_fetch::{
    AssetPriceFetchUseCase, FetchGuard, FetchTrigger,
};

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

struct OkProvider;
#[async_trait::async_trait]
impl vault_compass_lib::context::asset::PriceProvider for OkProvider {
    async fn fetch_price(
        &self,
        _symbol: &str,
    ) -> anyhow::Result<Option<vault_compass_lib::context::asset::Quote>> {
        Ok(Some(vault_compass_lib::context::asset::Quote {
            price: 120_000_000,
            date: Some("2026-09-11".to_string()),
        }))
    }
}

struct Ctx {
    use_case: AssetPriceFetchUseCase,
    bus: Arc<SideEffectEventBus>,
    asset_id: String,
}

async fn build_ctx() -> Ctx {
    let pool = make_pool().await;
    let bus = Arc::new(SideEffectEventBus::new());

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

    let asset = asset_service
        .create_asset(CreateAssetDTO {
            name: "Apple".to_string(),
            reference: "AAPL".to_string(),
            isin: None,
            class: AssetClass::Stocks,
            currency: "EUR".to_string(),
            risk_level: 4,
            category_id: SYSTEM_CATEGORY_ID.to_string(),
            exchange: None,
            interest_bearing: false,
        })
        .await
        .expect("seed asset");
    // A baseline price recorded before the refresh, so the "before" reading is non-zero.
    asset_service
        .record_asset_price(&asset.id, "2026-09-01", 100.0)
        .await
        .expect("seed baseline price");

    let account = account_service
        .create(
            "Test".to_string(),
            String::new(),
            "EUR".to_string(),
            UpdateFrequency::ManualMonth,
            false,
        )
        .await
        .expect("seed account");
    account_service
        .open_holding(
            &account.id,
            asset.id.clone(),
            "2024-01-01".to_string(),
            1_000_000,
            100_000_000,
        )
        .await
        .expect("seed holding");

    let dispatcher = Arc::new(Dispatcher::new(
        Arc::new(OkProvider),
        Arc::new(SqliteAssetPriceRepository::new(pool.clone())),
        Arc::clone(&bus),
        Arc::new(CurrencyService::new(
            Box::new(SqliteCurrencyPairRepository::new(pool.clone())),
            Box::new(SqliteCurrencyRateRepository::new(pool.clone())),
        )),
        Arc::new(|| chrono::Local::now().date_naive()),
    ));
    let use_case = AssetPriceFetchUseCase::new(
        account_service,
        asset_service,
        Arc::new(FetchGuard::new()),
        dispatcher,
    );

    Ctx {
        use_case,
        bus,
        asset_id: asset.id,
    }
}

async fn await_completion(bus: &SideEffectEventBus) -> Event {
    let mut rx = bus.subscribe();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            rx.changed()
                .await
                .expect("bus closed before AssetPriceFetchCompleted arrived");
            if matches!(*rx.borrow(), Event::AssetPriceFetchCompleted { .. }) {
                return rx.borrow().clone();
            }
        }
    })
    .await
    .expect("AssetPriceFetchCompleted within timeout")
}

/// PMV-010/011 — a Global refresh (`trigger == Manual`) publishes
/// `AssetPriceFetchCompleted` carrying a present Price Movement report.
/// Exercises the full call chain: command surface → orchestrator → dispatcher
/// → event bus.
#[tokio::test]
async fn fetch_all_with_manual_trigger_publishes_a_price_movement_report() {
    let ctx = build_ctx().await;

    ctx.use_case
        .fetch_all(FetchTrigger::Manual)
        .await
        .expect("dispatch");

    let event = await_completion(&ctx.bus).await;
    let Event::AssetPriceFetchCompleted { movement, .. } = event else {
        panic!("expected AssetPriceFetchCompleted");
    };

    assert!(
        movement.is_some(),
        "a Manual-trigger Global refresh must carry a Price Movement report (PMV-010/011)"
    );
    let report = movement.unwrap();
    assert_eq!(
        report.rows.len(),
        1,
        "one row for the single seeded account"
    );
    let _ = ctx.asset_id;
}

/// PMV-010/015 — the launch auto-fetch (`trigger == Launch`) never reports
/// movement, even though it runs the exact same scope (MKT-130).
#[tokio::test]
async fn fetch_all_with_launch_trigger_publishes_no_price_movement_report() {
    let ctx = build_ctx().await;

    ctx.use_case
        .fetch_all(FetchTrigger::Launch)
        .await
        .expect("dispatch");

    let event = await_completion(&ctx.bus).await;
    let Event::AssetPriceFetchCompleted { movement, .. } = event else {
        panic!("expected AssetPriceFetchCompleted");
    };

    assert!(
        movement.is_none(),
        "the launch auto-fetch must never carry a Price Movement report"
    );
}
