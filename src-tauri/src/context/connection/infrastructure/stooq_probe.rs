use crate::context::connection::domain::{ConnectionProbe, Provider, ProviderKeyTestOutcome};
use crate::core::logger::BACKEND;
use crate::shared::infrastructure::stooq::{recent_daily_window, StooqGate};
use anyhow::Result;
use async_trait::async_trait;
use chrono::Local;

/// Base URL of Stooq's keyed daily-download endpoint, the only surviving price
/// endpoint (ADR-015).
const STOOQ_DOWNLOAD_URL: &str = "https://stooq.com/q/d/l/";
/// Fixed, well-known symbol the provider is guaranteed to cover, so the probe is
/// deterministic even before the user has any holdings (KEY-021).
const STOOQ_PROBE_SYMBOL: &str = "spy.us";
/// First bytes of a genuine Stooq daily-download CSV; a rejected key yields a
/// non-CSV body instead.
const CSV_HEADER_PREFIX: &str = "Date,";

/// Production [`ConnectionProbe`] for Stooq: runs a fixed-symbol keyed request
/// through the shared [`StooqGate`] (so it clears the same proof-of-work gate the
/// fetch path does) and classifies the outcome (KEY-021/023).
pub struct StooqProbe {
    gate: StooqGate,
}

impl Default for StooqProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl StooqProbe {
    /// Creates a new probe backed by a fresh proof-of-work gate.
    pub fn new() -> Self {
        Self {
            gate: StooqGate::new(),
        }
    }
}

#[async_trait]
impl ConnectionProbe for StooqProbe {
    async fn probe(&self, _provider: Provider, key: &str) -> Result<ProviderKeyTestOutcome> {
        // A recent date window keeps the probe response to a handful of rows. The
        // key rides in the URL but never in the error label (KEY-014).
        let (from, to) = recent_daily_window(Local::now().date_naive());
        let url = format!(
            "{STOOQ_DOWNLOAD_URL}?s={STOOQ_PROBE_SYMBOL}&i=d&d1={from}&d2={to}&apikey={key}"
        );
        match self.gate.get_text(&url, STOOQ_PROBE_SYMBOL).await {
            // A genuine daily CSV means the key was accepted.
            Ok(body) if body.trim_start().starts_with(CSV_HEADER_PREFIX) => {
                Ok(ProviderKeyTestOutcome::Accepted)
            }
            // Reachable, but the body is not CSV (rejected key / challenge page).
            Ok(_) => Ok(ProviderKeyTestOutcome::Rejected),
            // Transport / verification failure — could not contact the provider.
            // The error is URL-free (the gate strips the key-bearing URL), so it
            // is safe to trace (KEY-014).
            Err(error) => {
                tracing::warn!(target: BACKEND, ?error, "connection: Stooq key probe unreachable");
                Ok(ProviderKeyTestOutcome::Unreachable)
            }
        }
    }
}
