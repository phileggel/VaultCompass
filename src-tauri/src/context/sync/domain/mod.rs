//! Domain layer of the sync bounded context (B0/B38). The resolution engine
//! (`resolution.rs`) lands in PR-C; PR-B ships the device aggregate, the folder/manifest/
//! segment value objects, and the wire shapes assembled into `SyncStatus` (D2).

/// The `ChangeLogRepository` port — the change log and the enrolment-owned device state.
pub mod change_log;
/// Undismissed-notice persistence shape (SYN-066, CFR-060).
pub mod conflict_notice;
/// How far this device has taken in another device's changes (SYN-033).
pub mod cursor;
/// `SyncDevice` aggregate + the `SyncStateRepository` port (SYN-016/018/070/072/084).
pub mod device;
/// Folder/header/manifest/segment value objects + the `FolderStore` port (D2/D8).
pub mod folder;
/// A received change waiting for a record this device has not received yet (SYN-041).
pub mod held_back;
/// The `RankStamper` port — ranks the rows that existed before sync did (CFR-014, D6).
pub mod rank_stamper;
/// The `PortfolioSnapshot` port — the whole current portfolio as `Created` changes (SYN-013).
pub mod snapshot;
/// `SyncStatus` / `SyncReport` wire shapes (SYN-063).
pub mod status;
/// What a removal leaves behind (CFR-015).
pub mod tombstone;

pub use change_log::ChangeLogRepository;
pub use conflict_notice::{ConflictNotice, ConflictNoticeKind};
pub use cursor::SyncCursor;
pub use device::{ensure_device_name, SyncDevice, SyncStateRepository};
pub use folder::{
    DerivationParameters, FolderHeader, FolderProblem, FolderStore, Manifest, Segment,
    SegmentChange, SyncFolderState, WriteHeaderOutcome,
};
pub use held_back::HeldBackChange;
pub use rank_stamper::RankStamper;
pub use snapshot::{PortfolioRecord, PortfolioSnapshot};
pub use status::{
    HoldingInconsistency, InconsistentHolding, RosterEntry, SyncFailure, SyncReport, SyncStatus,
};
pub use tombstone::Tombstone;

#[cfg(test)]
pub use device::MockSyncStateRepository;
#[cfg(test)]
pub use folder::MockFolderStore;
#[cfg(test)]
pub use rank_stamper::MockRankStamper;
#[cfg(test)]
pub use snapshot::MockPortfolioSnapshot;
