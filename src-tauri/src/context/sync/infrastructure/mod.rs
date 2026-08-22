//! Infrastructure layer of the sync bounded context (B0/B38, flat-first per B41).

/// `SqliteChangeRecorder` — the SQLite-backed `ChangeRecorder` (PR-A, D1).
pub mod change_log;
