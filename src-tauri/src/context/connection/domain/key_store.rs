use super::provider::{Provider, StorageTier};
use anyhow::Result;
use async_trait::async_trait;

/// Port over the storage-tier ladder that persists provider keys (ADR-011).
///
/// The secret never reaches the wire: `read` is backend-internal (KEY-018),
/// consumed only by the fetch path via `ConnectionService::resolve_key`.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait KeyStore: Send + Sync {
    /// Clears the key from every tier (KEY-013). Idempotent — clearing when no
    /// key exists succeeds.
    async fn clear(&self, provider: Provider) -> Result<()>;

    /// Stores `key` via the tier ladder (KEY-010/011/012), returning the tier the
    /// key landed in. `allow_plaintext` enables the tier-3 fallback (KEY-012).
    async fn store(
        &self,
        provider: Provider,
        key: &str,
        allow_plaintext: bool,
    ) -> Result<StorageTier>;

    /// Reports which tier currently holds the key, or `None` (KEY-016). Never
    /// returns the value.
    async fn locate(&self, provider: Provider) -> Result<Option<StorageTier>>;

    /// Reads the stored key back (KEY-018, backend-internal). Never surfaced to a
    /// command; consumed only by the fetch path.
    async fn read(&self, provider: Provider) -> Result<Option<String>>;
}
