use crate::context::connection::domain::{
    ConnectionProbe, KeyStore, Provider, ProviderConnection, ProviderKeyTestOutcome,
};
use crate::context::connection::error::ConnectionError;
use crate::core::logger::BACKEND;
use std::result::Result as StdResult;

/// Orchestrates provider-key management for the `connection` bounded context
/// (KEY-010/013/016/021/022). Injects the storage-tier ladder (`KeyStore`) and the
/// live probe (`ConnectionProbe`) as ports.
pub struct ConnectionService {
    key_store: Box<dyn KeyStore>,
    probe: Box<dyn ConnectionProbe>,
}

impl ConnectionService {
    /// Creates a new ConnectionService with the given ports.
    pub fn new(key_store: Box<dyn KeyStore>, probe: Box<dyn ConnectionProbe>) -> Self {
        Self { key_store, probe }
    }

    /// Lists every supported provider with its `has_key` / `active_tier` (KEY-016).
    /// A key-store read fault surfaces `KeyStoreError`, never `has_key = false`.
    pub async fn connections(&self) -> StdResult<Vec<ProviderConnection>, ConnectionError> {
        // Only Stooq is supported in this slice (KEY-031); the list shape scales
        // to further providers without a signature change.
        let provider = Provider::Stooq;
        let active_tier = self.key_store.locate(provider).await.map_err(|error| {
            tracing::error!(target: BACKEND, ?provider, ?error, "connection: key locate failed");
            ConnectionError::KeyStoreError
        })?;
        Ok(vec![ProviderConnection {
            provider,
            has_key: active_tier.is_some(),
            active_tier,
        }])
    }

    /// Persists a key via the tier ladder (KEY-010/011/012). Rejects a blank /
    /// whitespace key with `EmptyKey` (KEY-010). On overwrite, clears every tier
    /// first then stores (KEY-013). Returns the resulting `ProviderConnection`.
    pub async fn save_key(
        &self,
        provider: Provider,
        key: String,
        allow_plaintext: bool,
    ) -> StdResult<ProviderConnection, ConnectionError> {
        if key.trim().is_empty() {
            return Err(ConnectionError::EmptyKey);
        }
        // KEY-013 — clear every tier first so no stale key survives in a lower one.
        self.key_store.clear(provider).await.map_err(|error| {
            tracing::error!(target: BACKEND, ?provider, ?error, "connection: pre-store clear failed");
            ConnectionError::KeyStoreError
        })?;
        let active_tier = self
            .key_store
            .store(provider, &key, allow_plaintext)
            .await
            .map_err(|error| {
                tracing::error!(target: BACKEND, ?provider, ?error, "connection: key store failed");
                ConnectionError::KeyStoreError
            })?;
        Ok(ProviderConnection {
            provider,
            has_key: true,
            active_tier: Some(active_tier),
        })
    }

    /// Probes the provider with the supplied (not-necessarily-saved) value
    /// (KEY-021/022). The three outcomes are successful returns, not errors.
    /// Rejects a blank key with `EmptyKey`. Read-only wrt stored state.
    pub async fn test_key(
        &self,
        provider: Provider,
        key: String,
    ) -> StdResult<ProviderKeyTestOutcome, ConnectionError> {
        if key.trim().is_empty() {
            return Err(ConnectionError::EmptyKey);
        }
        self.probe.probe(provider, &key).await.map_err(|error| {
            tracing::error!(target: BACKEND, ?provider, ?error, "connection: key probe failed");
            ConnectionError::KeyStoreError
        })
    }

    /// Clears the key from every tier (KEY-013). Idempotent — removing when no
    /// key exists succeeds.
    pub async fn remove_key(&self, provider: Provider) -> StdResult<(), ConnectionError> {
        self.key_store.clear(provider).await.map_err(|error| {
            tracing::error!(target: BACKEND, ?provider, ?error, "connection: key clear failed");
            ConnectionError::KeyStoreError
        })
    }

