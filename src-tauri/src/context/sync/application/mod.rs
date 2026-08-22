//! Application layer of the sync bounded context (B0/B38). `resolution.rs`'s consumer (the
//! full apply executor) lands in PR-C — PR-B ships publish-only.

/// Enrolling as the first device of a new shared portfolio (SYN-013/026/081).
pub mod first_publish;
/// Settling-interval batching, 5s / 30s cap (SYN-067).
pub mod publisher;
/// One sync run — publish-only in PR-B (SYN-060/061/067/069).
pub mod run;
/// `SyncService` — device lifecycle, status assembly, notice dismissal.
pub mod service;

pub use first_publish::FirstPublish;
pub use publisher::Publisher;
pub use run::SyncRun;
pub use service::SyncService;
