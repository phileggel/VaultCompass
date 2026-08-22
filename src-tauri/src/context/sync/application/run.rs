//! One sync run — **publish-only in PR-B** (SYN-060 publish half, SYN-061, SYN-067, SYN-069).
//! Collects this device's unpublished changes, seals one segment, rewrites the manifest, and
//! marks the rows published. Reading other devices' areas, resolving, and applying land in
//! PR-C — `applied_changes`, `held_back_changes`, `dropped_changes`, and `notices_raised` are
//! always 0 on the `SyncReport` this produces.

use std::sync::Arc;

use crate::context::sync::domain::{
    ChangeLogRepository, FolderStore, Manifest, Segment, SyncDevice, SyncFailure, SyncReport,
    SyncStateRepository, SyncStatus,
};
use crate::context::sync::error::SyncError;
use crate::context::sync::infrastructure::codec::{
    decode_header, encode_manifest, encode_segment, header_data_format_version, DATA_FORMAT_VERSION,
};
use crate::context::sync::infrastructure::crypto::{verify_check, Key};

/// Executes a publish-only sync run against a device's own unpublished changes.
pub struct SyncRun {
    change_log: Arc<dyn ChangeLogRepository>,
    state_repo: Arc<dyn SyncStateRepository>,
    folder_store: Arc<dyn FolderStore>,
}

/// What the folder header says about continuing this run (SYN-035/084).
enum HeaderGate {
    /// The header is this portfolio's and readable: publish.
    Proceed,
    /// The header was written in a newer data format: publish anyway, and say so.
    UpdateRequired(u32),
    /// No header, or one whose passphrase check no longer matches the kept key: the
    /// portfolio was started over elsewhere.
    Reset,
}

fn header_gate(header_bytes: Option<&[u8]>, key: &Key) -> HeaderGate {
    let Some(bytes) = header_bytes else {
        return HeaderGate::Reset;
    };
    if let Some(version) =
        header_data_format_version(bytes).filter(|version| *version > DATA_FORMAT_VERSION)
    {
        return HeaderGate::UpdateRequired(version);
    }
    match decode_header(bytes) {
        Ok(header) if verify_check(key, &header.passphrase_check) => HeaderGate::Proceed,
        _ => HeaderGate::Reset,
    }
}

/// Loads this device's kept key (SYN-052). `Ok(None)` when the stored bytes cannot be a key —
/// the device can open nothing, which is the reset case.
pub(super) async fn kept_key(
    change_log: &dyn ChangeLogRepository,
) -> Result<Option<Key>, SyncError> {
    Ok(change_log
        .kept_key_bytes()
        .await?
        .and_then(|bytes| Key::from_bytes(bytes).ok()))
}

fn report(device: &SyncDevice, published_changes: u32, failures: Vec<SyncFailure>) -> SyncReport {
    let completed_at = chrono::Utc::now().to_rfc3339();
    SyncReport {
        published_changes,
        applied_changes: 0,
        held_back_changes: 0,
        dropped_changes: 0,
        notices_raised: 0,
        failures: failures.clone(),
        completed_at: completed_at.clone(),
        status: SyncStatus::for_device(device, Some(completed_at), failures),
    }
}

/// The folder condition a failed read or write reveals, as a run failure (SYN-069).
fn folder_failure(error: SyncError) -> Result<SyncFailure, SyncError> {
    match error {
        SyncError::FolderUnavailable { problem } | SyncError::PublishFailed { problem } => {
            Ok(SyncFailure::FolderUnavailable { problem })
        }
        other => Err(other),
    }
}

impl SyncRun {
    /// Creates a run bound to the given change log, sync state, and folder.
    pub fn new(
        change_log: Arc<dyn ChangeLogRepository>,
        state_repo: Arc<dyn SyncStateRepository>,
        folder_store: Arc<dyn FolderStore>,
    ) -> Self {
        Self {
            change_log,
            state_repo,
            folder_store,
        }
    }

