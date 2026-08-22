//! Cross-BC orchestration for the seven sync commands that read from or write into the
//! account/asset/currency bounded contexts (D3, ADR-003/ADR-004): a first publish reads the
//! whole portfolio, a join rebuilds it (PR-C), and status enriches with inconsistent holdings
//! (PR-C). Injects each BC's service — never a repository, never a sibling use case (B18).

use std::sync::Arc;

use crate::context::account::AccountService;
use crate::context::asset::{AssetService, SYSTEM_CATEGORY_ID};
use crate::context::currency::CurrencyService;
use crate::context::sync::{
    ensure_device_name, ensure_passphrase_length, header_data_format_version, FirstPublish,
    FolderStore, SyncError, SyncFailure, SyncFolderState, SyncReport, SyncRun, SyncService,
    SyncStateRepository, SyncStatus, DATA_FORMAT_VERSION,
};
use crate::core::cash::{is_cash_asset, SYSTEM_CASH_CATEGORY_ID};

use super::error::{PortfolioSyncError, PortfolioSyncTask};

/// Orchestrates the cross-BC sync commands (D3).
pub struct PortfolioSyncOrchestrator {
    account_service: Arc<AccountService>,
    asset_service: Arc<AssetService>,
    currency_service: Arc<CurrencyService>,
    sync_service: Arc<SyncService>,
    first_publish: Arc<FirstPublish>,
    sync_run: Arc<SyncRun>,
    state_repo: Arc<dyn SyncStateRepository>,
    folder_store: Arc<dyn FolderStore>,
}

