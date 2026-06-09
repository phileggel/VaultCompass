use super::provider::{Provider, ProviderKeyTestOutcome};
use anyhow::Result;
use async_trait::async_trait;

/// Port over a one-shot live provider probe (KEY-021/022).
///
/// Read-only with respect to stored state: probing neither stores, replaces, nor
/// removes any persisted key (KEY-022).
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ConnectionProbe: Send + Sync {
    /// Probes `provider` with the candidate `key` using a fixed well-known symbol
    /// (KEY-021) and reports the outcome. The three outcomes are successful
    /// returns, not errors (KEY-023).
    async fn probe(&self, provider: Provider, key: &str) -> Result<ProviderKeyTestOutcome>;
}