    /// Publishes this device's unpublished changes as one segment and rewrites its manifest
    /// (SYN-060 publish half, SYN-061, SYN-067). Never rejects for the folder's state
    /// (SYN-062) — `FolderUnavailable`, `UpdateRequired`, and `PortfolioReset` surface in the
    /// returned report's `failures`, not as an `Err`.
    pub async fn publish(&self, device: &SyncDevice) -> Result<SyncReport, SyncError> {
        self.folder_store.retarget(&device.folder);
        if let Err(problem) = self.folder_store.check_available().await {
            return Ok(report(
                device,
                0,
                vec![SyncFailure::FolderUnavailable { problem }],
            ));
        }
        let header_bytes = match self.folder_store.read_header_bytes().await {
            Ok(bytes) => bytes,
            Err(error) => return Ok(report(device, 0, vec![folder_failure(error)?])),
        };
        let Some(key) = kept_key(self.change_log.as_ref()).await? else {
            return self.pause_for_reset(device).await;
        };
        let mut failures = Vec::new();
        match header_gate(header_bytes.as_deref(), &key) {
            HeaderGate::Proceed => {}
            HeaderGate::UpdateRequired(data_format_version) => {
                failures.push(SyncFailure::UpdateRequired {
                    data_format_version,
                });
            }
            HeaderGate::Reset => return self.pause_for_reset(device).await,
        }

        let changes = self.change_log.list_unpublished(&device.device_id).await?;
        let published_changes = changes.len() as u32;
        if let (Some(first), Some(last)) = (changes.first(), changes.last()) {
            let segment = Segment {
                device_id: device.device_id.clone(),
                first_sequence: first.sequence,
                last_sequence: last.sequence,
                data_format_version: DATA_FORMAT_VERSION,
                changes: changes.clone(),
            };
            let written = match encode_segment(&key, &segment) {
                Ok(bytes) => {
                    self.folder_store
                        .write_segment(
                            &device.device_id,
                            segment.first_sequence,
                            segment.last_sequence,
                            bytes,
                        )
                        .await
                }
                Err(error) => Err(error),
            };
            if let Err(error) = written {
                failures.push(folder_failure(error)?);
                return Ok(report(device, 0, failures));
            }
            self.change_log
                .mark_published(
                    &device.device_id,
                    segment.first_sequence,
                    segment.last_sequence,
                )
                .await?;
        }

        let manifest = Manifest {
            device_id: device.device_id.clone(),
            device_name: device.device_name.clone(),
            data_format_version: DATA_FORMAT_VERSION,
            latest_sequence: self
                .change_log
                .latest_published_sequence(&device.device_id)
                .await?,
        };
        let written = match encode_manifest(&key, &manifest) {
            Ok(bytes) => {
                self.folder_store
                    .write_manifest(&device.device_id, bytes)
                    .await
            }
            Err(error) => Err(error),
        };
        if let Err(error) = written {
            failures.push(folder_failure(error)?);
        }
        Ok(report(device, published_changes, failures))
    }

