//! Integration tests for enabling as the first device (SYN-013/026) through the crate's public
//! API: seed a small portfolio via the real BC services, enable sync into a tempdir, and assert
//! the folder + database wiring is genuinely reachable end-to-end.
//!
//! Per `test_convention.md` Tier 3: only the public API is used, and `sync_now` is exercised
//! through the same public surface a second time after a new transaction.

use std::sync::Arc;

use vault_compass_lib::context::account::{
    AccountService, SqliteAccountRepository, SqliteFeeCatchUpRepository,
    SqliteFeeScheduleRepository, SqliteHoldingNoteRepository, SqliteHoldingRepository,
    SqliteTransactionRepository, UpdateFrequency,
};
use vault_compass_lib::context::asset::{
    AssetClass, AssetService, CreateAssetDTO, SqliteAssetCategoryRepository,
    SqliteAssetPriceRepository, SqliteAssetRepository, SYSTEM_CATEGORY_ID,
};
use vault_compass_lib::context::currency::{
    CurrencyService, SqliteCurrencyPairRepository, SqliteCurrencyRateRepository,
};
use vault_compass_lib::context::sync::{
    FirstPublish, FsFolderStore, SqliteChangeLogRepository, SqliteChangeRecorder,
    SqliteSyncStateRepository, SyncRun,
};
use vault_compass_lib::shared::infrastructure::change_recorder::ChangeRecorder;
use vault_compass_lib::use_cases::portfolio_sync::{
    PortfolioSyncOrchestrator, ServicePortfolioSnapshot, ServiceRankStamper,
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

struct Ctx {
    orchestrator: PortfolioSyncOrchestrator,
    account_service: Arc<AccountService>,
    asset_service: Arc<AssetService>,
    pool: sqlx::Pool<sqlx::Sqlite>,
}

async fn build_ctx(folder: &std::path::Path) -> Ctx {
    let pool = make_pool().await;
    // Every synced repository records through the real change recorder, as the production
    // container wires it (SYN-020): `sync_now` publishes what it captured.
    let recorder: Arc<dyn ChangeRecorder> = Arc::new(SqliteChangeRecorder::new(pool.clone()));
    let account_service = Arc::new(
        AccountService::new(
            Box::new(
                SqliteAccountRepository::new(pool.clone()).with_change_recorder(recorder.clone()),
            ),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
            Box::new(
                SqliteTransactionRepository::new(pool.clone())
                    .with_change_recorder(recorder.clone()),
            ),
        )
        .with_fee_schedule_repo(Box::new(
            SqliteFeeScheduleRepository::new(pool.clone()).with_change_recorder(recorder.clone()),
        ))
        .with_fee_catch_up_repo(Box::new(
            SqliteFeeCatchUpRepository::new(pool.clone()).with_change_recorder(recorder.clone()),
        ))
        .with_holding_note_repo(Box::new(
            SqliteHoldingNoteRepository::new(pool.clone()).with_change_recorder(recorder.clone()),
        )),
    );
    let asset_service = Arc::new(AssetService::new(
        Box::new(SqliteAssetRepository::new(pool.clone()).with_change_recorder(recorder.clone())),
        Box::new(
            SqliteAssetCategoryRepository::new(pool.clone()).with_change_recorder(recorder.clone()),
        ),
        Box::new(
            SqliteAssetPriceRepository::new(pool.clone()).with_change_recorder(recorder.clone()),
        ),
    ));
    let currency_service = Arc::new(CurrencyService::new(
        Box::new(
            SqliteCurrencyPairRepository::new(pool.clone()).with_change_recorder(recorder.clone()),
        ),
        Box::new(SqliteCurrencyRateRepository::new(pool.clone()).with_change_recorder(recorder)),
    ));
    let state_repo = Arc::new(SqliteSyncStateRepository::new(pool.clone()));
    let folder_store = Arc::new(FsFolderStore::new(folder));
    let change_log = Arc::new(SqliteChangeLogRepository::new(pool.clone()));
    let sync_run = Arc::new(SyncRun::new(
        change_log.clone(),
        state_repo.clone(),
        folder_store.clone(),
    ));
    let sync_service = Arc::new(
        vault_compass_lib::context::sync::SyncService::new(
            state_repo.clone(),
            folder_store.clone(),
        )
        .with_run(sync_run.clone()),
    );
    let snapshot = Arc::new(ServicePortfolioSnapshot::new(
        account_service.clone(),
        asset_service.clone(),
        currency_service.clone(),
    ));
    let rank_stamper = Arc::new(ServiceRankStamper::new(
        account_service.clone(),
        asset_service.clone(),
        currency_service.clone(),
    ));
    let first_publish = Arc::new(FirstPublish::new(
        change_log,
        state_repo.clone(),
        folder_store.clone(),
        rank_stamper,
        snapshot,
    ));
    let orchestrator = PortfolioSyncOrchestrator::new(
        account_service.clone(),
        asset_service.clone(),
        currency_service,
        sync_service,
        first_publish,
        sync_run,
        state_repo,
        folder_store,
    );
    Ctx {
        orchestrator,
        account_service,
        asset_service,
        pool,
    }
}

async fn seed_small_portfolio(ctx: &Ctx) -> (String, String) {
    let asset = ctx
        .asset_service
        .create_asset(CreateAssetDTO {
            name: "AAPL".into(),
            reference: "AAPL".into(),
            isin: None,
            class: AssetClass::Stocks,
            currency: "USD".into(),
            risk_level: 2,
            category_id: SYSTEM_CATEGORY_ID.into(),
            exchange: None,
            interest_bearing: false,
        })
        .await
        .unwrap();
    ctx.asset_service.seed_cash_asset("USD").await.unwrap();
    let account = ctx
        .account_service
        .create(
            "Portfolio".into(),
            String::new(),
            "USD".into(),
            UpdateFrequency::ManualMonth,
            false,
        )
        .await
        .unwrap();
    ctx.account_service
        .seed_cash_holding(&account.id)
        .await
        .unwrap();
    ctx.account_service
        .record_deposit(&account.id, "2026-01-01".into(), 1_000_000_000, None)
        .await
        .unwrap();
    (account.id, asset.id)
}

// SYN-013 — enabling as the first device on an empty folder writes the header, one segment,
// and the manifest.
#[tokio::test]
async fn enable_as_first_device_writes_header_segment_and_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = build_ctx(dir.path()).await;
    seed_small_portfolio(&ctx).await;

    let status = ctx
        .orchestrator
        .enable_sync(
            dir.path().to_string_lossy().to_string(),
            "correct horse battery staple".into(),
            "Desktop".into(),
        )
        .await
        .expect("enabling on an empty folder must succeed as the first device");
    assert!(status.enabled);

    let header_path = dir.path().join("vaultcompass-sync.json");
    assert!(header_path.exists(), "SYN-050: the header must be written");

    let devices_dir = dir.path().join("devices");
    let device_dirs: Vec<_> = std::fs::read_dir(&devices_dir)
        .expect("a devices/ directory must exist")
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(
        device_dirs.len(),
        1,
        "exactly this device's area must exist"
    );

    let segments_dir = device_dirs[0].path().join("segments");
    let segment_files: Vec<_> = std::fs::read_dir(&segments_dir)
        .expect("a segments/ directory must exist")
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(segment_files.len(), 1, "SYN-013: exactly one first segment");

    let manifest_path = device_dirs[0].path().join("manifest.bin");
    assert!(
        manifest_path.exists(),
        "SYN-037: the manifest must be written"
    );
}

