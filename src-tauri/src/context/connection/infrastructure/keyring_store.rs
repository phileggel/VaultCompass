use crate::context::connection::domain::{KeyStore, Provider, StorageTier};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Mutex;

/// Keyring service name under which provider keys are stored (tier 1).
const KEYRING_SERVICE: &str = "VaultCompass";

/// Stable per-provider account label used as the keychain entry key, the
/// session-store key, and the plaintext filename stem. Never the secret (KEY-014).
fn provider_account(provider: Provider) -> &'static str {
    match provider {
        Provider::Stooq => "stooq",
    }
}

/// True when a keyring error means the OS keychain is simply not available on
/// this host (typical of a minimal Linux WM with no Secret Service / portal),
/// as opposed to a genuine fault against an available keychain.
fn is_keychain_unavailable(error: &keyring::Error) -> bool {
    matches!(
        error,
        keyring::Error::NoStorageAccess(_) | keyring::Error::PlatformFailure(_)
    )
}

/// Layered key store implementing the ADR-011 tier ladder (KEY-011):
/// OS keychain (tier 1) → session memory (tier 2) → opt-in plaintext file (tier 3).
///
/// An unavailable keychain transparently falls through to the lower tiers; a
/// genuine keychain fault surfaces as an error so a read fault is never mistaken
/// for "no key" (KEY-016). The secret never reaches a log (KEY-014).
pub struct LayeredKeyStore {
    /// Tier 2 — process-lifetime session memory (KEY-017): RAM-only, cleared on
    /// exit by virtue of living in this process, never written to disk.
    session: Mutex<HashMap<&'static str, String>>,
    /// Directory holding tier-3 opt-in plaintext key files (KEY-012).
    plaintext_dir: PathBuf,
}

impl LayeredKeyStore {
    /// Creates a store whose tier-3 plaintext files live under `plaintext_dir`.
    pub fn new(plaintext_dir: PathBuf) -> Self {
        Self {
            session: Mutex::new(HashMap::new()),
            plaintext_dir,
        }
    }

    fn plaintext_path(&self, account: &str) -> PathBuf {
        // `account` is always a fixed `&'static str` from `provider_account`, so
        // traversal is structurally impossible; assert the invariant in case the
        // source is ever widened to a user-supplied value.
        debug_assert!(
            !account.contains(std::path::MAIN_SEPARATOR),
            "provider account label must not contain a path separator"
        );
        self.plaintext_dir.join(format!("connection-{account}.key"))
    }

    fn session_get(&self, account: &str) -> Result<Option<String>> {
        Ok(self
            .session
            .lock()
            .map_err(|_| anyhow!("session key store lock poisoned"))?
            .get(account)
            .cloned())
    }

    fn session_set(&self, account: &'static str, key: String) -> Result<()> {
        self.session
            .lock()
            .map_err(|_| anyhow!("session key store lock poisoned"))?
            .insert(account, key);
        Ok(())
    }

    fn session_remove(&self, account: &str) -> Result<()> {
        self.session
            .lock()
            .map_err(|_| anyhow!("session key store lock poisoned"))?
            .remove(account);
        Ok(())
    }

    fn write_plaintext(&self, account: &str, key: &str) -> Result<()> {
        fs::create_dir_all(&self.plaintext_dir)
            .context("failed to create connection key directory")?;
        let path = self.plaintext_path(account);
        fs::write(&path, key).context("failed to write plaintext key file")?;
        // A credential file must be owner-read-only; the default umask would
        // leave it world-readable. No-op concern on Windows, where the file
        // inherits the private data dir's ACLs.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
                .context("failed to restrict plaintext key file permissions")?;
        }
        Ok(())
    }

    fn read_plaintext(&self, account: &str) -> Result<Option<String>> {
        match fs::read_to_string(self.plaintext_path(account)) {
            Ok(contents) => Ok(Some(contents)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).context("failed to read plaintext key file"),
        }
    }

    fn remove_plaintext(&self, account: &str) -> Result<()> {
        match fs::remove_file(self.plaintext_path(account)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error).context("failed to remove plaintext key file"),
        }
    }
}

