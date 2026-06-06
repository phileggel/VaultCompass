use crate::context::currency::domain::rate_provider::{EurSnapshot, RateProvider};
use crate::context::currency::domain::CurrencyRateSource;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;

const FRANKFURTER_URL: &str = "https://api.frankfurter.dev/v1/latest?base=EUR";
const REQUEST_TIMEOUT_SECS: u64 = 10;
const MICROS_PER_UNIT: f64 = 1_000_000.0;

/// Production [`RateProvider`] backed by the Frankfurter JSON endpoint (ADR-009, FXR-070).
pub struct ReqwestFrankfurterClient {
    client: reqwest::Client,
}

impl ReqwestFrankfurterClient {
    /// Creates a new client with a 10-second per-request timeout.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .expect("reqwest client build");
        Self { client }
    }
}

impl Default for ReqwestFrankfurterClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RateProvider for ReqwestFrankfurterClient {
    async fn fetch_eur_snapshot(&self) -> Result<EurSnapshot> {
        let response = self
            .client
            .get(FRANKFURTER_URL)
            .send()
            .await
            .context("Frankfurter fetch request failed")?;
        if !response.status().is_success() {
            anyhow::bail!("Frankfurter returned status {}", response.status());
        }
        let body = crate::shared::infrastructure::http::read_capped_text(response)
            .await
            .context("Frankfurter response read failed")?;
        parse_frankfurter_snapshot(&body)
    }
}

/// Frankfurter JSON response shape (`{"date":"…","rates":{"USD":1.16,…}}`); the
/// `amount`/`base` fields are present but unused (we always request `base=EUR`).
#[derive(serde::Deserialize)]
struct FrankfurterResponse {
    date: String,
    rates: HashMap<String, f64>,
}

/// Parses the Frankfurter JSON response body into an `EurSnapshot` (FXR-070).
///
/// Expected shape: `{"amount":1.0,"base":"EUR","date":"YYYY-MM-DD","rates":{...}}`.
/// Rates are converted to i64 micros: `(value * 1_000_000.0).round() as i64`.
/// EUR itself is NOT present in the returned map (it is the implicit base).
pub(crate) fn parse_frankfurter_snapshot(body: &str) -> Result<EurSnapshot> {
    let parsed: FrankfurterResponse =
        serde_json::from_str(body).context("Frankfurter JSON parse failed")?;
    let rates = parsed
        .rates
        .into_iter()
        .map(|(code, value)| {
            // Reject non-finite or non-positive rates from an anomalous/compromised
            // feed before the micros cast (mirrors the Stooq client guard).
            if !value.is_finite() || value <= 0.0 {
                anyhow::bail!("Frankfurter rate out of range for {code}: {value}");
            }
            Ok((code, (value * MICROS_PER_UNIT).round() as i64))
        })
        .collect::<Result<_>>()?;
    Ok(EurSnapshot {
        date: parsed.date,
        rates,
        source: CurrencyRateSource::Frankfurter,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::currency::domain::CurrencyRateSource;

    const FIXTURE: &str = r#"{"amount":1.0,"base":"EUR","date":"2026-06-01","rates":{"USD":1.1646,"GBP":0.86493,"JPY":185.74}}"#;

    // FXR-070 — parses date and source from the Frankfurter JSON body
    #[test]
    fn parse_frankfurter_snapshot_parses_date_and_source() {
        let snapshot = parse_frankfurter_snapshot(FIXTURE).expect("should parse");
        assert_eq!(snapshot.date, "2026-06-01");
        assert_eq!(snapshot.source, CurrencyRateSource::Frankfurter);
    }

    // FXR-070 — USD rate is converted to 1_164_600 micros
    #[test]
    fn parse_frankfurter_snapshot_usd_rate_to_micros() {
        let snapshot = parse_frankfurter_snapshot(FIXTURE).expect("should parse");
        assert_eq!(snapshot.rates.get("USD").copied(), Some(1_164_600));
    }

    // FXR-070 — GBP rate is converted to 864_930 micros
    #[test]
    fn parse_frankfurter_snapshot_gbp_rate_to_micros() {
        let snapshot = parse_frankfurter_snapshot(FIXTURE).expect("should parse");
        assert_eq!(snapshot.rates.get("GBP").copied(), Some(864_930));
    }

    // FXR-070 — JPY rate is converted to 185_740_000 micros
    #[test]
    fn parse_frankfurter_snapshot_jpy_rate_to_micros() {
        let snapshot = parse_frankfurter_snapshot(FIXTURE).expect("should parse");
        assert_eq!(snapshot.rates.get("JPY").copied(), Some(185_740_000));
    }

    // FXR-070 — EUR is NOT present in the parsed rates map (it is the implicit base)
    #[test]
    fn parse_frankfurter_snapshot_eur_not_in_rates_map() {
        let snapshot = parse_frankfurter_snapshot(FIXTURE).expect("should parse");
        assert!(
            !snapshot.rates.contains_key("EUR"),
            "EUR must not appear as an explicit key in the rates map"
        );
    }

    // FXR-070 — malformed JSON body → Err
    #[test]
    fn parse_frankfurter_snapshot_malformed_json_returns_err() {
        let result = parse_frankfurter_snapshot("not valid json {{");
        assert!(result.is_err(), "malformed JSON must return Err");
    }

    // Hardening — a negative rate from an anomalous/compromised feed is rejected
    // before the micros cast rather than stored as a negative rate.
    #[test]
    fn parse_frankfurter_snapshot_rejects_negative_rate() {
        let body = r#"{"amount":1.0,"base":"EUR","date":"2026-06-01","rates":{"USD":-1.1646}}"#;
        let result = parse_frankfurter_snapshot(body);
        assert!(result.is_err(), "negative rate must return Err");
    }

    // Hardening — a zero rate is rejected (would later divide-by-zero in cross-rate).
    #[test]
    fn parse_frankfurter_snapshot_rejects_zero_rate() {
        let body = r#"{"amount":1.0,"base":"EUR","date":"2026-06-01","rates":{"USD":0.0}}"#;
        let result = parse_frankfurter_snapshot(body);
        assert!(result.is_err(), "zero rate must return Err");
    }
}
