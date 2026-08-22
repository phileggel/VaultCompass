//! `Tombstone` (CFR-015): what a removal leaves behind. Read-side value object mirroring the
//! `tombstones` table PR-A already writes via `SqliteChangeRecorder`; PR-B does not read
//! tombstones yet (that starts with PR-C's resolution engine) but the shape is declared here
//! so the sync domain module is complete per D2's file list.

use crate::shared::domain::{LogicalTimestamp, Origin, RecordKind};

/// What a removal leaves behind (CFR-015): stands in for the removed record when a later or
/// earlier change to it arrives. Permanent — never pruned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tombstone {
    /// The kind of the removed record.
    pub record_kind: RecordKind,
    /// Its identity (CFR-012).
    pub record_identity: String,
    /// The removal's logical timestamp.
    pub logical_timestamp: LogicalTimestamp,
    /// Whether the user or the application removed it (CFR-016).
    pub origin: Origin,
    /// The device that removed it, named in conflict notices.
    pub removed_by: String,
}