    /// SYN-084 — the portfolio was started over elsewhere: the device pauses itself and
    /// publishes nothing under the old key.
    async fn pause_for_reset(&self, device: &SyncDevice) -> Result<SyncReport, SyncError> {
        let paused = if device.paused {
            device.clone()
        } else {
            device.clone().pause()?
        };
        self.state_repo.save_device(&paused).await?;
        Ok(report(&paused, 0, vec![SyncFailure::PortfolioReset]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::sync::domain::{
        DerivationParameters, FolderHeader, MockFolderStore, MockSyncStateRepository,
    };
    use crate::context::sync::infrastructure::codec::encode_header;
    use crate::context::sync::infrastructure::crypto::make_check;
    use crate::context::sync::infrastructure::SqliteChangeLogRepository;
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::{Pool, Sqlite};

    fn run_over(
        pool: &Pool<Sqlite>,
        state_repo: Arc<dyn SyncStateRepository>,
        folder_store: MockFolderStore,
    ) -> SyncRun {
        SyncRun::new(
            Arc::new(SqliteChangeLogRepository::new(pool.clone())),
            state_repo,
            Arc::new(folder_store),
        )
    }

    async fn make_pool() -> Pool<Sqlite> {
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

    /// The key the seeded `sync_device` row keeps: 32 zero bytes.
    fn kept_key_for_tests() -> Key {
        Key::from_bytes(vec![0; 32]).expect("32 bytes is a valid key")
    }

    async fn seed_sync_device(pool: &Pool<Sqlite>, device_id: &str) {
        let derived_key = kept_key_for_tests().as_bytes().to_vec();
        sqlx::query(
            r#"INSERT INTO sync_device
               (id, device_id, device_name, folder, joined_at, paused, portfolio_created_at,
                logical_clock, derived_key, data_format_version)
               VALUES (1, ?, 'Desktop', '/tmp/sync', '2026-08-22T00:00:00Z', 0,
                       '2026-08-22T00:00:00Z', 0, ?, 1)"#,
        )
        .bind(device_id)
        .bind(derived_key)
        .execute(pool)
        .await
        .expect("seed sync_device");
    }

    async fn seed_unpublished_change(pool: &Pool<Sqlite>, device_id: &str, sequence: i64) {
        sqlx::query(
            r#"INSERT INTO changes
               (device_id, sequence, logical_timestamp, based_on, record_kind, record_identity,
                operation, origin, content, published)
               VALUES (?, ?, ?, NULL, 'Account', ?, 'Created', 'User', ?, 0)"#,
        )
        .bind(device_id)
        .bind(sequence)
        .bind(format!("{sequence:020}"))
        .bind(format!("account-{sequence}"))
        .bind(format!("{{\"id\":\"account-{sequence}\"}}"))
        .execute(pool)
        .await
        .expect("seed unpublished change");
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

    /// The header of the portfolio the seeded device follows: its passphrase check was made
    /// under the kept key.
    fn matching_header_bytes() -> Vec<u8> {
        encode_header(&FolderHeader {
            derivation_parameters: DerivationParameters {
                salt: vec![1; 16],
                memory_cost_kib: 19_456,
                iterations: 2,
                parallelism: 1,
            },
            passphrase_check: make_check(&kept_key_for_tests()),
            data_format_version: DATA_FORMAT_VERSION,
            created_at: "00000000000000000001".into(),
            created_by_device_id: "desktop-device".into(),
        })
        .expect("a valid header encodes")
    }

    // SYN-060/067 — unpublished changes are collected into one segment, and the rows are
    // marked published afterward.
    #[tokio::test]
    async fn publish_collects_unpublished_changes_and_marks_them_published() {
        let pool = make_pool().await;
        seed_sync_device(&pool, "desktop-device").await;
        seed_unpublished_change(&pool, "desktop-device", 1).await;
        seed_unpublished_change(&pool, "desktop-device", 2).await;

        let state_repo = Arc::new(MockSyncStateRepository::new());
        let mut folder_store = MockFolderStore::new();
        folder_store.expect_retarget().return_const(());
        folder_store.expect_check_available().returning(|| Ok(()));
        folder_store
            .expect_read_header_bytes()
            .returning(|| Ok(Some(matching_header_bytes())));
        folder_store
            .expect_write_segment()
            .withf(|device_id, first, last, _| {
                device_id == "desktop-device" && *first == 1 && *last == 2
            })
            .times(1)
            .returning(|_, _, _, _| Ok(()));
        folder_store
            .expect_write_manifest()
            .times(1)
            .returning(|_, _| Ok(()));
        let run = run_over(&pool, state_repo, folder_store);

        let report = run
            .publish(&device())
            .await
            .expect("publish must not error");
        assert_eq!(
            report.published_changes, 2,
            "both unpublished rows must be counted"
        );
        assert!(report.failures.is_empty());

        let published_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM changes WHERE published = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            published_count, 2,
            "SYN-031/067: the rows must be marked published"
        );
    }

    // SYN-067 — a second run with nothing new writes no segment and reports 0 published; the
    // manifest is still rewritten (a rename travels through it, SYN-072).
    #[tokio::test]
    async fn second_run_with_nothing_new_publishes_nothing() {
        let pool = make_pool().await;
        seed_sync_device(&pool, "desktop-device").await;
        seed_unpublished_change(&pool, "desktop-device", 1).await;

        let state_repo = Arc::new(MockSyncStateRepository::new());
        let mut folder_store = MockFolderStore::new();
        folder_store.expect_retarget().return_const(());
        folder_store.expect_check_available().returning(|| Ok(()));
        folder_store
            .expect_read_header_bytes()
            .returning(|| Ok(Some(matching_header_bytes())));
        folder_store
            .expect_write_segment()
            .times(1)
            .returning(|_, _, _, _| Ok(()));
        folder_store
            .expect_write_manifest()
            .times(2)
            .returning(|_, _| Ok(()));
        let run = run_over(&pool, state_repo, folder_store);

        run.publish(&device()).await.unwrap();
        let second = run.publish(&device()).await.unwrap();
        assert_eq!(second.published_changes, 0);
    }

    // SYN-069 — an unavailable folder is reported in `failures`, not thrown; unpublished
    // changes stay unpublished so the next run retries them.
    #[tokio::test]
    async fn folder_unavailable_reports_failure_and_leaves_changes_unpublished() {
        let pool = make_pool().await;
        seed_sync_device(&pool, "desktop-device").await;
        seed_unpublished_change(&pool, "desktop-device", 1).await;

        let state_repo = Arc::new(MockSyncStateRepository::new());
        let mut folder_store = MockFolderStore::new();
        folder_store.expect_retarget().return_const(());
        folder_store
            .expect_check_available()
            .returning(|| Err(crate::context::sync::domain::FolderProblem::Missing));
        let run = run_over(&pool, state_repo, folder_store);

        let report = run
            .publish(&device())
            .await
            .expect("SYN-062: never rejects for folder state");
        assert!(
            report
                .failures
                .iter()
                .any(|f| matches!(f, SyncFailure::FolderUnavailable { .. })),
            "an unavailable folder must surface in SyncReport.failures: {:?}",
            report.failures
        );

        let published_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM changes WHERE published = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            published_count, 0,
            "unpublished rows must stay unpublished for the retry"
        );
    }

    // SYN-069 — a segment write that fails midway is reported, and the rows stay unpublished.
    #[tokio::test]
    async fn failed_segment_write_reports_failure_and_leaves_changes_unpublished() {
        let pool = make_pool().await;
        seed_sync_device(&pool, "desktop-device").await;
        seed_unpublished_change(&pool, "desktop-device", 1).await;

        let state_repo = Arc::new(MockSyncStateRepository::new());
        let mut folder_store = MockFolderStore::new();
        folder_store.expect_retarget().return_const(());
        folder_store.expect_check_available().returning(|| Ok(()));
        folder_store
            .expect_read_header_bytes()
            .returning(|| Ok(Some(matching_header_bytes())));
        folder_store.expect_write_segment().returning(|_, _, _, _| {
            Err(SyncError::PublishFailed {
                problem: crate::context::sync::domain::FolderProblem::OutOfSpace,
            })
        });
        let run = run_over(&pool, state_repo, folder_store);

        let report = run
            .publish(&device())
            .await
            .expect("SYN-062: never rejects");
        assert!(report.failures.contains(&SyncFailure::FolderUnavailable {
            problem: crate::context::sync::domain::FolderProblem::OutOfSpace
        }));
        assert_eq!(report.published_changes, 0);

        let published_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM changes WHERE published = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(published_count, 0);
    }

    // SYN-035 — a too-new header/manifest already in the folder reports UpdateRequired, but
    // this device's own segment still publishes (publishing continues deliberately).
    #[tokio::test]
    async fn too_new_header_reports_update_required_but_this_device_still_publishes() {
        let pool = make_pool().await;
        seed_sync_device(&pool, "desktop-device").await;
        seed_unpublished_change(&pool, "desktop-device", 1).await;

        let state_repo = Arc::new(MockSyncStateRepository::new());
        let mut folder_store = MockFolderStore::new();
        folder_store.expect_retarget().return_const(());
        folder_store.expect_check_available().returning(|| Ok(()));
        folder_store
            .expect_read_header_bytes()
            .returning(|| Ok(Some(b"{\"data_format_version\":99}".to_vec())));
        folder_store
            .expect_write_segment()
            .returning(|_, _, _, _| Ok(()));
        folder_store
            .expect_write_manifest()
            .returning(|_, _| Ok(()));
        let run = run_over(&pool, state_repo, folder_store);

        let report = run
            .publish(&device())
            .await
            .expect("SYN-062: never rejects");
        assert!(
            report
                .failures
                .iter()
                .any(|f| matches!(f, SyncFailure::UpdateRequired { data_format_version } if *data_format_version == 99)),
            "a too-new format must surface as UpdateRequired: {:?}",
            report.failures
        );
        assert_eq!(
            report.published_changes, 1,
            "SYN-035: the gated device still publishes its own real user intent"
        );
    }

    // SYN-084 — a passphrase-check mismatch against the kept key reports PortfolioReset and
    // pauses the device; nothing is published under the old key into the reset folder.
    #[tokio::test]
    async fn passphrase_mismatch_reports_portfolio_reset_and_pauses_the_device() {
        let pool = make_pool().await;
        seed_sync_device(&pool, "desktop-device").await;
        seed_unpublished_change(&pool, "desktop-device", 1).await;

        let mut state_repo = MockSyncStateRepository::new();
        state_repo.expect_save_device().returning(|_| Ok(()));
        let mut folder_store = MockFolderStore::new();
        folder_store.expect_retarget().return_const(());
        folder_store.expect_check_available().returning(|| Ok(()));
        folder_store
            .expect_read_header_bytes()
            .returning(|| Ok(Some(b"{\"passphrase_check\":\"does-not-match\"}".to_vec())));
        let run = run_over(&pool, Arc::new(state_repo), folder_store);

        let report = run
            .publish(&device())
            .await
            .expect("SYN-062: never rejects");
        assert!(
            report.failures.contains(&SyncFailure::PortfolioReset),
            "a passphrase-check mismatch must surface as PortfolioReset: {:?}",
            report.failures
        );
        assert!(
            report.status.paused,
            "SYN-084: the device pauses itself on a detected reset"
        );

        let published_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM changes WHERE published = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            published_count, 0,
            "SYN-084: nothing is ever published under the old key into a reset folder"
        );
    }

    // SYN-084 — a folder with no header at all is a reset too.
    #[tokio::test]
    async fn missing_header_reports_portfolio_reset() {
        let pool = make_pool().await;
        seed_sync_device(&pool, "desktop-device").await;

        let mut state_repo = MockSyncStateRepository::new();
        state_repo.expect_save_device().returning(|_| Ok(()));
        let mut folder_store = MockFolderStore::new();
        folder_store.expect_retarget().return_const(());
        folder_store.expect_check_available().returning(|| Ok(()));
        folder_store
            .expect_read_header_bytes()
            .returning(|| Ok(None));
        let run = run_over(&pool, Arc::new(state_repo), folder_store);

        let report = run
            .publish(&device())
            .await
            .expect("SYN-062: never rejects");
        assert!(report.failures.contains(&SyncFailure::PortfolioReset));
    }
}
