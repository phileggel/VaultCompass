/// Integration tests for the asset-price fetch orchestrator (MKT-122, MKT-132, MKT-111, MKT-113).
///
/// These tests exercise the full stack through the public API: orchestrator constructor →
/// AccountService / AssetService → real in-memory SQLite. No mocks — per test_convention.md
/// Tier 3 constraint.
use std::sync::Arc;
use vault_compass_lib::context::account::{
    AccountService, SqliteAccountRepository, SqliteHoldingRepository, SqliteTransactionRepository,
};
use vault_compass_lib::context::asset::{
    AssetService, SqliteAssetCategoryRepository, SqliteAssetPriceRepository, SqliteAssetRepository,
};
use vault_compass_lib::context::connection::{
    ConnectionProbe, ConnectionService, KeyStore, Provider, ProviderKeyTestOutcome, StorageTier,
};
use vault_compass_lib::context::currency::{
    CurrencyService, SqliteCurrencyPairRepository, SqliteCurrencyRateRepository,
};
use vault_compass_lib::core::SideEffectEventBus;
use vault_compass_lib::use_cases::asset_price_fetch::{AssetPriceFetchUseCase, FetchGuard};

/// A key store that always resolves a fixed Stooq key, so the pre-key fetch tests
/// proceed past the KEY-044 no-key short-circuit. Real key storage is covered by
/// the connection BC's own unit tests; here the resolved value only needs to be
/// `Some`, never the truth.
struct StubKeyStore;
#[async_trait::async_trait]
impl KeyStore for StubKeyStore {
    async fn clear(&self, _provider: Provider) -> anyhow::Result<()> {
        Ok(())
    }
    async fn store(
        &self,
        _provider: Provider,
        _key: &str,
        _allow_plaintext: bool,
    ) -> anyhow::Result<StorageTier> {
        Ok(StorageTier::SessionMemory)
    }
    async fn locate(&self, _provider: Provider) -> anyhow::Result<Option<StorageTier>> {
        Ok(Some(StorageTier::SessionMemory))
    }
    async fn read(&self, _provider: Provider) -> anyhow::Result<Option<String>> {
        Ok(Some("test-stooq-key".to_string()))
    }
}
struct StubProbe;
#[async_trait::async_trait]
impl ConnectionProbe for StubProbe {
    async fn probe(
        &self,
        _provider: Provider,
        _key: &str,
    ) -> anyhow::Result<ProviderKeyTestOutcome> {
        Ok(ProviderKeyTestOutcome::Accepted)
    }
}
/// A `ConnectionService` backed by [`StubKeyStore`] for fetch-orchestrator tests.
fn stub_connection_service() -> Arc<ConnectionService> {
    Arc::new(ConnectionService::new(
        Box::new(StubKeyStore),
        Box::new(StubProbe),
    ))
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
    use_case: AssetPriceFetchUseCase,
    fetch_guard: Arc<FetchGuard>,
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
    let fetch_guard = Arc::new(FetchGuard::new());

    let use_case = {
        use vault_compass_lib::use_cases::asset_price_fetch::dispatcher::Dispatcher;

        struct NoOpProvider;
        #[async_trait::async_trait]
        impl vault_compass_lib::context::asset::PriceProvider for NoOpProvider {
            async fn fetch_price(
                &self,
                _symbol: &str,
                _api_key: Option<String>,
            ) -> anyhow::Result<Option<vault_compass_lib::context::asset::Quote>> {
                Ok(Some(vault_compass_lib::context::asset::Quote {
                    price: 100_000_000,
                    date: None,
                }))
            }
        }

        let price_repo: Arc<dyn vault_compass_lib::context::asset::AssetPriceRepository> =
            Arc::new(SqliteAssetPriceRepository::new(pool.clone()));

        let dispatcher = Arc::new(Dispatcher::new(
            Arc::new(NoOpProvider),
            price_repo,
            Arc::clone(&bus),
            Arc::new(CurrencyService::new(
                Box::new(SqliteCurrencyPairRepository::new(pool.clone())),
                Box::new(SqliteCurrencyRateRepository::new(pool.clone())),
            )),
            Arc::new(|| chrono::Local::now().date_naive()),
        ));

        AssetPriceFetchUseCase::new(
            Arc::clone(&account_service),
            Arc::clone(&asset_service),
            Arc::clone(&fetch_guard),
            dispatcher,
            stub_connection_service(),
        )
    };

    Ctx {
        use_case,
        fetch_guard,
    }
}

