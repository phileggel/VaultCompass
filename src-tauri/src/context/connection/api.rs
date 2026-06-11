// The `#[tauri::command]` expansion generates an `unreachable!` arm that the
// crate-level `deny(clippy::unreachable)` would reject; allow it at this boundary
// module, consistent with the other BCs' `api.rs`.
#![allow(clippy::unreachable)]

use crate::context::connection::application::ConnectionService;
use crate::context::connection::domain::{Provider, ProviderConnection, ProviderKeyTestOutcome};
use crate::context::connection::error::ConnectionError;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fmt;
use std::result::Result as StdResult;
use std::sync::Arc;

/// Arguments for `save_provider_key` (KEY-010/012).
///
/// `Debug` is implemented manually to redact the secret `key` (KEY-014): the
/// derived form would render it verbatim in any `{:?}`.
#[derive(Clone, Serialize, Deserialize, Type)]
pub struct SaveProviderKeyArgs {
    /// Which provider the key authenticates.
    pub provider: Provider,
    /// The pasted secret; write-only from the UI's perspective (KEY-018).
    pub key: String,
    /// Tier-3 opt-in (KEY-012); `false` keeps the key off disk on a keyring-less host.
    pub allow_plaintext: bool,
}

impl fmt::Debug for SaveProviderKeyArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SaveProviderKeyArgs")
            .field("provider", &self.provider)
            .field("key", &"[REDACTED]")
            .field("allow_plaintext", &self.allow_plaintext)
            .finish()
    }
}

/// Arguments for `test_provider_key` (KEY-021). `Debug` redacts the secret `key`
/// (KEY-014).
#[derive(Clone, Serialize, Deserialize, Type)]
pub struct TestProviderKeyArgs {
    /// Which provider to probe.
    pub provider: Provider,
    /// The candidate key to test (not necessarily the stored one).
    pub key: String,
}

impl fmt::Debug for TestProviderKeyArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestProviderKeyArgs")
            .field("provider", &self.provider)
            .field("key", &"[REDACTED]")
            .finish()
    }
}

/// Arguments for `remove_provider_key` (KEY-013).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RemoveProviderKeyArgs {
    /// Which provider's key to clear.
    pub provider: Provider,
}

/// Lists every supported provider with its `has_key` / `active_tier` (KEY-016).
#[tauri::command]
#[specta::specta]
pub async fn get_provider_connections(
    svc: tauri::State<'_, Arc<ConnectionService>>,
) -> StdResult<Vec<ProviderConnection>, ConnectionError> {
    svc.connections().await
}

/// Persists a provider key via the tier ladder (KEY-010/011/012). Returns the
/// resulting `ProviderConnection` so the FE learns the `active_tier`.
#[tauri::command]
#[specta::specta]
pub async fn save_provider_key(
    svc: tauri::State<'_, Arc<ConnectionService>>,
    args: SaveProviderKeyArgs,
) -> StdResult<ProviderConnection, ConnectionError> {
    svc.save_key(args.provider, args.key, args.allow_plaintext)
        .await
}

/// Probes a provider with the supplied candidate key (KEY-021/022). The three
/// outcomes are successful returns, not errors (KEY-023).
#[tauri::command]
#[specta::specta]
pub async fn test_provider_key(
    svc: tauri::State<'_, Arc<ConnectionService>>,
    args: TestProviderKeyArgs,
) -> StdResult<ProviderKeyTestOutcome, ConnectionError> {
    svc.test_key(args.provider, args.key).await
}

/// Clears a provider's key from every tier (KEY-013). Idempotent.
#[tauri::command]
#[specta::specta]
pub async fn remove_provider_key(
    svc: tauri::State<'_, Arc<ConnectionService>>,
    args: RemoveProviderKeyArgs,
) -> StdResult<(), ConnectionError> {
    svc.remove_key(args.provider).await
}

#[cfg(test)]
mod tests {
    use super::*;

    // KEY-014 — `{:?}` on the args carrying a secret renders `[REDACTED]`, never
    // the key. Guards against a regression to `#[derive(Debug)]`.
    #[test]
    fn save_args_debug_redacts_the_key() {
        let args = SaveProviderKeyArgs {
            provider: Provider::Stooq,
            key: "super-secret-key".to_string(),
            allow_plaintext: false,
        };
        let rendered = format!("{args:?}");
        assert!(rendered.contains("[REDACTED]"));
        assert!(!rendered.contains("super-secret-key"));
    }

    // KEY-014 — same redaction guarantee for the test-probe args.
    #[test]
    fn test_args_debug_redacts_the_key() {
        let args = TestProviderKeyArgs {
            provider: Provider::Stooq,
            key: "super-secret-key".to_string(),
        };
        let rendered = format!("{args:?}");
        assert!(rendered.contains("[REDACTED]"));
        assert!(!rendered.contains("super-secret-key"));
    }
}