impl PortfolioSyncOrchestrator {
    /// Creates the orchestrator injecting every BC service it may need to read from or write
    /// into, plus the sync BC's own device lifecycle, first-publish, and run components.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        account_service: Arc<AccountService>,
        asset_service: Arc<AssetService>,
        currency_service: Arc<CurrencyService>,
        sync_service: Arc<SyncService>,
        first_publish: Arc<FirstPublish>,
        sync_run: Arc<SyncRun>,
        state_repo: Arc<dyn SyncStateRepository>,
        folder_store: Arc<dyn FolderStore>,
    ) -> Self {
        Self {
            account_service,
            asset_service,
            currency_service,
            sync_service,
            first_publish,
            sync_run,
            state_repo,
            folder_store,
        }
    }

    /// Pre-flight read of a candidate folder (SYN-011/014/019). Never rejects.
    pub async fn inspect_sync_folder(
        &self,
        folder: String,
    ) -> Result<SyncFolderState, PortfolioSyncError> {
        self.folder_store.retarget(&folder);
        let mut problem = self.folder_store.check_available().await.err();
        let header = match problem {
            Some(_) => None,
            None => match self.folder_store.read_header_bytes().await {
                Ok(header) => header,
                Err(SyncError::FolderUnavailable { problem: found }) => {
                    problem = Some(found);
                    None
                }
                Err(error) => return Err(error.into()),
            },
        };
        let holds_portfolio = header.is_some();
        let data_format_version = header.as_deref().and_then(header_data_format_version);
        Ok(SyncFolderState {
            problem,
            holds_portfolio,
            data_format_version,
            format_readable: !holds_portfolio
                || data_format_version.is_some_and(|version| version <= DATA_FORMAT_VERSION),
            installation_holds_user_data: self.installation_holds_user_data().await?,
        })
    }

    /// Whether this installation holds user-entered records (SYN-014): any account, any
    /// user-created asset or category, any currency pair. System-seeded records (SYN-027)
    /// do not count.
    async fn installation_holds_user_data(&self) -> Result<bool, PortfolioSyncError> {
        if !self.account_service.get_all().await?.is_empty() {
            return Ok(true);
        }
        let assets = self.asset_service.get_all_assets_with_archived().await?;
        if assets.iter().any(|asset| !is_cash_asset(&asset.id)) {
            return Ok(true);
        }
        let categories = self.asset_service.get_all_categories().await?;
        if categories.iter().any(|category| {
            category.id != SYSTEM_CATEGORY_ID && category.id != SYSTEM_CASH_CATEGORY_ID
        }) {
            return Ok(true);
        }
        Ok(!self
            .currency_service
            .list_currency_pairs()
            .await?
            .is_empty())
    }

    /// Enables sync (SYN-011). The first-device branch (no header yet) delegates to
    /// `FirstPublish`; the join branch (a header already exists) always returns
    /// `InstallationHoldsUserData` in PR-B — the rebuild ships in PR-C.
    pub async fn enable_sync(
        &self,
        folder: String,
        passphrase: String,
        device_name: String,
    ) -> Result<SyncStatus, PortfolioSyncError> {
        if self.state_repo.get_device().await?.is_some() {
            return Err(SyncError::AlreadyEnabled.into());
        }
        self.folder_store.retarget(&folder);
        if self.folder_store.read_header_bytes().await?.is_some() {
            return Err(PortfolioSyncTask::InstallationHoldsUserData.into());
        }
        Ok(self
            .first_publish
            .enable_as_first_device(folder, passphrase, device_name)
            .await?)
    }

    /// Re-enrolls this device as the new origin of the portfolio under a new passphrase,
    /// clearing the folder first (SYN-071).
    pub async fn start_sync_over(
        &self,
        folder: String,
        passphrase: String,
        device_name: String,
    ) -> Result<SyncStatus, PortfolioSyncError> {
        ensure_passphrase_length(&passphrase)?;
        ensure_device_name(&device_name)?;
        self.folder_store.retarget(&folder);
        self.folder_store
            .check_available()
            .await
            .map_err(|problem| SyncError::FolderUnavailable { problem })?;
        for device_id in self.folder_store.list_device_ids().await? {
            self.folder_store.remove_device_area(&device_id).await?;
        }
        self.folder_store.remove_header().await?;
        Ok(self
            .first_publish
            .enable_as_first_device(folder, passphrase, device_name)
            .await?)
    }

    /// Designates a different folder for an already-enrolled device (SYN-074): the same
    /// portfolio, or an empty folder (first-device path).
    pub async fn change_sync_folder(
        &self,
        folder: String,
    ) -> Result<SyncStatus, PortfolioSyncError> {
        let device = self
            .state_repo
            .get_device()
            .await?
            .ok_or(SyncError::SyncDisabled)?;
        Ok(self.first_publish.change_folder(device, folder).await?)
    }

    /// Runs a publish-only sync immediately (SYN-061).
    pub async fn sync_now(&self) -> Result<SyncReport, PortfolioSyncError> {
        let device = self
            .state_repo
            .get_device()
            .await?
            .ok_or(SyncError::SyncDisabled)?;
        device.ensure_not_paused()?;
        let report = self.sync_run.publish(&device).await?;
        self.sync_service.remember_run(&report);
        Ok(report)
    }

    /// Resumes sync on a paused device: publishes paused-era changes, then runs as `sync_now`
    /// (SYN-073). The device stays paused while the folder is unavailable or the publish
    /// fails, and after a detected reset (SYN-084).
    pub async fn resume_sync(&self) -> Result<SyncReport, PortfolioSyncError> {
        let device = self.sync_service.resume_sync_precondition().await?;
        self.folder_store.retarget(&device.folder);
        self.folder_store
            .check_available()
            .await
            .map_err(|problem| SyncError::FolderUnavailable { problem })?;
        let resumed = device.resume()?;
        let report = self.sync_run.publish(&resumed).await?;
        if let Some(problem) = report.failures.iter().find_map(|failure| match failure {
            SyncFailure::FolderUnavailable { problem } => Some(*problem),
            _ => None,
        }) {
            return Err(SyncError::PublishFailed { problem }.into());
        }
        if !report.failures.contains(&SyncFailure::PortfolioReset) {
            self.state_repo.save_device(&resumed).await?;
        }
        self.sync_service.remember_run(&report);
        Ok(report)
    }

    /// Reads the current sync status, enriched with inconsistent holdings (PR-C).
    pub async fn get_sync_status(&self) -> Result<SyncStatus, PortfolioSyncError> {
        Ok(self.sync_service.status().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::account::{
        SqliteAccountRepository, SqliteHoldingRepository, SqliteTransactionRepository,
        UpdateFrequency,
    };
    use crate::context::asset::{
        SqliteAssetCategoryRepository, SqliteAssetPriceRepository, SqliteAssetRepository,
    };
    use crate::context::currency::{SqliteCurrencyPairRepository, SqliteCurrencyRateRepository};
    use crate::context::sync::{
        FolderProblem, MockFolderStore, MockSyncStateRepository, SqliteChangeLogRepository,
        SqliteSyncStateRepository, SyncDevice,
    };
    use crate::use_cases::portfolio_sync::{ServicePortfolioSnapshot, ServiceRankStamper};
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

    struct Ctx {
        orchestrator: PortfolioSyncOrchestrator,
    }

    fn build_ctx_with_state_repo(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        state_repo: Arc<dyn SyncStateRepository>,
        folder_store: Arc<dyn FolderStore>,
    ) -> Ctx {
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
        let change_log = Arc::new(SqliteChangeLogRepository::new(pool.clone()));
        let sync_run = Arc::new(SyncRun::new(
            change_log.clone(),
            Arc::clone(&state_repo),
            Arc::clone(&folder_store),
        ));
        let sync_service = Arc::new(
            SyncService::new(Arc::clone(&state_repo), Arc::clone(&folder_store))
                .with_run(Arc::clone(&sync_run)),
        );
        let snapshot = Arc::new(ServicePortfolioSnapshot::new(
            Arc::clone(&account_service),
            Arc::clone(&asset_service),
            Arc::clone(&currency_service),
        ));
        let rank_stamper = Arc::new(ServiceRankStamper::new(
            Arc::clone(&account_service),
            Arc::clone(&asset_service),
            Arc::clone(&currency_service),
        ));
        let first_publish = Arc::new(FirstPublish::new(
            change_log,
            Arc::clone(&state_repo),
            Arc::clone(&folder_store),
            rank_stamper,
            snapshot,
        ));
        let orchestrator = PortfolioSyncOrchestrator::new(
            account_service,
            asset_service,
            currency_service,
            sync_service,
            first_publish,
            sync_run,
            state_repo,
            folder_store,
        );
        Ctx { orchestrator }
    }

    fn device() -> SyncDevice {
        SyncDevice::restore(
            "desktop-device".into(),
            "Desktop".into(),
            "/tmp/sync".into(),
            "2026-08-22T00:00:00Z".into(),
            false,
            "2026-08-22T00:00:00Z".into(),
            1,
        )
    }

    // SYN-014 — a candidate folder already holding user data on THIS installation is flagged,
    // regardless of the folder's own content.
    #[tokio::test]
    async fn inspect_sync_folder_flags_installation_holds_user_data_when_local_accounts_exist() {
        let pool = make_pool().await;
        let account_service = AccountService::new(
            Box::new(SqliteAccountRepository::new(pool.clone())),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
            Box::new(SqliteTransactionRepository::new(pool.clone())),
        );
        account_service
            .create(
                "Portfolio".into(),
                String::new(),
                "USD".into(),
                UpdateFrequency::ManualMonth,
                false,
            )
            .await
            .unwrap();

        let dir = tempfile::tempdir().unwrap();
        let ctx = build_ctx_with_state_repo(
            &pool,
            Arc::new(SqliteSyncStateRepository::new(pool.clone())),
            Arc::new(crate::context::sync::FsFolderStore::new(dir.path())),
        );

        let state = ctx
            .orchestrator
            .inspect_sync_folder(dir.path().to_string_lossy().to_string())
            .await
            .expect("inspect_sync_folder never rejects (SYN-011/014/019)");
        assert!(
            state.installation_holds_user_data,
            "SYN-014: an existing local account must be flagged"
        );
        assert!(state.problem.is_none());
        assert!(!state.holds_portfolio);
        assert!(state.format_readable);
    }

    // SYN-019 — a fresh installation and a missing folder: the problem is reported, never
    // thrown.
    #[tokio::test]
    async fn inspect_sync_folder_reports_a_missing_folder_on_a_fresh_installation() {
        let pool = make_pool().await;
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        let ctx = build_ctx_with_state_repo(
            &pool,
            Arc::new(SqliteSyncStateRepository::new(pool.clone())),
            Arc::new(crate::context::sync::FsFolderStore::new(&missing)),
        );

        let state = ctx
            .orchestrator
            .inspect_sync_folder(missing.to_string_lossy().to_string())
            .await
            .unwrap();
        assert_eq!(state.problem, Some(FolderProblem::Missing));
        assert!(!state.installation_holds_user_data);
    }

    // SYN-019/035 — a folder holding a portfolio in a newer data format is reported as
    // unreadable for joining.
    #[tokio::test]
    async fn inspect_sync_folder_reports_a_too_new_portfolio_as_unreadable() {
        let pool = make_pool().await;
        let mut folder_store = MockFolderStore::new();
        folder_store.expect_retarget().return_const(());
        folder_store.expect_check_available().returning(|| Ok(()));
        folder_store
            .expect_read_header_bytes()
            .returning(|| Ok(Some(b"{\"data_format_version\":99}".to_vec())));
        let ctx = build_ctx_with_state_repo(
            &pool,
            Arc::new(SqliteSyncStateRepository::new(pool.clone())),
            Arc::new(folder_store),
        );

        let state = ctx
            .orchestrator
            .inspect_sync_folder("/tmp/sync".into())
            .await
            .unwrap();
        assert!(state.holds_portfolio);
        assert_eq!(state.data_format_version, Some(99));
        assert!(!state.format_readable);
    }

    // SYN-010 — enable_sync rejects AlreadyEnabled when a sync_device row already exists.
    #[tokio::test]
    async fn enable_sync_rejects_already_enabled() {
        let pool = make_pool().await;
        let mut state_repo = MockSyncStateRepository::new();
        state_repo
            .expect_get_device()
            .returning(|| Ok(Some(device())));
        let ctx = build_ctx_with_state_repo(
            &pool,
            Arc::new(state_repo),
            Arc::new(MockFolderStore::new()),
        );

        let result = ctx
            .orchestrator
            .enable_sync(
                "/tmp/sync".into(),
                "correct horse battery staple".into(),
                "Desktop".into(),
            )
            .await;
        assert!(matches!(
            result,
            Err(PortfolioSyncError::Sync(SyncError::AlreadyEnabled))
        ));
    }

    // D3/SYN-014 — the join branch (a header already exists) always returns
    // InstallationHoldsUserData in PR-B; the rebuild is PR-C's job.
    #[tokio::test]
    async fn enable_sync_join_branch_always_returns_installation_holds_user_data_in_pr_b() {
        let pool = make_pool().await;
        let mut state_repo = MockSyncStateRepository::new();
        state_repo.expect_get_device().returning(|| Ok(None));
        let mut folder_store = MockFolderStore::new();
        folder_store.expect_retarget().return_const(());
        folder_store
            .expect_read_header_bytes()
            .returning(|| Ok(Some(b"{\"data_format_version\":1}".to_vec())));
        let ctx = build_ctx_with_state_repo(&pool, Arc::new(state_repo), Arc::new(folder_store));

        let result = ctx
            .orchestrator
            .enable_sync(
                "/tmp/sync".into(),
                "correct horse battery staple".into(),
                "Desktop".into(),
            )
            .await;
        assert!(matches!(
            result,
            Err(PortfolioSyncError::Task(
                PortfolioSyncTask::InstallationHoldsUserData
            ))
        ));
    }

    // SYN-010 — sync_now rejects SyncDisabled while never enabled.
    #[tokio::test]
    async fn sync_now_rejects_when_disabled() {
        let pool = make_pool().await;
        let mut state_repo = MockSyncStateRepository::new();
        state_repo.expect_get_device().returning(|| Ok(None));
        let ctx = build_ctx_with_state_repo(
            &pool,
            Arc::new(state_repo),
            Arc::new(MockFolderStore::new()),
        );

        let result = ctx.orchestrator.sync_now().await;
        assert!(matches!(
            result,
            Err(PortfolioSyncError::Sync(SyncError::SyncDisabled))
        ));
    }

    // SYN-070 — sync_now rejects SyncPaused on a paused device.
    #[tokio::test]
    async fn sync_now_rejects_when_paused() {
        let pool = make_pool().await;
        let mut state_repo = MockSyncStateRepository::new();
        let mut paused = device();
        paused.paused = true;
        state_repo
            .expect_get_device()
            .returning(move || Ok(Some(paused.clone())));
        let ctx = build_ctx_with_state_repo(
            &pool,
            Arc::new(state_repo),
            Arc::new(MockFolderStore::new()),
        );

        let result = ctx.orchestrator.sync_now().await;
        assert!(matches!(
            result,
            Err(PortfolioSyncError::Sync(SyncError::SyncPaused))
        ));
    }

    // SYN-073/069 — resume_sync rejects FolderUnavailable while the folder is unavailable, and
    // the device stays paused.
    #[tokio::test]
    async fn resume_sync_rejects_folder_unavailable_and_keeps_the_device_paused() {
        let pool = make_pool().await;
        let mut state_repo = MockSyncStateRepository::new();
        let mut paused = device();
        paused.paused = true;
        state_repo
            .expect_get_device()
            .returning(move || Ok(Some(paused.clone())));
        state_repo.expect_save_device().times(0);
        let mut folder_store = MockFolderStore::new();
        folder_store.expect_retarget().return_const(());
        folder_store
            .expect_check_available()
            .returning(|| Err(FolderProblem::Unmounted));
        let ctx = build_ctx_with_state_repo(&pool, Arc::new(state_repo), Arc::new(folder_store));

        let result = ctx.orchestrator.resume_sync().await;
        assert!(matches!(
            result,
            Err(PortfolioSyncError::Sync(SyncError::FolderUnavailable {
                problem: FolderProblem::Unmounted
            }))
        ));
    }

    // SYN-073 — resume_sync rejects NotPaused on an active device.
    #[tokio::test]
    async fn resume_sync_rejects_when_not_paused() {
        let pool = make_pool().await;
        let mut state_repo = MockSyncStateRepository::new();
        state_repo
            .expect_get_device()
            .returning(|| Ok(Some(device())));
        let ctx = build_ctx_with_state_repo(
            &pool,
            Arc::new(state_repo),
            Arc::new(MockFolderStore::new()),
        );

        let result = ctx.orchestrator.resume_sync().await;
        assert!(matches!(
            result,
            Err(PortfolioSyncError::Sync(SyncError::NotPaused))
        ));
    }

    // SYN-012/018 — start_sync_over validates its inputs before clearing anything.
    #[tokio::test]
    async fn start_sync_over_validates_inputs_before_clearing_the_folder() {
        let pool = make_pool().await;
        let ctx = build_ctx_with_state_repo(
            &pool,
            Arc::new(MockSyncStateRepository::new()),
            Arc::new(MockFolderStore::new()),
        );

        let short = ctx
            .orchestrator
            .start_sync_over("/tmp/sync".into(), "short".into(), "Desktop".into())
            .await;
        assert!(matches!(
            short,
            Err(PortfolioSyncError::Sync(SyncError::PassphraseTooShort {
                minimum: 12
            }))
        ));
        let blank = ctx
            .orchestrator
            .start_sync_over(
                "/tmp/sync".into(),
                "correct horse battery staple".into(),
                " ".into(),
            )
            .await;
        assert!(matches!(
            blank,
            Err(PortfolioSyncError::Sync(SyncError::DeviceNameBlank))
        ));
    }

    // SYN-010 — change_sync_folder rejects SyncDisabled while never enabled.
    #[tokio::test]
    async fn change_sync_folder_rejects_when_disabled() {
        let pool = make_pool().await;
        let mut state_repo = MockSyncStateRepository::new();
        state_repo.expect_get_device().returning(|| Ok(None));
        let ctx = build_ctx_with_state_repo(
            &pool,
            Arc::new(state_repo),
            Arc::new(MockFolderStore::new()),
        );

        let result = ctx
            .orchestrator
            .change_sync_folder("/tmp/elsewhere".into())
            .await;
        assert!(matches!(
            result,
            Err(PortfolioSyncError::Sync(SyncError::SyncDisabled))
        ));
    }

    // SYN-063 — get_sync_status delegates to SyncService and returns the disabled shape.
    #[tokio::test]
    async fn get_sync_status_returns_disabled_shape_when_never_enabled() {
        let pool = make_pool().await;
        let mut state_repo = MockSyncStateRepository::new();
        state_repo.expect_get_device().returning(|| Ok(None));
        let ctx = build_ctx_with_state_repo(
            &pool,
            Arc::new(state_repo),
            Arc::new(MockFolderStore::new()),
        );

        let status = ctx
            .orchestrator
            .get_sync_status()
            .await
            .expect("must not error");
        assert!(!status.enabled);
    }
}