/// MKT-111 — fetch_all returns NoFetchableHoldings when no non-cash derivable holdings exist
/// (empty database). Exercises the full stack end-to-end.
#[tokio::test]
async fn fetch_all_returns_no_fetchable_holdings_on_empty_db() {
    use vault_compass_lib::use_cases::asset_price_fetch::{
        FetchAllAssetPricesError, FetchPriceTask,
    };

    let ctx = build_ctx().await;
    let result = ctx.use_case.fetch_all(true).await;

    assert!(
        matches!(
            result,
            Err(FetchAllAssetPricesError::Failure(
                FetchPriceTask::NoFetchableHoldings
            ))
        ),
        "expected NoFetchableHoldings on empty DB, got: {result:?}"
    );
}

/// MKT-132 — fetch_for_account returns AccountNotFound for an unknown account_id.
/// Exercises the full existence-check stack.
#[tokio::test]
async fn fetch_for_account_returns_account_not_found_for_unknown_id() {
    use vault_compass_lib::context::account::AccountApplicationError;
    use vault_compass_lib::use_cases::asset_price_fetch::FetchAccountAssetPricesError;

    let ctx = build_ctx().await;
    let result = ctx.use_case.fetch_for_account("does-not-exist", true).await;

    assert!(
        matches!(
            result,
            Err(FetchAccountAssetPricesError::Account(
                AccountApplicationError::AccountNotFound { .. }
            ))
        ),
        "expected Account(AccountNotFound), got: {result:?}"
    );
}

/// MKT-113 — fetch_all returns FetchAlreadyRunning when the guard is held externally.
/// Verifies the in-flight guard propagates through the public use-case API.
#[tokio::test]
async fn fetch_all_returns_fetch_already_running_while_guard_held() {
    use vault_compass_lib::use_cases::asset_price_fetch::{
        FetchAllAssetPricesError, FetchPriceTask,
    };

    let ctx = build_ctx().await;
    let _lease = ctx
        .fetch_guard
        .try_acquire()
        .expect("guard must be free at test start");

    let result = ctx.use_case.fetch_all(true).await;
    assert!(
        matches!(
            result,
            Err(FetchAllAssetPricesError::Failure(
                FetchPriceTask::FetchAlreadyRunning
            ))
        ),
        "expected FetchAlreadyRunning, got: {result:?}"
    );
}