// SYN-061 — after enabling, sync_now following a new transaction publishes a second segment.
#[tokio::test]
async fn sync_now_after_a_new_transaction_publishes_a_second_segment() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = build_ctx(dir.path()).await;
    let (account_id, asset_id) = seed_small_portfolio(&ctx).await;

    ctx.orchestrator
        .enable_sync(
            dir.path().to_string_lossy().to_string(),
            "correct horse battery staple".into(),
            "Desktop".into(),
        )
        .await
        .expect("first publish must succeed");

    ctx.account_service
        .buy_holding(
            &account_id,
            asset_id,
            "2026-02-01".into(),
            10_000_000,
            50_000_000,
            1_000_000,
            0,
            None,
            None,
        )
        .await
        .unwrap();

    let report = ctx
        .orchestrator
        .sync_now()
        .await
        .expect("sync_now after enabling must succeed");
    assert!(
        report.published_changes > 0,
        "the new transaction must publish"
    );

    let devices_dir = dir.path().join("devices");
    let device_dir = std::fs::read_dir(&devices_dir)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let segment_count = std::fs::read_dir(device_dir.join("segments"))
        .unwrap()
        .count();
    assert_eq!(
        segment_count, 2,
        "the new transaction must publish a second segment"
    );

    let unpublished: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM changes WHERE published = 0")
        .fetch_one(&ctx.pool)
        .await
        .unwrap();
    assert_eq!(
        unpublished, 0,
        "SYN-031/067: every captured change is marked published once its segment is written"
    );
}
