//! Multi-device sync bounded context (SYN + CFR, ADR-019). PR-A ships only the change-log
//! slice (`infrastructure::change_log::SqliteChangeRecorder`) — device lifecycle, the
//! resolution engine, and the apply executor land in PR-B/PR-C.

/// Application layer (gold layout, B0/B38) — empty in PR-A; `service.rs`/`run.rs`/
/// `publisher.rs` land in PR-B/PR-C.
pub mod application;
/// Domain layer (gold layout, B0/B38) — empty in PR-A; `resolution.rs`, `device.rs`, and the
/// rest of D2's module list land in PR-B/PR-C.
pub mod domain;
/// Infrastructure layer — `SqliteChangeRecorder` (PR-A); crypto/codec/folder_store/device
/// land in PR-B.
pub mod infrastructure;

pub use infrastructure::change_log::SqliteChangeRecorder;