/// MKT-110 — fetch_for_account uses derive_stooq_symbol_with_exchange so an asset
/// carrying `exchange = Some(XPAR)` resolves to `<ref>.fr` (exchange-qualified)
/// rather than the bare-ticker legacy form. Guards the wiring of the picker-driven
/// exchange field into the actual Stooq fetch URL.
#[tokio::test]
async fn fetch_for_account_passes_exchange_qualified_symbol_to_provider() {
    use std::sync::Mutex;
    use vault_compass_lib::context::account::{
        AccountService, SqliteAccountRepository, SqliteHoldingRepository,
        SqliteTransactionRepository, UpdateFrequency,
    };
    use vault_compass_lib::context::asset::{
        AssetPriceRepository, AssetService, CreateAssetDTO, PriceProvider,
        SqliteAssetCategoryRepository, SqliteAssetPriceRepository, SqliteAssetRepository,
        SYSTEM_CATEGORY_ID,
    };
    use vault_compass_lib::core::SideEffectEventBus;
    use vault_compass_lib::use_cases::asset_price_fetch::dispatcher::Dispatcher;

    struct CapturingProvider {
        seen: Arc<Mutex<Vec<String>>>,
    }
    #[async_trait::async_trait]
    impl PriceProvider for CapturingProvider {
        async fn fetch_price(
            &self,
            symbol: &str,
            _api_key: Option<String>,
        ) -> anyhow::Result<Option<vault_compass_lib::context::asset::Quote>> {
            self.seen.lock().unwrap().push(symbol.to_string());
            Ok(Some(vault_compass_lib::context::asset::Quote {
                price: 100_000_000,
                date: None,
            }))
        }
    }

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
            name: "Air Liquide".to_string(),
            reference: "AI".to_string(),
            isin: None,
            class: vault_compass_lib::context::asset::AssetClass::Stocks,
            currency: "EUR".to_string(),
            risk_level: 4,
            category_id: SYSTEM_CATEGORY_ID.to_string(),
            exchange: Some(vault_compass_lib::context::asset::Exchange {
                code: "XPAR".to_string(),
                label: "Euronext Paris".to_string(),
            }),
        })
        .await
        .expect("seed asset with XPAR exchange");

    let account = account_service
        .create(
            "Test".to_string(),
            "EUR".to_string(),
            UpdateFrequency::ManualMonth,
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

    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let provider = Arc::new(CapturingProvider {
        seen: Arc::clone(&seen),
    });
    let price_repo: Arc<dyn AssetPriceRepository> =
        Arc::new(SqliteAssetPriceRepository::new(pool.clone()));
    let dispatcher = Arc::new(Dispatcher::new(
        provider,
        price_repo,
        Arc::clone(&bus),
        Arc::new(CurrencyService::new(
            Box::new(SqliteCurrencyPairRepository::new(pool.clone())),
            Box::new(SqliteCurrencyRateRepository::new(pool.clone())),
        )),
        Arc::new(|| chrono::Local::now().date_naive()),
    ));
    let use_case = AssetPriceFetchUseCase::new(
        Arc::clone(&account_service),
        Arc::clone(&asset_service),
        Arc::new(FetchGuard::new()),
        dispatcher,
        stub_connection_service(),
    );

    use_case
        .fetch_for_account(&account.id, true)
        .await
        .expect("fetch_for_account dispatch");

    // Dispatcher::spawn launches an async task — give it a moment to call the provider.
    for _ in 0..50 {
        if !seen.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    let symbols = seen.lock().unwrap().clone();
    assert_eq!(
        symbols,
        vec!["ai.fr".to_string()],
        "MKT-110: orchestrator must derive `ai.fr` (XPAR → .fr suffix) for an asset carrying `exchange = Some(XPAR)`, not the bare `ai` legacy form"
    );
}

/// MKT-151 / ADR-014 — a locked asset (`price_refresh_blocked`) is excluded from
/// fetch scope. With the account's only holding locked, `build_scope` yields an
/// empty set and `fetch_for_account` is rejected with `NoFetchableHoldings`
/// (MKT-111), proving the asset was skipped before any provider call.
#[tokio::test]
async fn fetch_for_account_skips_locked_asset() {
    use vault_compass_lib::context::account::UpdateFrequency;
    use vault_compass_lib::context::asset::{
        AssetClass, AssetPriceRepository, CreateAssetDTO, PriceProvider, SYSTEM_CATEGORY_ID,
    };
    use vault_compass_lib::use_cases::asset_price_fetch::dispatcher::Dispatcher;
    use vault_compass_lib::use_cases::asset_price_fetch::{
        FetchAccountAssetPricesError, FetchPriceTask,
    };

    struct NoOpProvider;
    #[async_trait::async_trait]
    impl PriceProvider for NoOpProvider {
        async fn fetch_price(
            &self,
            _symbol: &str,
            _api_key: Option<String>,
        ) -> anyhow::Result<Option<vault_compass_lib::context::asset::Quote>> {
            Ok(Some(vault_compass_lib::context::asset::Quote {
                price: 100_000_000,
                date: None,
            }))
        }
    }

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
            currency: "USD".to_string(),
            risk_level: 4,
            category_id: SYSTEM_CATEGORY_ID.to_string(),
            exchange: None,
        })
        .await
        .expect("seed asset");

    // MKT-156 — lock the asset before the fetch.
    asset_service
        .block_price_refresh(&asset.id)
        .await
        .expect("lock asset");

    let account = account_service
        .create(
            "Test".to_string(),
            "USD".to_string(),
            UpdateFrequency::ManualMonth,
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

    let price_repo: Arc<dyn AssetPriceRepository> =
        Arc::new(SqliteAssetPriceRepository::new(pool.clone()));
    let dispatcher = Arc::new(Dispatcher::new(
        Arc::new(NoOpProvider),
        price_repo,
        Arc::clone(&bus),
        Arc::new(CurrencyService::new(
            Box::new(SqliteCurrencyPairRepository::new(pool.clone())),
            Box::new(SqliteCurrencyRateRepository::new(pool.clone())),
        )),
        Arc::new(|| chrono::Local::now().date_naive()),
    ));
    let use_case = AssetPriceFetchUseCase::new(
        Arc::clone(&account_service),
        Arc::clone(&asset_service),
        Arc::new(FetchGuard::new()),
        dispatcher,
        stub_connection_service(),
    );

    let result = use_case.fetch_for_account(&account.id, true).await;
    assert!(
        matches!(
            result,
            Err(FetchAccountAssetPricesError::Failure(
                FetchPriceTask::NoFetchableHoldings
            ))
        ),
        "MKT-151: a locked asset must be excluded from fetch scope → NoFetchableHoldings, got: {result:?}"
    );
}

