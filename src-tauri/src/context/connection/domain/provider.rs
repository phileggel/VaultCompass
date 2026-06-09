use serde::{Deserialize, Serialize};
use specta::Type;

/// Which external provider a connection authenticates (KEY-031). Extensible:
/// Finnhub / OpenFIGI arrive as further variants in later slices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum Provider {
    /// Stooq daily-close price provider (ADR-015).
    Stooq,
}

/// Where a stored key currently lives, per the ADR-011 ladder (KEY-011, KEY-015).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum StorageTier {
    /// Tier 1 — default, persists, OS-encrypted.
    OsKeychain,
    /// Tier 2 — fallback, RAM-only, cleared on exit (KEY-017).
    SessionMemory,
    /// Tier 3 — explicit opt-in only (KEY-012).
    PlaintextFile,
}

/// Result of probing a provider with a candidate key (KEY-023).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum ProviderKeyTestOutcome {
    /// The provider accepted the key.
    Accepted,
    /// The provider was reachable but rejected the key.
    Rejected,
    /// The provider could not be contacted (network failure).
    Unreachable,
}

/// Non-secret state of one provider's connection, surfaced to the UI (KEY-016).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ProviderConnection {
    /// Which provider this connection authenticates.
    pub provider: Provider,
    /// Whether a key is stored (KEY-016); the value is never exposed (KEY-018).
    pub has_key: bool,
    /// Where the key lives (KEY-015); `None` when `has_key` is false.
    pub active_tier: Option<StorageTier>,
}
