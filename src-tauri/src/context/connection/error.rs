/// Single flat error enum for the `connection` bounded context (gold error model).
///
/// `#[serde(tag = "code")]` makes each variant serialize as
/// `{ "code": "VariantName", ...payload }` on the wire. `KeyStoreError` is the
/// keychain-world analog of the SQLite contexts' `DatabaseError`: opaque on the
/// wire, full diagnostic chain preserved server-side via `tracing::error!`
/// (KEY-014 — the secret never appears in a variant or a log).
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type, Clone)]
#[serde(tag = "code")]
pub enum ConnectionError {
    /// The supplied key is blank or whitespace-only (KEY-010/021).
    #[error("Key must not be blank")]
    EmptyKey,

    /// An infrastructure failure occurred against a storage tier (OS keychain /
    /// session memory / plaintext file). Opaque on the wire — carries no secret
    /// payload (KEY-014). The diagnostic chain is preserved via `tracing::error!`.
    #[error("An unexpected key-store error occurred")]
    KeyStoreError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, to_value};

    /// Verifies the `#[serde(tag = "code")]` contract: every variant emits a
    /// flat `{ "code": "VariantName" }` object on the wire, with no secret
    /// payload (KEY-014). Mirrors currency's `each_variant_emits_a_code`.
    #[test]
    fn each_variant_emits_a_code() {
        assert_eq!(
            to_value(ConnectionError::EmptyKey).unwrap(),
            json!({ "code": "EmptyKey" })
        );
        assert_eq!(
            to_value(ConnectionError::KeyStoreError).unwrap(),
            json!({ "code": "KeyStoreError" })
        );
    }
}