/// MKT-156 — unblocking re-admits an asset to fetch scope. After locking then
/// unblocking the account's only holding, `fetch_for_account` dispatches
/// successfully (scope is non-empty again), exercising the unblock repo path.
#[tokio::test]
async fn fetch_for_account_includes_unblocked_asset() {
    use vault_compass_lib::context::account::UpdateFrequency;
    use vault_compass_lib::context::asset::{
        AssetClass, AssetPriceRepository, CreateAssetDTO, PriceProvider, SYSTEM_CATEGORY_ID,
    };
    use vault_compass_lib::use_cases::asset_price_fetch::dispatcher::Dispatcher;

    struct NoOpProvider;
    #[async_trait::async_trait]
    impl PriceProvider for NoOpProvider {
        async fn fetch_price(
            &self,
            _symbol: &str,
            _api_key: Option<String>,
        ) -> anyhow::Result<Option<vault_compass_lib::context::asset::Quote>> {
            Ok(Some(vault_compass_lib::context::asset::Quote {
                price: 100_000_000,
                date: None,
            }))
        }
    }

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
            currency: "USD".to_string(),
            risk_level: 4,
            category_id: SYSTEM_CATEGORY_ID.to_string(),
            exchange: None,
        })
        .await
        .expect("seed asset");

    // Lock then unblock (MKT-156) — the asset must be fetchable again.
    asset_service
        .block_price_refresh(&asset.id)
        .await
        .expect("lock asset");
    asset_service
        .unblock_price_refresh(&asset.id)
        .await
        .expect("unlock asset");

    let account = account_service
        .create(
            "Test".to_string(),
            "USD".to_string(),
            UpdateFrequency::ManualMonth,
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

    let price_repo: Arc<dyn AssetPriceRepository> =
        Arc::new(SqliteAssetPriceRepository::new(pool.clone()));
    let dispatcher = Arc::new(Dispatcher::new(
        Arc::new(NoOpProvider),
        price_repo,
        Arc::clone(&bus),
        Arc::new(CurrencyService::new(
            Box::new(SqliteCurrencyPairRepository::new(pool.clone())),
            Box::new(SqliteCurrencyRateRepository::new(pool.clone())),
        )),
        Arc::new(|| chrono::Local::now().date_naive()),
    ));
    let use_case = AssetPriceFetchUseCase::new(
        Arc::clone(&account_service),
        Arc::clone(&asset_service),
        Arc::new(FetchGuard::new()),
        dispatcher,
        stub_connection_service(),
    );

    // Scope is non-empty (asset unblocked) → dispatch succeeds, not NoFetchableHoldings.
    use_case
        .fetch_for_account(&account.id, true)
        .await
        .expect("MKT-156: an unblocked asset must re-enter fetch scope and dispatch");
}

