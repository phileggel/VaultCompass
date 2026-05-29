//! Account Performance use case: cross-context recompute-on-read of per-period
//! values and Simple Dietz performance metrics (PRF spec, ADR-013).

mod api;
mod orchestrator;

pub use api::*;
pub use orchestrator::*;
