/// Storage-tier ladder port (`KeyStore`).
pub mod key_store;
/// Live provider probe port (`ConnectionProbe`).
pub mod probe;
/// Provider, tier, and connection-state types.
pub mod provider;

pub use key_store::KeyStore;
pub use probe::ConnectionProbe;
pub use provider::{Provider, ProviderConnection, ProviderKeyTestOutcome, StorageTier};

#[cfg(test)]
pub use key_store::MockKeyStore;
#[cfg(test)]
pub use probe::MockConnectionProbe;