/// MKT-119 — a fetch task publishes `AssetPriceFetchCompleted` carrying outcome
/// counts. With one fetchable holding and a provider that returns a usable quote,
/// the terminal event reports `ok = 1, skipped = 0`.
#[tokio::test]
async fn fetch_publishes_completion_event_with_counts() {
    use vault_compass_lib::context::account::UpdateFrequency;
    use vault_compass_lib::context::asset::{
        AssetClass, CreateAssetDTO, PriceProvider, SYSTEM_CATEGORY_ID,
    };
    use vault_compass_lib::core::event_bus::Event;
    use vault_compass_lib::use_cases::asset_price_fetch::dispatcher::Dispatcher;

    struct OkProvider;
    #[async_trait::async_trait]
    impl PriceProvider for OkProvider {
        async fn fetch_price(
            &self,
            _symbol: &str,
            _api_key: Option<String>,
        ) -> anyhow::Result<Option<vault_compass_lib::context::asset::Quote>> {
            Ok(Some(vault_compass_lib::context::asset::Quote {
                price: 100_000_000,
                date: Some("2026-01-02".to_string()),
            }))
        }
    }

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
            currency: "USD".to_string(),
            risk_level: 4,
            category_id: SYSTEM_CATEGORY_ID.to_string(),
            exchange: None,
        })
        .await
        .expect("seed asset");
    let account = account_service
        .create(
            "Test".to_string(),
            "USD".to_string(),
            UpdateFrequency::ManualMonth,
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
        Arc::clone(&account_service),
        Arc::clone(&asset_service),
        Arc::new(FetchGuard::new()),
        dispatcher,
        stub_connection_service(),
    );

    let mut rx = bus.subscribe();
    use_case
        .fetch_for_account(&account.id, true)
        .await
        .expect("dispatch");

    // The fetch runs in a detached task; wait for the terminal completion event.
    let counts = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            rx.changed()
                .await
                .expect("bus closed before AssetPriceFetchCompleted arrived");
            if let Event::AssetPriceFetchCompleted { ok, skipped } = *rx.borrow() {
                return (ok, skipped);
            }
        }
    })
    .await
    .expect("AssetPriceFetchCompleted within timeout");

    assert_eq!(counts, (1, 0), "one holding fetched ok, none skipped");
}

// =============================================================================
// KEY-043 / KEY-044 — Stooq keyed fetch + no-key short-circuit.
//
// These tests target the provider-seam rewire described in the
// api-key-management plan (§Detailed Implementation Plan / Backend):
//   - `PriceProvider::fetch_price` gains an `api_key: &str` parameter (KEY-043).
//   - `Dispatcher::spawn` gains a `stooq_key: Option<String>` parameter; `None`
//     short-circuits the whole scope and publishes `AssetPriceFetchCompleted
//     { ok: 0, skipped: <scope_len> }` (KEY-044).
//
// They reference the not-yet-existing signatures, so they fail to compile against
// the current production code — a valid red baseline.
// =============================================================================

