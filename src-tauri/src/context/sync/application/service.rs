//! `SyncService` — device lifecycle (pause/rename/leave), notice dismissal, and the sync-owned
//! half of `SyncStatus` assembly (D2). The cross-BC commands (`enable_sync`, `sync_now`, …)
//! live in `use_cases::portfolio_sync`; this service only ever touches sync-owned state.

use std::sync::{Arc, Mutex, PoisonError};

use crate::context::sync::application::run::SyncRun;
use crate::context::sync::domain::{
    FolderStore, SyncDevice, SyncFailure, SyncReport, SyncStateRepository, SyncStatus,
};
use crate::context::sync::error::SyncError;
use crate::core::logger::BACKEND;

/// What the last run of this process left behind for `SyncStatus` (SYN-063): when it
/// completed and what needs attention.
#[derive(Clone)]
struct LastRun {
    completed_at: String,
    failures: Vec<SyncFailure>,
}

/// Device lifecycle + status assembly for the sync bounded context.
pub struct SyncService {
    state_repo: Arc<dyn SyncStateRepository>,
    folder_store: Arc<dyn FolderStore>,
    sync_run: Option<Arc<SyncRun>>,
    last_run: Mutex<Option<LastRun>>,
}

impl SyncService {
    /// Creates the service bound to the given sync state and folder.
    pub fn new(
        state_repo: Arc<dyn SyncStateRepository>,
        folder_store: Arc<dyn FolderStore>,
    ) -> Self {
        Self {
            state_repo,
            folder_store,
            sync_run: None,
            last_run: Mutex::new(None),
        }
    }

    /// Attaches the publish run `leave_sync` flushes unpublished changes through (SYN-082).
    pub fn with_run(mut self, sync_run: Arc<SyncRun>) -> Self {
        self.sync_run = Some(sync_run);
        self
    }

    /// The attached run, or `DatabaseError` when none was wired (a wiring bug).
    fn sync_run(&self) -> Result<&SyncRun, SyncError> {
        self.sync_run.as_deref().ok_or_else(|| {
            tracing::error!(target: BACKEND, "sync_run not wired on SyncService");
            SyncError::DatabaseError
        })
    }

    /// Loads the device, rejecting `SyncDisabled` when sync has never been enabled.
    async fn require_device(&self) -> Result<SyncDevice, SyncError> {
        self.state_repo
            .get_device()
            .await?
            .ok_or(SyncError::SyncDisabled)
    }

    /// Keeps what a completed run left behind for `SyncStatus` (SYN-063/069).
    pub fn remember_run(&self, report: &SyncReport) {
        *self.last_run.lock().unwrap_or_else(PoisonError::into_inner) = Some(LastRun {
            completed_at: report.completed_at.clone(),
            failures: report.failures.clone(),
        });
    }

