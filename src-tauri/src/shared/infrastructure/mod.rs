//! Shared infrastructure adapters reused by multiple bounded contexts.

/// Outbound HTTP response helpers.
pub mod http;
/// Stooq proof-of-work gate, shared by the asset price fetcher and the
/// connection key-probe.
pub mod stooq;
