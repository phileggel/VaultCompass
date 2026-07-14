//! Shared infrastructure adapters reused by multiple bounded contexts.

/// Composition root wiring repositories into application services.
pub mod container;
/// Outbound HTTP response helpers.
pub mod http;
/// Daily fetch scheduler abstraction + platform adapters (SPF-012, SPF-017).
pub mod scheduler;