    fn last_run(&self) -> Option<LastRun> {
        self.last_run
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn forget_runs(&self) {
        *self.last_run.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }

    fn status_of(&self, device: &SyncDevice) -> SyncStatus {
        let last_run = self.last_run();
        SyncStatus::for_device(
            device,
            last_run.as_ref().map(|run| run.completed_at.clone()),
            last_run.map(|run| run.failures).unwrap_or_default(),
        )
    }

    /// Pauses sync on this device (SYN-070). `SyncDisabled` while never enabled;
    /// `AlreadyPaused` when already paused.
    pub async fn pause_sync(&self) -> Result<SyncStatus, SyncError> {
        let paused = self.require_device().await?.pause()?;
        self.state_repo.save_device(&paused).await?;
        Ok(self.status_of(&paused))
    }

    /// The precondition half of `resume_sync` (SYN-073): `SyncDisabled` while never enabled;
    /// `NotPaused` while not paused. Returns the loaded device so the use case can continue
    /// with publishing paused-era changes and running a sync.
    pub async fn resume_sync_precondition(&self) -> Result<SyncDevice, SyncError> {
        let device = self.require_device().await?;
        if !device.paused {
            return Err(SyncError::NotPaused);
        }
        Ok(device)
    }

    /// Renames this device (SYN-072). `DeviceNameBlank` on a blank name; the manifest is
    /// republished at the next sync, not here.
    pub async fn rename_sync_device(&self, device_name: String) -> Result<SyncStatus, SyncError> {
        let renamed = self.require_device().await?.rename(device_name)?;
        self.state_repo.save_device(&renamed).await?;
        Ok(self.status_of(&renamed))
    }

    /// Leaves sync on this device for good (SYN-082): publishes unpublished changes, removes
    /// its own manifest, drops device state (identity, cursors, held-back, notices, key), and
    /// leaves the local portfolio and its folder area untouched. Rejects `FolderUnavailable`
    /// so unpublished work is never abandoned. A device that has detected a reset (SYN-084)
    /// skips the folder entirely: nothing is ever written under the old key.
    pub async fn leave_sync(&self) -> Result<(), SyncError> {
        let device = self.require_device().await?;
        let reset_detected = self
            .last_run()
            .is_some_and(|run| run.failures.contains(&SyncFailure::PortfolioReset));
        if !reset_detected {
            self.folder_store.retarget(&device.folder);
            self.folder_store
                .check_available()
                .await
                .map_err(|problem| SyncError::FolderUnavailable { problem })?;
            let report = self.sync_run()?.publish(&device).await?;
            self.remember_run(&report);
            if let Some(problem) = report.failures.iter().find_map(|failure| match failure {
                SyncFailure::FolderUnavailable { problem } => Some(*problem),
                _ => None,
            }) {
                return Err(SyncError::PublishFailed { problem });
            }
            if !report.failures.contains(&SyncFailure::PortfolioReset) {
                self.folder_store.remove_manifest(&device.device_id).await?;
            }
        }
        self.state_repo.discard_device_state().await?;
        self.forget_runs();
        Ok(())
    }

    /// The automatic publish-only run a settled burst of recorded changes fires (SYN-067):
    /// publishes on an enabled, non-paused device and remembers the run. Never fails outward —
    /// a failed run is logged, and the next recorded change retries.
    pub async fn publish_recorded_changes(&self) {
        let device = match self.state_repo.get_device().await {
            Ok(Some(device)) if !device.paused => device,
            Ok(_) => return,
            Err(error) => {
                tracing::warn!(target: BACKEND, err = %error, "publish_recorded_changes: device not loaded");
                return;
            }
        };
        let Ok(sync_run) = self.sync_run() else {
            return;
        };
        match sync_run.publish(&device).await {
            Ok(report) => self.remember_run(&report),
            Err(error) => {
                tracing::warn!(target: BACKEND, err = %error, "publish_recorded_changes: run failed");
            }
        }
    }

    /// Dismisses a conflict notice (SYN-066). `NoticeNotFound` when it does not exist.
    pub async fn dismiss_conflict_notice(&self, notice_id: String) -> Result<(), SyncError> {
        let _device = self.require_device().await?;
        self.state_repo.dismiss_notice(&notice_id).await
    }

    /// Assembles the sync-owned half of `SyncStatus` (SYN-063). While disabled, `enabled` is
    /// `false`, the `Option` fields are `None`, and every collection is empty.
    pub async fn status(&self) -> Result<SyncStatus, SyncError> {
        Ok(match self.state_repo.get_device().await? {
            Some(device) => self.status_of(&device),
            None => SyncStatus::disabled(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::sync::domain::MockFolderStore;
    use crate::context::sync::domain::MockSyncStateRepository;

    fn paused_device() -> SyncDevice {
        SyncDevice::restore(
            "desktop-device".into(),
            "Desktop".into(),
            "/tmp/sync".into(),
            "2026-08-22T00:00:00Z".into(),
            true,
            "2026-08-22T00:00:00Z".into(),
            1,
        )
    }

    fn active_device() -> SyncDevice {
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

    // SYN-010 — pause_sync rejects SyncDisabled while sync has never been enabled.
    #[tokio::test]
    async fn pause_sync_rejects_when_disabled() {
        let mut state_repo = MockSyncStateRepository::new();
        state_repo.expect_get_device().returning(|| Ok(None));
        let service = SyncService::new(Arc::new(state_repo), Arc::new(MockFolderStore::new()));
        let result = service.pause_sync().await;
        assert!(matches!(result, Err(SyncError::SyncDisabled)));
    }

    // SYN-070 — pause_sync rejects AlreadyPaused on an already-paused device.
    #[tokio::test]
    async fn pause_sync_rejects_when_already_paused() {
        let mut state_repo = MockSyncStateRepository::new();
        state_repo
            .expect_get_device()
            .returning(|| Ok(Some(paused_device())));
        let service = SyncService::new(Arc::new(state_repo), Arc::new(MockFolderStore::new()));
        let result = service.pause_sync().await;
        assert!(matches!(result, Err(SyncError::AlreadyPaused)));
    }

    // SYN-070 — pause_sync saves the paused device and reports it paused.
    #[tokio::test]
    async fn pause_sync_saves_and_reports_the_paused_device() {
        let mut state_repo = MockSyncStateRepository::new();
        state_repo
            .expect_get_device()
            .returning(|| Ok(Some(active_device())));
        state_repo
            .expect_save_device()
            .withf(|device| device.paused)
            .times(1)
            .returning(|_| Ok(()));
        let service = SyncService::new(Arc::new(state_repo), Arc::new(MockFolderStore::new()));
        let status = service.pause_sync().await.unwrap();
        assert!(status.enabled);
        assert!(status.paused);
        assert_eq!(status.device_id.as_deref(), Some("desktop-device"));
    }

    // SYN-010 — resume_sync_precondition rejects SyncDisabled while never enabled.
    #[tokio::test]
    async fn resume_sync_precondition_rejects_when_disabled() {
        let mut state_repo = MockSyncStateRepository::new();
        state_repo.expect_get_device().returning(|| Ok(None));
        let service = SyncService::new(Arc::new(state_repo), Arc::new(MockFolderStore::new()));
        let result = service.resume_sync_precondition().await;
        assert!(matches!(result, Err(SyncError::SyncDisabled)));
    }

    // SYN-073 — resume_sync_precondition rejects NotPaused on an active device.
    #[tokio::test]
    async fn resume_sync_precondition_rejects_when_not_paused() {
        let mut state_repo = MockSyncStateRepository::new();
        state_repo
            .expect_get_device()
            .returning(|| Ok(Some(active_device())));
        let service = SyncService::new(Arc::new(state_repo), Arc::new(MockFolderStore::new()));
        let result = service.resume_sync_precondition().await;
        assert!(matches!(result, Err(SyncError::NotPaused)));
    }

    // SYN-018/072 — rename_sync_device rejects a blank name.
    #[tokio::test]
    async fn rename_sync_device_rejects_blank_name() {
        let mut state_repo = MockSyncStateRepository::new();
        state_repo
            .expect_get_device()
            .returning(|| Ok(Some(active_device())));
        let service = SyncService::new(Arc::new(state_repo), Arc::new(MockFolderStore::new()));
        let result = service.rename_sync_device("   ".into()).await;
        assert!(matches!(result, Err(SyncError::DeviceNameBlank)));
    }

    // SYN-010 — rename_sync_device rejects SyncDisabled while never enabled.
    #[tokio::test]
    async fn rename_sync_device_rejects_when_disabled() {
        let mut state_repo = MockSyncStateRepository::new();
        state_repo.expect_get_device().returning(|| Ok(None));
        let service = SyncService::new(Arc::new(state_repo), Arc::new(MockFolderStore::new()));
        let result = service.rename_sync_device("Laptop".into()).await;
        assert!(matches!(result, Err(SyncError::SyncDisabled)));
    }

    // SYN-072 — rename_sync_device saves the new name and reports it.
    #[tokio::test]
    async fn rename_sync_device_saves_and_reports_the_new_name() {
        let mut state_repo = MockSyncStateRepository::new();
        state_repo
            .expect_get_device()
            .returning(|| Ok(Some(active_device())));
        state_repo
            .expect_save_device()
            .withf(|device| device.device_name == "Laptop")
            .times(1)
            .returning(|_| Ok(()));
        let service = SyncService::new(Arc::new(state_repo), Arc::new(MockFolderStore::new()));
        let status = service.rename_sync_device("Laptop".into()).await.unwrap();
        assert_eq!(status.device_name.as_deref(), Some("Laptop"));
    }

    // SYN-010 — leave_sync rejects SyncDisabled while never enabled.
    #[tokio::test]
    async fn leave_sync_rejects_when_disabled() {
        let mut state_repo = MockSyncStateRepository::new();
        state_repo.expect_get_device().returning(|| Ok(None));
        let service = SyncService::new(Arc::new(state_repo), Arc::new(MockFolderStore::new()));
        let result = service.leave_sync().await;
        assert!(matches!(result, Err(SyncError::SyncDisabled)));
    }

    // SYN-082/069 — leave_sync rejects FolderUnavailable before touching anything, so
    // unpublished work is never abandoned.
    #[tokio::test]
    async fn leave_sync_rejects_folder_unavailable_and_keeps_the_device() {
        let mut state_repo = MockSyncStateRepository::new();
        state_repo
            .expect_get_device()
            .returning(|| Ok(Some(active_device())));
        state_repo.expect_discard_device_state().times(0);
        let mut folder_store = MockFolderStore::new();
        folder_store.expect_retarget().return_const(());
        folder_store
            .expect_check_available()
            .returning(|| Err(crate::context::sync::domain::FolderProblem::Unmounted));
        let service = SyncService::new(Arc::new(state_repo), Arc::new(folder_store));
        let result = service.leave_sync().await;
        assert!(matches!(
            result,
            Err(SyncError::FolderUnavailable {
                problem: crate::context::sync::domain::FolderProblem::Unmounted
            })
        ));
    }

    // SYN-067/070 — the automatic publish never touches the folder while the device is paused
    // (the folder store has no expectations: any call would panic).
    #[tokio::test]
    async fn publish_recorded_changes_does_nothing_while_paused() {
        let mut state_repo = MockSyncStateRepository::new();
        state_repo
            .expect_get_device()
            .returning(|| Ok(Some(paused_device())));
        let service = SyncService::new(Arc::new(state_repo), Arc::new(MockFolderStore::new()));
        service.publish_recorded_changes().await;
        assert!(service.last_run().is_none());
    }

    // SYN-066 — dismiss_conflict_notice surfaces NoticeNotFound for an unknown id.
    #[tokio::test]
    async fn dismiss_conflict_notice_surfaces_notice_not_found() {
        let mut state_repo = MockSyncStateRepository::new();
        state_repo
            .expect_get_device()
            .returning(|| Ok(Some(active_device())));
        state_repo.expect_dismiss_notice().returning(|notice_id| {
            Err(SyncError::NoticeNotFound {
                notice_id: notice_id.to_string(),
            })
        });
        let service = SyncService::new(Arc::new(state_repo), Arc::new(MockFolderStore::new()));
        let result = service
            .dismiss_conflict_notice("does-not-exist".into())
            .await;
        assert!(matches!(
            result,
            Err(SyncError::NoticeNotFound { notice_id }) if notice_id == "does-not-exist"
        ));
    }

    // SYN-063 — status() returns the disabled shape: enabled=false, Option fields None,
    // collections empty.
    #[tokio::test]
    async fn status_returns_disabled_shape_when_never_enabled() {
        let mut state_repo = MockSyncStateRepository::new();
        state_repo.expect_get_device().returning(|| Ok(None));
        let service = SyncService::new(Arc::new(state_repo), Arc::new(MockFolderStore::new()));
        let status = service
            .status()
            .await
            .expect("status must not error while disabled");
        assert!(!status.enabled);
        assert!(status.device_id.is_none());
        assert!(status.device_name.is_none());
        assert!(status.folder.is_none());
        assert!(status.roster.is_empty());
        assert!(status.notices.is_empty());
        assert!(status.failures.is_empty());
        assert_eq!(status.held_back_count, 0);
    }

    // SYN-063/069 — status() carries the last remembered run's completion time and failures.
    #[tokio::test]
    async fn status_reports_the_last_remembered_run() {
        let mut state_repo = MockSyncStateRepository::new();
        state_repo
            .expect_get_device()
            .returning(|| Ok(Some(active_device())));
        let service = SyncService::new(Arc::new(state_repo), Arc::new(MockFolderStore::new()));
        let failures = vec![SyncFailure::FolderUnavailable {
            problem: crate::context::sync::domain::FolderProblem::Missing,
        }];
        service.remember_run(&SyncReport {
            published_changes: 0,
            applied_changes: 0,
            held_back_changes: 0,
            dropped_changes: 0,
            notices_raised: 0,
            failures: failures.clone(),
            completed_at: "2026-08-22T10:00:00Z".into(),
            status: SyncStatus::for_device(&active_device(), None, vec![]),
        });

        let status = service.status().await.unwrap();
        assert!(status.enabled);
        assert_eq!(
            status.last_sync_completed_at.as_deref(),
            Some("2026-08-22T10:00:00Z")
        );
        assert_eq!(status.failures, failures);
    }
}