    /// Backend-internal: reads the stored key for the fetch path (KEY-018). Never
    /// exposed as a command — the value never reaches the wire.
    pub async fn resolve_key(
        &self,
        provider: Provider,
    ) -> StdResult<Option<String>, ConnectionError> {
        self.key_store.read(provider).await.map_err(|error| {
            tracing::error!(target: BACKEND, ?provider, ?error, "connection: key resolve failed");
            ConnectionError::KeyStoreError
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::connection::domain::{
        MockConnectionProbe, MockKeyStore, Provider, ProviderKeyTestOutcome, StorageTier,
    };

    // ---- get_provider_connections (KEY-016) ----

    // KEY-016 — connections reports has_key=true with the locating tier when a key
    // is stored, never reading the secret value.
    #[tokio::test]
    async fn connections_reports_has_key_and_active_tier() {
        let mut key_store = MockKeyStore::new();
        key_store
            .expect_locate()
            .returning(|_| Ok(Some(StorageTier::OsKeychain)));
        let probe = MockConnectionProbe::new();
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        let connections = service.connections().await.expect("connections ok");
        let stooq = connections
            .iter()
            .find(|c| c.provider == Provider::Stooq)
            .expect("Stooq row present");
        assert!(stooq.has_key, "a located key must report has_key=true");
        assert_eq!(stooq.active_tier, Some(StorageTier::OsKeychain));
    }

    // KEY-016 — when no key is stored, connections reports has_key=false and a
    // None active_tier.
    #[tokio::test]
    async fn connections_reports_no_key_when_none_stored() {
        let mut key_store = MockKeyStore::new();
        key_store.expect_locate().returning(|_| Ok(None));
        let probe = MockConnectionProbe::new();
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        let connections = service.connections().await.expect("connections ok");
        let stooq = connections
            .iter()
            .find(|c| c.provider == Provider::Stooq)
            .expect("Stooq row present");
        assert!(!stooq.has_key);
        assert_eq!(stooq.active_tier, None);
    }

    // KEY-016 — a key-store read fault surfaces KeyStoreError, NOT has_key=false.
    // A read fault must never be silently mistaken for "no key".
    #[tokio::test]
    async fn connections_surfaces_key_store_error_not_no_key() {
        let mut key_store = MockKeyStore::new();
        key_store
            .expect_locate()
            .returning(|_| Err(anyhow::anyhow!("keychain query failed")));
        let probe = MockConnectionProbe::new();
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        let error = service
            .connections()
            .await
            .expect_err("read fault must surface an error");
        assert!(
            matches!(error, ConnectionError::KeyStoreError),
            "read fault must map to KeyStoreError, got: {error:?}"
        );
    }

    // ---- save_provider_key (KEY-010/011/012/013) ----

    // KEY-010 — a blank key is rejected with EmptyKey and nothing is stored.
    #[tokio::test]
    async fn save_key_rejects_blank_key() {
        let mut key_store = MockKeyStore::new();
        key_store.expect_store().never();
        key_store.expect_clear().never();
        let probe = MockConnectionProbe::new();
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        let error = service
            .save_key(Provider::Stooq, "".to_string(), false)
            .await
            .expect_err("blank key must be rejected");
        assert!(matches!(error, ConnectionError::EmptyKey), "got: {error:?}");
    }

    // KEY-010 — a whitespace-only key is rejected with EmptyKey.
    #[tokio::test]
    async fn save_key_rejects_whitespace_key() {
        let mut key_store = MockKeyStore::new();
        key_store.expect_clear().never();
        key_store.expect_store().never();
        let probe = MockConnectionProbe::new();
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        let error = service
            .save_key(Provider::Stooq, "   \t  ".to_string(), false)
            .await
            .expect_err("whitespace key must be rejected");
        assert!(matches!(error, ConnectionError::EmptyKey), "got: {error:?}");
    }

    // KEY-013 — overwrite clears every tier first, then stores the new value;
    // the returned connection reports the resulting active_tier.
    #[tokio::test]
    async fn save_key_clears_all_tiers_then_stores() {
        let mut key_store = MockKeyStore::new();
        key_store.expect_clear().times(1).returning(|_| Ok(()));
        key_store
            .expect_store()
            .times(1)
            .returning(|_, _, _| Ok(StorageTier::SessionMemory));
        let probe = MockConnectionProbe::new();
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        let connection = service
            .save_key(Provider::Stooq, "valid-key".to_string(), false)
            .await
            .expect("save ok");
        assert!(connection.has_key);
        assert_eq!(connection.active_tier, Some(StorageTier::SessionMemory));
    }

    // KEY-011 — a store failure on the selected tier surfaces KeyStoreError rather
    // than silently losing the key.
    #[tokio::test]
    async fn save_key_surfaces_store_failure_as_key_store_error() {
        let mut key_store = MockKeyStore::new();
        key_store.expect_clear().returning(|_| Ok(()));
        key_store
            .expect_store()
            .returning(|_, _, _| Err(anyhow::anyhow!("keychain write failed")));
        let probe = MockConnectionProbe::new();
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        let error = service
            .save_key(Provider::Stooq, "valid-key".to_string(), false)
            .await
            .expect_err("store failure must surface an error");
        assert!(
            matches!(error, ConnectionError::KeyStoreError),
            "got: {error:?}"
        );
    }

    // ---- test_provider_key (KEY-021/022) ----

    // KEY-021/023 — Accepted is a successful RETURN variant, not an error.
    #[tokio::test]
    async fn test_key_returns_accepted_outcome() {
        let key_store = MockKeyStore::new();
        let mut probe = MockConnectionProbe::new();
        probe
            .expect_probe()
            .returning(|_, _| Ok(ProviderKeyTestOutcome::Accepted));
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        let outcome = service
            .test_key(Provider::Stooq, "candidate-key".to_string())
            .await
            .expect("test ok");
        assert_eq!(outcome, ProviderKeyTestOutcome::Accepted);
    }

    // KEY-023 — Rejected is a successful RETURN variant, not an error.
    #[tokio::test]
    async fn test_key_returns_rejected_outcome() {
        let key_store = MockKeyStore::new();
        let mut probe = MockConnectionProbe::new();
        probe
            .expect_probe()
            .returning(|_, _| Ok(ProviderKeyTestOutcome::Rejected));
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        let outcome = service
            .test_key(Provider::Stooq, "bad-key".to_string())
            .await
            .expect("test ok");
        assert_eq!(outcome, ProviderKeyTestOutcome::Rejected);
    }

    // KEY-023 — Unreachable is a successful RETURN variant, not an error.
    #[tokio::test]
    async fn test_key_returns_unreachable_outcome() {
        let key_store = MockKeyStore::new();
        let mut probe = MockConnectionProbe::new();
        probe
            .expect_probe()
            .returning(|_, _| Ok(ProviderKeyTestOutcome::Unreachable));
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        let outcome = service
            .test_key(Provider::Stooq, "any-key".to_string())
            .await
            .expect("test ok");
        assert_eq!(outcome, ProviderKeyTestOutcome::Unreachable);
    }

    // KEY-021 — a blank key is rejected with EmptyKey and the provider is never
    // probed (there is no value to test).
    #[tokio::test]
    async fn test_key_rejects_blank_key_without_probing() {
        let key_store = MockKeyStore::new();
        let mut probe = MockConnectionProbe::new();
        probe.expect_probe().never();
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        let error = service
            .test_key(Provider::Stooq, "   ".to_string())
            .await
            .expect_err("blank key must be rejected");
        assert!(matches!(error, ConnectionError::EmptyKey), "got: {error:?}");
    }

    // KEY-022 — testing is read-only wrt stored state: it neither stores, clears,
    // nor reads the persisted key.
    #[tokio::test]
    async fn test_key_does_not_touch_stored_state() {
        let mut key_store = MockKeyStore::new();
        key_store.expect_store().never();
        key_store.expect_clear().never();
        key_store.expect_read().never();
        let mut probe = MockConnectionProbe::new();
        probe
            .expect_probe()
            .returning(|_, _| Ok(ProviderKeyTestOutcome::Accepted));
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        let outcome = service
            .test_key(Provider::Stooq, "candidate-key".to_string())
            .await
            .expect("test ok");
        assert_eq!(outcome, ProviderKeyTestOutcome::Accepted);
    }

    // ---- remove_provider_key (KEY-013) ----

    // KEY-013 — remove clears every tier and returns Ok(()).
    #[tokio::test]
    async fn remove_key_clears_all_tiers() {
        let mut key_store = MockKeyStore::new();
        key_store.expect_clear().times(1).returning(|_| Ok(()));
        let probe = MockConnectionProbe::new();
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        service
            .remove_key(Provider::Stooq)
            .await
            .expect("remove ok");
    }

    // KEY-013 — remove is idempotent: clearing when no key is stored succeeds.
    #[tokio::test]
    async fn remove_key_is_idempotent_when_none_stored() {
        let mut key_store = MockKeyStore::new();
        // The underlying clear is itself idempotent and reports success.
        key_store.expect_clear().returning(|_| Ok(()));
        let probe = MockConnectionProbe::new();
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        service
            .remove_key(Provider::Stooq)
            .await
            .expect("idempotent remove must succeed");
    }

    // KEY-013 — a clear failure surfaces KeyStoreError.
    #[tokio::test]
    async fn remove_key_surfaces_clear_failure_as_key_store_error() {
        let mut key_store = MockKeyStore::new();
        key_store
            .expect_clear()
            .returning(|_| Err(anyhow::anyhow!("keychain delete failed")));
        let probe = MockConnectionProbe::new();
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        let error = service
            .remove_key(Provider::Stooq)
            .await
            .expect_err("clear failure must surface an error");
        assert!(
            matches!(error, ConnectionError::KeyStoreError),
            "got: {error:?}"
        );
    }

    // ---- resolve_key (KEY-018, backend-internal) ----

    // KEY-018 — resolve_key returns the stored value for the fetch path. This is
    // backend-internal and is never exposed as a Tauri command.
    #[tokio::test]
    async fn resolve_key_returns_stored_value_for_fetch_path() {
        let mut key_store = MockKeyStore::new();
        key_store
            .expect_read()
            .returning(|_| Ok(Some("resolved-key".to_string())));
        let probe = MockConnectionProbe::new();
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        let resolved = service
            .resolve_key(Provider::Stooq)
            .await
            .expect("resolve ok");
        assert_eq!(resolved, Some("resolved-key".to_string()));
    }

    // KEY-018 — resolve_key returns None when no key is stored.
    #[tokio::test]
    async fn resolve_key_returns_none_when_no_key() {
        let mut key_store = MockKeyStore::new();
        key_store.expect_read().returning(|_| Ok(None));
        let probe = MockConnectionProbe::new();
        let service = ConnectionService::new(Box::new(key_store), Box::new(probe));

        let resolved = service
            .resolve_key(Provider::Stooq)
            .await
            .expect("resolve ok");
        assert_eq!(resolved, None);
    }
}
