//! Tier-3 integration: two installations ("Desktop", "Laptop") sharing one encrypted
//! folder converge on the same portfolio (SYN-013/014/036/065/080/083, CFR-040/041/042/044).
//! Per `test_convention.md` Tier 3: only the crate's public API is used — two `SqlitePool`s,
//! one `tempfile::tempdir()` folder, real BC services on each side (`Ctx { orchestrator, … }`,
//! mirroring `management_fee_crud.rs`'s and `sync_first_publish.rs`'s shape).
//!
//! Every scenario syncs both ways twice: the second round picks up what the other device
//! published in the first.

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
    SqliteSyncStateRepository, SyncRun, SyncService, SyncStateRepository,
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

/// One device's full service graph, wired exactly as the production container wires it
/// (SYN-020: every synced repository records through the real change recorder).
struct Ctx {
    orchestrator: PortfolioSyncOrchestrator,
    account_service: Arc<AccountService>,
    asset_service: Arc<AssetService>,
    pool: sqlx::Pool<sqlx::Sqlite>,
}

async fn build_ctx(folder: &std::path::Path) -> Ctx {
    let pool = make_pool().await;
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
        Box::new(
            SqliteCurrencyRateRepository::new(pool.clone()).with_change_recorder(recorder.clone()),
        ),
    ));
    let state_repo: Arc<dyn SyncStateRepository> =
        Arc::new(SqliteSyncStateRepository::new(pool.clone()));
    let folder_store = Arc::new(FsFolderStore::new(folder));
    let change_log = Arc::new(SqliteChangeLogRepository::new(pool.clone()));
    let sync_run = Arc::new(SyncRun::new(
        change_log.clone(),
        state_repo.clone(),
        folder_store.clone(),
        recorder,
    ));
    let sync_service = Arc::new(
        SyncService::new(state_repo.clone(), folder_store.clone()).with_run(sync_run.clone()),
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

const PASSPHRASE: &str = "correct horse battery staple";

// SYN-013/014/036 — a fresh installation joining an existing portfolio rebuilds by
// replaying every published change and ends up byte-identical (same accounts, assets,
// transactions) to the originating device.
#[tokio::test]
async fn join_produces_a_byte_identical_portfolio() {
    let dir = tempfile::tempdir().unwrap();
    let desktop = build_ctx(dir.path()).await;
    seed_small_portfolio(&desktop).await;
    desktop
        .orchestrator
        .enable_sync(
            dir.path().to_string_lossy().to_string(),
            PASSPHRASE.into(),
            "Desktop".into(),
        )
        .await
        .expect("Desktop, holding the portfolio, must enable as the first device");

    let laptop = build_ctx(dir.path()).await;
    laptop
        .orchestrator
        .enable_sync(
            dir.path().to_string_lossy().to_string(),
            PASSPHRASE.into(),
            "Laptop".into(),
        )
        .await
        .expect("SYN-014/036: a fresh Laptop must join and rebuild the shared portfolio");

    let desktop_accounts = desktop.account_service.get_all().await.unwrap();
    let laptop_accounts = laptop.account_service.get_all().await.unwrap();
    assert_eq!(
        desktop_accounts.len(),
        laptop_accounts.len(),
        "SYN-036: the joined portfolio must carry every account Desktop published"
    );
    assert_eq!(
        desktop_accounts.first().map(|account| &account.name),
        laptop_accounts.first().map(|account| &account.name),
    );

    // SYN-014: after joining, a sync_device row, a cursor on Desktop, and the joiner's own
    // manifest must all exist.
    let laptop_device_row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_device")
        .fetch_one(&laptop.pool)
        .await
        .unwrap();
    assert_eq!(
        laptop_device_row_count, 1,
        "SYN-014: the joiner must have its own sync_device row after joining"
    );
}

// CFR-040/041 — sequential edits recorded while apart accumulate on both devices, in the
// same replay order, with no notices for changes that never collided.
#[tokio::test]
async fn sequential_edits_sync_to_identical_portfolios_with_no_notices() {
    let dir = tempfile::tempdir().unwrap();
    let desktop = build_ctx(dir.path()).await;
    let (account_id, asset_id) = seed_small_portfolio(&desktop).await;
    // The position both devices will touch exists before Laptop joins (HNO: a note needs a
    // held asset).
    desktop
        .account_service
        .buy_holding(
            &account_id,
            asset_id.clone(),
            "2026-01-15".into(),
            10_000_000,
            50_000_000,
            1_000_000,
            0,
            None,
            None,
        )
        .await
        .unwrap();
    desktop
        .orchestrator
        .enable_sync(
            dir.path().to_string_lossy().to_string(),
            PASSPHRASE.into(),
            "Desktop".into(),
        )
        .await
        .expect("Desktop must enable as the first device");

    let laptop = build_ctx(dir.path()).await;
    laptop
        .orchestrator
        .enable_sync(
            dir.path().to_string_lossy().to_string(),
            PASSPHRASE.into(),
            "Laptop".into(),
        )
        .await
        .expect("Laptop must join the shared portfolio");

    // Desktop deposits while apart.
    desktop
        .account_service
        .record_deposit(&account_id, "2026-02-01".into(), 500_000_000, None)
        .await
        .unwrap();
    // Laptop records a holding note while apart.
    laptop
        .account_service
        .upsert_holding_note(
            &account_id,
            asset_id.clone(),
            "watching this position".into(),
            None,
            None,
        )
        .await
        .unwrap();

    desktop.orchestrator.sync_now().await.unwrap();
    laptop.orchestrator.sync_now().await.unwrap();
    // A second round both ways picks up what the other device published in the first round.
    let desktop_report = desktop.orchestrator.sync_now().await.unwrap();
    let laptop_report = laptop.orchestrator.sync_now().await.unwrap();

    let desktop_txs = desktop
        .account_service
        .get_all_transactions_for_account(&account_id)
        .await
        .unwrap();
    let laptop_txs = laptop
        .account_service
        .get_all_transactions_for_account(&account_id)
        .await
        .unwrap();
    assert_eq!(
        desktop_txs.len(),
        laptop_txs.len(),
        "CFR-040: every transaction created on either device must end up on both"
    );
    assert_eq!(desktop_txs.len(), 3, "deposit, buy, deposit");
    let desktop_note = desktop
        .account_service
        .get_holding_notes(&account_id)
        .await
        .unwrap();
    assert_eq!(
        desktop_note.first().map(|note| note.text.as_str()),
        Some("watching this position"),
        "Laptop's note must reach Desktop"
    );
    assert_eq!(
        desktop_report.notices_raised, 0,
        "CFR-060: sequential, non-colliding changes must never raise a notice"
    );
    assert_eq!(laptop_report.notices_raised, 0);
}

// CFR-020/060 — a concurrent rename of the same account on both devices: the later rank
// wins everywhere, and exactly one notice is raised, on the losing device.
#[tokio::test]
async fn concurrent_rename_of_the_same_account_produces_one_notice_on_the_losing_device() {
    let dir = tempfile::tempdir().unwrap();
    let desktop = build_ctx(dir.path()).await;
    let (account_id, _asset_id) = seed_small_portfolio(&desktop).await;
    desktop
        .orchestrator
        .enable_sync(
            dir.path().to_string_lossy().to_string(),
            PASSPHRASE.into(),
            "Desktop".into(),
        )
        .await
        .expect("Desktop must enable as the first device");
    let laptop = build_ctx(dir.path()).await;
    laptop
        .orchestrator
        .enable_sync(
            dir.path().to_string_lossy().to_string(),
            PASSPHRASE.into(),
            "Laptop".into(),
        )
        .await
        .expect("Laptop must join the shared portfolio");

    // Both rename the same account concurrently, without syncing between the two edits.
    desktop
        .account_service
        .update(
            account_id.clone(),
            "Renamed on Desktop".into(),
            String::new(),
            "USD".into(),
            UpdateFrequency::ManualMonth,
            false,
        )
        .await
        .unwrap();
    laptop
        .account_service
        .update(
            account_id.clone(),
            "Renamed on Laptop".into(),
            String::new(),
            "USD".into(),
            UpdateFrequency::ManualMonth,
            false,
        )
        .await
        .unwrap();

    desktop.orchestrator.sync_now().await.unwrap();
    laptop.orchestrator.sync_now().await.unwrap();
    desktop.orchestrator.sync_now().await.unwrap();
    laptop.orchestrator.sync_now().await.unwrap();

    let desktop_account = desktop
        .account_service
        .get_by_id(&account_id)
        .await
        .unwrap()
        .expect("account must still exist");
    let laptop_account = laptop
        .account_service
        .get_by_id(&account_id)
        .await
        .unwrap()
        .expect("account must still exist");
    assert_eq!(
        desktop_account.name, laptop_account.name,
        "CFR-020: the higher-ranked rename must prevail identically on both devices"
    );

    let desktop_notices = desktop
        .orchestrator
        .get_sync_status()
        .await
        .unwrap()
        .notices;
    let laptop_notices = laptop.orchestrator.get_sync_status().await.unwrap().notices;
    let total_notices = desktop_notices.len() + laptop_notices.len();
    assert_eq!(
        total_notices, 1,
        "CFR-060: exactly one notice, on the device whose rename lost, {desktop_notices:?} \
         {laptop_notices:?}"
    );
}

// CFR-032 — Desktop deletes an account while Laptop, unsynced, records a transaction on
// it: the transaction is dropped on both, and only Laptop (whose change lost) is told.
#[tokio::test]
async fn deleting_an_account_drops_a_concurrent_transaction_and_notifies_only_the_losing_device() {
    let dir = tempfile::tempdir().unwrap();
    let desktop = build_ctx(dir.path()).await;
    let (account_id, asset_id) = seed_small_portfolio(&desktop).await;
    desktop
        .orchestrator
        .enable_sync(
            dir.path().to_string_lossy().to_string(),
            PASSPHRASE.into(),
            "Desktop".into(),
        )
        .await
        .expect("Desktop must enable as the first device");
    let laptop = build_ctx(dir.path()).await;
    laptop
        .orchestrator
        .enable_sync(
            dir.path().to_string_lossy().to_string(),
            PASSPHRASE.into(),
            "Laptop".into(),
        )
        .await
        .expect("Laptop must join the shared portfolio");

    desktop.account_service.delete(&account_id).await.unwrap();
    laptop
        .account_service
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

    desktop.orchestrator.sync_now().await.unwrap();
    laptop.orchestrator.sync_now().await.unwrap();
    desktop.orchestrator.sync_now().await.unwrap();
    laptop.orchestrator.sync_now().await.unwrap();

    assert!(
        desktop
            .account_service
            .get_by_id(&account_id)
            .await
            .unwrap()
            .is_none(),
        "CFR-022: the account stays deleted on the device that deleted it"
    );
    assert!(
        laptop
            .account_service
            .get_by_id(&account_id)
            .await
            .unwrap()
            .is_none(),
        "CFR-032: the account must be removed on Laptop too, taking its concurrent buy with it"
    );
    let laptop_notices = laptop.orchestrator.get_sync_status().await.unwrap().notices;
    assert!(
        !laptop_notices.is_empty(),
        "CFR-032/060: Laptop, whose transaction was dropped, must be told"
    );
}

// CFR-042 — two independent sales that individually exceed the shared position: both
// survive after merge, and the holding is inconsistent on both devices.
#[tokio::test]
async fn independent_oversell_on_both_devices_keeps_both_sales() {
    let dir = tempfile::tempdir().unwrap();
    let desktop = build_ctx(dir.path()).await;
    let (account_id, asset_id) = seed_small_portfolio(&desktop).await;
    desktop
        .account_service
        .buy_holding(
            &account_id,
            asset_id.clone(),
            "2026-01-15".into(),
            15_000_000,
            50_000_000,
            1_000_000,
            0,
            None,
            None,
        )
        .await
        .unwrap();
    desktop
        .orchestrator
        .enable_sync(
            dir.path().to_string_lossy().to_string(),
            PASSPHRASE.into(),
            "Desktop".into(),
        )
        .await
        .expect("Desktop must enable as the first device");
    let laptop = build_ctx(dir.path()).await;
    laptop
        .orchestrator
        .enable_sync(
            dir.path().to_string_lossy().to_string(),
            PASSPHRASE.into(),
            "Laptop".into(),
        )
        .await
        .expect("Laptop must join the shared portfolio");

    // Each sells 10 of the 15 held — individually valid, together an oversell of -5.
    desktop
        .account_service
        .sell_holding(
            &account_id,
            asset_id.clone(),
            "2026-03-01".into(),
            10_000_000,
            55_000_000,
            1_000_000,
            0,
            None,
            None,
        )
        .await
        .unwrap();
    laptop
        .account_service
        .sell_holding(
            &account_id,
            asset_id,
            "2026-03-02".into(),
            10_000_000,
            56_000_000,
            1_000_000,
            0,
            None,
            None,
        )
        .await
        .unwrap();

    desktop.orchestrator.sync_now().await.unwrap();
    laptop.orchestrator.sync_now().await.unwrap();
    desktop.orchestrator.sync_now().await.unwrap();
    laptop.orchestrator.sync_now().await.unwrap();

    let desktop_txs = desktop
        .account_service
        .get_all_transactions_for_account(&account_id)
        .await
        .unwrap();
    let laptop_txs = laptop
        .account_service
        .get_all_transactions_for_account(&account_id)
        .await
        .unwrap();
    assert_eq!(
        desktop_txs.len(),
        laptop_txs.len(),
        "CFR-042: merge never drops a transaction to restore the invariant"
    );

    let desktop_status = desktop.orchestrator.get_sync_status().await.unwrap();
    let laptop_status = laptop.orchestrator.get_sync_status().await.unwrap();
    assert!(
        !desktop_status.inconsistent_holdings.is_empty(),
        "CFR-042/SYN-040: the oversold holding must be marked inconsistent on Desktop"
    );
    assert!(
        !laptop_status.inconsistent_holdings.is_empty(),
        "CFR-042/SYN-040: the oversold holding must be marked inconsistent on Laptop too"
    );
}

// CFR-044 — a fee schedule's catch-up position converges by maximum after sync, whatever
// order the two devices' segments arrive in.
#[tokio::test]
async fn fee_catch_up_positions_converge_by_maximum() {
    let dir = tempfile::tempdir().unwrap();
    let desktop = build_ctx(dir.path()).await;
    let (account_id, asset_id) = seed_small_portfolio(&desktop).await;
    desktop
        .account_service
        .update(
            account_id.clone(),
            "Portfolio".into(),
            String::new(),
            "USD".into(),
            UpdateFrequency::ManualMonth,
            true,
        )
        .await
        .unwrap();
    desktop
        .account_service
        .create_fee_schedule(
            &account_id,
            asset_id.clone(),
            1_000_000,
            vault_compass_lib::context::account::FeeFrequency::Monthly,
            "2026-01-01".into(),
            None,
        )
        .await
        .unwrap();
    desktop
        .orchestrator
        .enable_sync(
            dir.path().to_string_lossy().to_string(),
            PASSPHRASE.into(),
            "Desktop".into(),
        )
        .await
        .expect("Desktop must enable as the first device");
    let laptop = build_ctx(dir.path()).await;
    laptop
        .orchestrator
        .enable_sync(
            dir.path().to_string_lossy().to_string(),
            PASSPHRASE.into(),
            "Laptop".into(),
        )
        .await
        .expect("Laptop must join the shared portfolio");

    // Desktop advances its catch-up position to August; Laptop, unsynced, still holds July.
    desktop
        .account_service
        .advance_fee_schedule_cursor(&account_id, &asset_id, "2026-08-31".into())
        .await
        .unwrap();
    laptop
        .account_service
        .advance_fee_schedule_cursor(&account_id, &asset_id, "2026-07-31".into())
        .await
        .unwrap();

    desktop.orchestrator.sync_now().await.unwrap();
    laptop.orchestrator.sync_now().await.unwrap();
    desktop.orchestrator.sync_now().await.unwrap();
    laptop.orchestrator.sync_now().await.unwrap();

    let desktop_position = desktop
        .account_service
        .list_fee_catch_up_positions_for_account(&account_id)
        .await
        .unwrap();
    let laptop_position = laptop
        .account_service
        .list_fee_catch_up_positions_for_account(&account_id)
        .await
        .unwrap();
    assert_eq!(
        desktop_position
            .first()
            .map(|p| p.last_applied_period.clone()),
        Some("2026-08-31".to_string()),
        "CFR-044: the maximum of the two positions must stand on Desktop"
    );
    assert_eq!(
        desktop_position
            .first()
            .map(|p| p.last_applied_period.clone()),
        laptop_position
            .first()
            .map(|p| p.last_applied_period.clone()),
        "CFR-044: both devices must converge on the same maximum"
    );
}
