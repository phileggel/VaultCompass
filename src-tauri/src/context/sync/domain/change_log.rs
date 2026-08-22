//! The `ChangeLogRepository` port: this device's own change log (`changes`) and the
//! enrolment-owned half of the `sync_device` singleton — the kept key (SYN-052) and the
//! logical clock (CFR-010). The enrolment methods take the live connection so the device row,
//! the first segment's changes, and every rank stamp commit together or not at all (SYN-013),
//! the way `ChangeRecorder` rides the write it describes.

use sqlx::{Sqlite, SqliteConnection, Transaction};

use crate::context::sync::domain::device::SyncDevice;
use crate::context::sync::domain::folder::SegmentChange;
use crate::context::sync::error::SyncError;

/// Persistence for the change log and the enrolment-owned device state.
#[async_trait::async_trait]
pub trait ChangeLogRepository: Send + Sync {
    /// The raw bytes of the kept key (SYN-052), or `None` while sync has never been enabled.
    async fn kept_key_bytes(&self) -> Result<Option<Vec<u8>>, SyncError>;

    /// The device's logical clock (CFR-010); 0 while sync has never been enabled.
    async fn logical_clock(&self) -> Result<i64, SyncError>;

    /// This device's unpublished changes, in sequence order (SYN-060).
    async fn list_unpublished(&self, device_id: &str) -> Result<Vec<SegmentChange>, SyncError>;

    /// Marks `first_sequence..=last_sequence` published once their segment is in the folder
    /// (SYN-031/067).
    async fn mark_published(
        &self,
        device_id: &str,
        first_sequence: i64,
        last_sequence: i64,
    ) -> Result<(), SyncError>;

    /// The highest published sequence — the manifest's `latest_sequence` (SYN-037); 0 when
    /// nothing has been published.
    async fn latest_published_sequence(&self, device_id: &str) -> Result<i64, SyncError>;

    /// Opens the enrolment transaction (SYN-013).
    async fn begin(&self) -> Result<Transaction<'static, Sqlite>, SyncError>;

    /// Writes the singleton device row with its kept key and logical clock, on `conn`.
    async fn save_enrolment(
        &self,
        conn: &mut SqliteConnection,
        device: &SyncDevice,
        derived_key: &[u8],
        logical_clock: i64,
    ) -> Result<(), SyncError>;

    /// Marks every change `device_id` recorded before this enrolment published, on `conn` —
    /// the first segment replays the whole portfolio instead (SYN-026).
    async fn retire_earlier_changes(
        &self,
        conn: &mut SqliteConnection,
        device_id: &str,
    ) -> Result<(), SyncError>;

    /// The next free sequence for `device_id` (SYN-025), on `conn`.
    async fn next_sequence(
        &self,
        conn: &mut SqliteConnection,
        device_id: &str,
    ) -> Result<i64, SyncError>;

    /// Appends one change already carried by a written segment, on `conn`.
    async fn append_published_change(
        &self,
        conn: &mut SqliteConnection,
        device_id: &str,
        change: &SegmentChange,
    ) -> Result<(), SyncError>;
}
