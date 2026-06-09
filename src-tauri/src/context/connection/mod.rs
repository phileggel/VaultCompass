/// External API and Tauri command handlers (boundary, BC root per B39).
pub mod api;
/// Application layer (the `ConnectionService` orchestrator).
pub mod application;
/// Core business entities and ports (`KeyStore`, `ConnectionProbe`).
pub mod domain;
/// Flat BC error enum (`ConnectionError`).
pub mod error;
/// Adapters: OS-keychain ladder + live Stooq probe.
pub mod infrastructure;

// Glob re-export mirrors the currency BC: `collect_commands!` in
// specta_builder.rs resolves each command via `connection::<cmd>`, which needs the
// `#[specta::specta]`-generated companion items re-exported alongside the fns.
pub use api::*;
pub use application::ConnectionService;
pub use domain::{
    ConnectionProbe, KeyStore, Provider, ProviderConnection, ProviderKeyTestOutcome, StorageTier,
};
pub use error::ConnectionError;
pub use infrastructure::{LayeredKeyStore, StooqProbe};

// Re-exported at the BC root so external test code reaches the port mocks through
// `connection::` rather than `connection::domain::` (B14).
#[cfg(test)]
pub use domain::{MockConnectionProbe, MockKeyStore};
