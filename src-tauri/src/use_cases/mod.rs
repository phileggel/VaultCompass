//! Application use cases layer.
//!
//! Cross-cutting application use cases that orchestrate multiple bounded
//! contexts or platform capabilities.

/// Application auto-update: detection, download, and installation (R1–R27).
pub mod update_checker;