/// Stores `key` in the OS keychain. `Ok(true)` when stored, `Ok(false)` when the
/// keychain is unavailable (caller falls through to a lower tier), `Err` on a
/// genuine keychain fault.
async fn keychain_set(account: &'static str, key: String) -> Result<bool> {
    tokio::task::spawn_blocking(move || -> Result<bool> {
        let entry = match keyring::Entry::new(KEYRING_SERVICE, account) {
            Ok(entry) => entry,
            Err(error) if is_keychain_unavailable(&error) => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        match entry.set_password(&key) {
            Ok(()) => Ok(true),
            Err(error) if is_keychain_unavailable(&error) => Ok(false),
            Err(error) => Err(error.into()),
        }
    })
    .await
    .context("keychain set task panicked")?
}

/// Reads a key from the OS keychain. `Ok(None)` covers both "no entry" and
/// "keychain unavailable"; `Err` is a genuine fault.
async fn keychain_get(account: &'static str) -> Result<Option<String>> {
    tokio::task::spawn_blocking(move || -> Result<Option<String>> {
        let entry = match keyring::Entry::new(KEYRING_SERVICE, account) {
            Ok(entry) => entry,
            Err(error) if is_keychain_unavailable(&error) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) if is_keychain_unavailable(&error) => Ok(None),
            Err(error) => Err(error.into()),
        }
    })
    .await
    .context("keychain get task panicked")?
}

/// Deletes a key from the OS keychain. A missing entry or an unavailable keychain
/// is a success (clearing is idempotent); a genuine fault is an error.
async fn keychain_delete(account: &'static str) -> Result<()> {
    tokio::task::spawn_blocking(move || -> Result<()> {
        let entry = match keyring::Entry::new(KEYRING_SERVICE, account) {
            Ok(entry) => entry,
            Err(error) if is_keychain_unavailable(&error) => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) if is_keychain_unavailable(&error) => Ok(()),
            Err(error) => Err(error.into()),
        }
    })
    .await
    .context("keychain delete task panicked")?
}

#[async_trait]
impl KeyStore for LayeredKeyStore {
    async fn clear(&self, provider: Provider) -> Result<()> {
        let account = provider_account(provider);
        // KEY-013 — clear every tier, not just the active one. Session and
        // plaintext are cleared best-effort before the keychain so a keychain
        // fault still leaves the lower tiers wiped.
        self.session_remove(account)?;
        self.remove_plaintext(account)?;
        keychain_delete(account).await
    }

    async fn store(
        &self,
        provider: Provider,
        key: &str,
        allow_plaintext: bool,
    ) -> Result<StorageTier> {
        let account = provider_account(provider);
        // Tier 1 — OS keychain. A genuine fault propagates (KEY-011: never lose
        // the key silently); an unavailable keychain falls through.
        if keychain_set(account, key.to_string()).await? {
            return Ok(StorageTier::OsKeychain);
        }
        // Tier 3 — opt-in plaintext, the persistent alternative when the keychain
        // is unavailable (KEY-012). Only reached on explicit opt-in.
        if allow_plaintext {
            self.write_plaintext(account, key)?;
            return Ok(StorageTier::PlaintextFile);
        }
        // Tier 2 — session memory, the guaranteed in-process floor (KEY-011).
        self.session_set(account, key.to_string())?;
        Ok(StorageTier::SessionMemory)
    }

    async fn locate(&self, provider: Provider) -> Result<Option<StorageTier>> {
        let account = provider_account(provider);
        match keychain_get(account).await {
            Ok(Some(_)) => return Ok(Some(StorageTier::OsKeychain)),
            Ok(None) => {}
            Err(error) => {
                // KEY-016 — a key may still live in a lower tier; only when none
                // does should a keychain fault surface (never mistaken for "no key").
                if let Some(tier) = self.locate_lower_tiers(account)? {
                    return Ok(Some(tier));
                }
                return Err(error).context("keychain status query failed");
            }
        }
        self.locate_lower_tiers(account)
    }

    async fn read(&self, provider: Provider) -> Result<Option<String>> {
        let account = provider_account(provider);
        match keychain_get(account).await {
            Ok(Some(key)) => return Ok(Some(key)),
            Ok(None) => {}
            Err(error) => {
                if let Some(key) = self.read_lower_tiers(account)? {
                    return Ok(Some(key));
                }
                return Err(error).context("keychain read failed");
            }
        }
        self.read_lower_tiers(account)
    }
}

