//! Multi-device sync bounded context (SYN + CFR, ADR-019). PR-A shipped only the change-log
//! slice; PR-B adds the device aggregate, folder/crypto/codec infrastructure, the publish-only
//! run, and the first-device publish path (D2). The resolution engine and the apply executor
//! land in PR-C.

/// External API and Tauri command handlers (boundary, BC root per B39) — the four BC-local
/// commands (D3): `pause_sync`, `leave_sync`, `rename_sync_device`, `dismiss_conflict_notice`.
pub mod api;
/// Application layer (gold layout, B0/B38): device lifecycle, the publish-only run, the
/// settling-interval batcher, and enrolling as the first device.
pub mod application;
/// Domain layer (gold layout, B0/B38): the device aggregate, folder/manifest/segment value
/// objects, and the wire shapes assembled into `SyncStatus`. The resolution engine
/// (`resolution.rs`) lands in PR-C.
pub mod domain;
/// Flat BC error enum (`SyncError`).
pub mod error;
/// Infrastructure layer — `SqliteChangeRecorder` (PR-A); crypto, codec, folder store, and the
/// SQLite-backed sync state repository (PR-B).
pub mod infrastructure;

pub use api::*;
pub use application::{FirstPublish, Publisher, SyncRun, SyncService};
pub use domain::{
    ensure_device_name, ChangeLogRepository, ConflictNotice, ConflictNoticeKind,
    DerivationParameters, FolderHeader, FolderProblem, FolderStore, HeldBackChange,
    HoldingInconsistency, InconsistentHolding, Manifest, PortfolioRecord, PortfolioSnapshot,
    RankStamper, RosterEntry, Segment, SegmentChange, SyncCursor, SyncDevice, SyncFailure,
    SyncFolderState, SyncReport, SyncStateRepository, SyncStatus, Tombstone, WriteHeaderOutcome,
};
pub use error::SyncError;
pub use infrastructure::codec::{header_data_format_version, DATA_FORMAT_VERSION};
pub use infrastructure::crypto::ensure_passphrase_length;
pub use infrastructure::{
    FsFolderStore, RecordedChangeHook, SqliteChangeLogRepository, SqliteChangeRecorder,
    SqliteSyncStateRepository,
};

#[cfg(test)]
pub use domain::{
    MockFolderStore, MockPortfolioSnapshot, MockRankStamper, MockSyncStateRepository,
};