/// KEY-044 — when no Stooq key is resolved, a dispatched fetch makes ZERO
/// `provider.fetch_price` calls and publishes `AssetPriceFetchCompleted
/// { ok: 0, skipped: <scope_len> }`. The dispatcher detects the absent key once
/// at task start and skips the entire scope without any per-asset provider call.
#[tokio::test]
async fn dispatcher_with_no_key_skips_whole_scope_and_reports_all_skipped() {
    use std::sync::Mutex;
    use vault_compass_lib::context::asset::{
        Asset, AssetClass, AssetPriceRepository, PriceProvider, Quote, SYSTEM_CATEGORY_ID,
    };
    use vault_compass_lib::context::currency::CurrencyPair;
    use vault_compass_lib::core::event_bus::Event;
    use vault_compass_lib::use_cases::asset_price_fetch::dispatcher::Dispatcher;
    use vault_compass_lib::use_cases::asset_price_fetch::FetchGuard;

    // A provider that records whether it was ever called. Under KEY-044 it must
    // not be called at all when no key is present.
    struct CountingProvider {
        calls: Arc<Mutex<u32>>,
    }
    #[async_trait::async_trait]
    impl PriceProvider for CountingProvider {
        async fn fetch_price(
            &self,
            _symbol: &str,
            _api_key: Option<String>,
        ) -> anyhow::Result<Option<Quote>> {
            *self.calls.lock().unwrap() += 1;
            Ok(Some(Quote {
                price: 100_000_000,
                date: None,
            }))
        }
    }

    let pool = make_pool().await;
    let bus = Arc::new(SideEffectEventBus::new());

    let asset_service = Arc::new(AssetService::new(
        Box::new(SqliteAssetRepository::new(pool.clone())),
        Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
        Box::new(SqliteAssetPriceRepository::new(pool.clone())),
    ));
    let asset: Asset = asset_service
        .create_asset(vault_compass_lib::context::asset::CreateAssetDTO {
            name: "Apple".to_string(),
            reference: "AAPL".to_string(),
            isin: None,
            class: AssetClass::Stocks,
            currency: "USD".to_string(),
            risk_level: 4,
            category_id: SYSTEM_CATEGORY_ID.to_string(),
            exchange: None,
        })
        .await
        .expect("seed asset");

    let calls = Arc::new(Mutex::new(0u32));
    let provider = Arc::new(CountingProvider {
        calls: Arc::clone(&calls),
    });
    let price_repo: Arc<dyn AssetPriceRepository> =
        Arc::new(SqliteAssetPriceRepository::new(pool.clone()));
    let dispatcher = Arc::new(Dispatcher::new(
        provider,
        price_repo,
        Arc::clone(&bus),
        Arc::new(CurrencyService::new(
            Box::new(SqliteCurrencyPairRepository::new(pool.clone())),
            Box::new(SqliteCurrencyRateRepository::new(pool.clone())),
        )),
        Arc::new(|| chrono::Local::now().date_naive()),
    ));

    let scope: Vec<(Asset, String)> = vec![(asset, "aapl.us".to_string())];
    let fx_pairs: Vec<CurrencyPair> = Vec::new();
    let lease = Arc::new(FetchGuard::new())
        .try_acquire()
        .expect("guard free at start");

    let mut rx = bus.subscribe();
    // KEY-044 — dispatch with `stooq_key = None`.
    Arc::clone(&dispatcher).spawn(scope, fx_pairs, lease, true, None);

    let counts = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            rx.changed()
                .await
                .expect("bus closed before AssetPriceFetchCompleted arrived");
            if let Event::AssetPriceFetchCompleted { ok, skipped } = *rx.borrow() {
                return (ok, skipped);
            }
        }
    })
    .await
    .expect("AssetPriceFetchCompleted within timeout");

    assert_eq!(
        counts,
        (0, 1),
        "KEY-044: no key must skip the whole scope (ok=0, skipped=scope_len)"
    );
    assert_eq!(
        *calls.lock().unwrap(),
        0,
        "KEY-044: no per-asset provider call must be made when no key is present"
    );
}