impl LayeredKeyStore {
    fn locate_lower_tiers(&self, account: &str) -> Result<Option<StorageTier>> {
        if self.session_get(account)?.is_some() {
            return Ok(Some(StorageTier::SessionMemory));
        }
        if self.read_plaintext(account)?.is_some() {
            return Ok(Some(StorageTier::PlaintextFile));
        }
        Ok(None)
    }

    fn read_lower_tiers(&self, account: &str) -> Result<Option<String>> {
        if let Some(key) = self.session_get(account)? {
            return Ok(Some(key));
        }
        self.read_plaintext(account)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Each test gets an isolated plaintext dir; the session tier is per-instance.
    // Neither path touches the OS keychain, so these are deterministic on any host
    // (including a developer machine that has a real Stooq key saved).
    fn temp_store(tag: &str) -> LayeredKeyStore {
        let dir =
            std::env::temp_dir().join(format!("vaultcompass-keytest-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        LayeredKeyStore::new(dir)
    }

    fn cleanup(store: &LayeredKeyStore) {
        let _ = std::fs::remove_dir_all(&store.plaintext_dir);
    }

    // KEY-017 — the session tier holds and clears a key in process memory.
    #[test]
    fn session_tier_round_trips() {
        let store = temp_store("session");
        store.session_set("stooq", "k1".to_string()).unwrap();
        assert_eq!(store.session_get("stooq").unwrap(), Some("k1".to_string()));
        store.session_remove("stooq").unwrap();
        assert_eq!(store.session_get("stooq").unwrap(), None);
    }

    // KEY-012/013 — the plaintext tier writes, reads, and removes a key file.
    #[test]
    fn plaintext_tier_round_trips() {
        let store = temp_store("plaintext");
        assert_eq!(store.read_plaintext("stooq").unwrap(), None);
        store.write_plaintext("stooq", "k2").unwrap();
        assert_eq!(
            store.read_plaintext("stooq").unwrap(),
            Some("k2".to_string())
        );
        store.remove_plaintext("stooq").unwrap();
        assert_eq!(store.read_plaintext("stooq").unwrap(), None);
        cleanup(&store);
    }

    // KEY-012 — the plaintext key file is owner-read-only, never world-readable.
    #[cfg(unix)]
    #[test]
    fn plaintext_file_is_owner_read_only() {
        use std::os::unix::fs::PermissionsExt;
        let store = temp_store("plaintext-mode");
        store.write_plaintext("stooq", "k3").unwrap();
        let mode = std::fs::metadata(store.plaintext_path("stooq"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        cleanup(&store);
    }

    // remove_plaintext on an absent file is a no-op success (idempotent clear).
    #[test]
    fn remove_plaintext_is_idempotent_when_absent() {
        let store = temp_store("plaintext-absent");
        store.remove_plaintext("stooq").unwrap();
        cleanup(&store);
    }

    // KEY-011/016 — lower-tier location prefers session over plaintext, else None.
    #[test]
    fn locate_lower_tiers_prefers_session_then_plaintext() {
        let store = temp_store("locate");
        assert_eq!(store.locate_lower_tiers("stooq").unwrap(), None);
        store.write_plaintext("stooq", "k").unwrap();
        assert_eq!(
            store.locate_lower_tiers("stooq").unwrap(),
            Some(StorageTier::PlaintextFile)
        );
        store.session_set("stooq", "k".to_string()).unwrap();
        assert_eq!(
            store.locate_lower_tiers("stooq").unwrap(),
            Some(StorageTier::SessionMemory)
        );
        cleanup(&store);
    }

    // KEY-018 — lower-tier read returns the session value over the plaintext one.
    #[test]
    fn read_lower_tiers_prefers_session_value() {
        let store = temp_store("read");
        store.write_plaintext("stooq", "file-key").unwrap();
        assert_eq!(
            store.read_lower_tiers("stooq").unwrap(),
            Some("file-key".to_string())
        );
        store
            .session_set("stooq", "session-key".to_string())
            .unwrap();
        assert_eq!(
            store.read_lower_tiers("stooq").unwrap(),
            Some("session-key".to_string())
        );
        cleanup(&store);
    }
}
