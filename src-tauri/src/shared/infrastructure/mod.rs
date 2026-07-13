//! Shared infrastructure adapters reused by multiple bounded contexts.

/// Outbound HTTP response helpers.
pub mod http;
/// Daily fetch scheduler abstraction + platform adapters (SPF-012, SPF-017).
pub mod scheduler;