/// KEY-043 — when a Stooq key is present, `provider.fetch_price` is called with
/// the api-key argument threaded through from the dispatched `stooq_key`.
#[tokio::test]
async fn dispatcher_with_key_threads_api_key_into_fetch_price() {
    use std::sync::Mutex;
    use vault_compass_lib::context::asset::{
        Asset, AssetClass, AssetPriceRepository, PriceProvider, Quote, SYSTEM_CATEGORY_ID,
    };
    use vault_compass_lib::context::currency::CurrencyPair;
    use vault_compass_lib::core::event_bus::Event;
    use vault_compass_lib::use_cases::asset_price_fetch::dispatcher::Dispatcher;
    use vault_compass_lib::use_cases::asset_price_fetch::FetchGuard;

    // A provider that records the api key it was handed (the sentinel "<keyless>"
    // when called in keyless mode, KEY-053).
    struct KeyCapturingProvider {
        seen_keys: Arc<Mutex<Vec<String>>>,
    }
    #[async_trait::async_trait]
    impl PriceProvider for KeyCapturingProvider {
        async fn fetch_price(
            &self,
            _symbol: &str,
            api_key: Option<String>,
        ) -> anyhow::Result<Option<Quote>> {
            self.seen_keys
                .lock()
                .expect("KeyCapturingProvider seen_keys lock")
                .push(api_key.unwrap_or_else(|| "<keyless>".to_string()));
            Ok(Some(Quote {
                price: 100_000_000,
                date: None,
            }))
        }
    }

    let pool = make_pool().await;
    let bus = Arc::new(SideEffectEventBus::new());

    let asset_service = Arc::new(AssetService::new(
        Box::new(SqliteAssetRepository::new(pool.clone())),
        Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
        Box::new(SqliteAssetPriceRepository::new(pool.clone())),
    ));
    let asset: Asset = asset_service
        .create_asset(vault_compass_lib::context::asset::CreateAssetDTO {
            name: "Apple".to_string(),
            reference: "AAPL".to_string(),
            isin: None,
            class: AssetClass::Stocks,
            currency: "USD".to_string(),
            risk_level: 4,
            category_id: SYSTEM_CATEGORY_ID.to_string(),
            exchange: None,
        })
        .await
        .expect("seed asset");

    let seen_keys = Arc::new(Mutex::new(Vec::<String>::new()));
    let provider = Arc::new(KeyCapturingProvider {
        seen_keys: Arc::clone(&seen_keys),
    });
    let price_repo: Arc<dyn AssetPriceRepository> =
        Arc::new(SqliteAssetPriceRepository::new(pool.clone()));
    let dispatcher = Arc::new(Dispatcher::new(
        provider,
        price_repo,
        Arc::clone(&bus),
        Arc::new(CurrencyService::new(
            Box::new(SqliteCurrencyPairRepository::new(pool.clone())),
            Box::new(SqliteCurrencyRateRepository::new(pool.clone())),
        )),
        Arc::new(|| chrono::Local::now().date_naive()),
    ));

    let scope: Vec<(Asset, String)> = vec![(asset, "aapl.us".to_string())];
    let fx_pairs: Vec<CurrencyPair> = Vec::new();
    let lease = Arc::new(FetchGuard::new())
        .try_acquire()
        .expect("guard free at start");

    let mut rx = bus.subscribe();
    // KEY-043 — dispatch with a present key; it must reach `fetch_price`.
    Arc::clone(&dispatcher).spawn(
        scope,
        fx_pairs,
        lease,
        true,
        Some("secret-stooq-key".to_string()),
    );

    // Wait for the terminal completion event — it fires after the per-asset fetch
    // loop, so `fetch_price` has run by the time it arrives (event-driven, not a
    // busy-sleep poll).
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            rx.changed()
                .await
                .expect("bus closed before AssetPriceFetchCompleted arrived");
            if matches!(*rx.borrow(), Event::AssetPriceFetchCompleted { .. }) {
                return;
            }
        }
    })
    .await
    .expect("AssetPriceFetchCompleted within timeout");

    let keys = seen_keys.lock().expect("seen_keys lock").clone();
    assert_eq!(
        keys,
        vec!["secret-stooq-key".to_string()],
        "KEY-043: the stored Stooq key must be threaded into provider.fetch_price"
    );
}

/// KEY-053 — in keyless mode (`use_api_key = false`, no stored key) the KEY-044
/// short-circuit is suppressed: `provider.fetch_price` is still called, with `None`
/// for the api key (the anonymous request).
#[tokio::test]
async fn dispatcher_keyless_mode_fetches_without_key() {
    use std::sync::Mutex;
    use vault_compass_lib::context::asset::{
        Asset, AssetClass, AssetPriceRepository, PriceProvider, Quote, SYSTEM_CATEGORY_ID,
    };
    use vault_compass_lib::context::currency::CurrencyPair;
    use vault_compass_lib::core::event_bus::Event;
    use vault_compass_lib::use_cases::asset_price_fetch::dispatcher::Dispatcher;
    use vault_compass_lib::use_cases::asset_price_fetch::FetchGuard;

    // Records whether `fetch_price` ran and whether it received a key.
    struct KeylessRecordingProvider {
        seen: Arc<Mutex<Vec<Option<String>>>>,
    }
    #[async_trait::async_trait]
    impl PriceProvider for KeylessRecordingProvider {
        async fn fetch_price(
            &self,
            _symbol: &str,
            api_key: Option<String>,
        ) -> anyhow::Result<Option<Quote>> {
            self.seen.lock().expect("seen lock").push(api_key);
            Ok(Some(Quote {
                price: 100_000_000,
                date: None,
            }))
        }
    }

    let pool = make_pool().await;
    let bus = Arc::new(SideEffectEventBus::new());
    let asset_service = Arc::new(AssetService::new(
        Box::new(SqliteAssetRepository::new(pool.clone())),
        Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
        Box::new(SqliteAssetPriceRepository::new(pool.clone())),
    ));
    let asset: Asset = asset_service
        .create_asset(vault_compass_lib::context::asset::CreateAssetDTO {
            name: "Apple".to_string(),
            reference: "AAPL".to_string(),
            isin: None,
            class: AssetClass::Stocks,
            currency: "USD".to_string(),
            risk_level: 4,
            category_id: SYSTEM_CATEGORY_ID.to_string(),
            exchange: None,
        })
        .await
        .expect("seed asset");

    let seen = Arc::new(Mutex::new(Vec::<Option<String>>::new()));
    let provider = Arc::new(KeylessRecordingProvider {
        seen: Arc::clone(&seen),
    });
    let price_repo: Arc<dyn AssetPriceRepository> =
        Arc::new(SqliteAssetPriceRepository::new(pool.clone()));
    let dispatcher = Arc::new(Dispatcher::new(
        provider,
        price_repo,
        Arc::clone(&bus),
        Arc::new(CurrencyService::new(
            Box::new(SqliteCurrencyPairRepository::new(pool.clone())),
            Box::new(SqliteCurrencyRateRepository::new(pool.clone())),
        )),
        Arc::new(|| chrono::Local::now().date_naive()),
    ));

    let scope: Vec<(Asset, String)> = vec![(asset, "aapl.us".to_string())];
    let fx_pairs: Vec<CurrencyPair> = Vec::new();
    let lease = Arc::new(FetchGuard::new())
        .try_acquire()
        .expect("guard free at start");

    let mut rx = bus.subscribe();
    // KEY-053 — keyless mode (use_api_key = false) with no key: must NOT short-circuit.
    Arc::clone(&dispatcher).spawn(scope, fx_pairs, lease, false, None);

    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            rx.changed().await.expect("bus closed");
            if matches!(*rx.borrow(), Event::AssetPriceFetchCompleted { .. }) {
                return;
            }
        }
    })
    .await
    .expect("AssetPriceFetchCompleted within timeout");

    let seen = seen.lock().expect("seen lock").clone();
    assert_eq!(
        seen,
        vec![None],
        "KEY-053: keyless mode must call fetch_price once with no api key (not short-circuit)"
    );
}
