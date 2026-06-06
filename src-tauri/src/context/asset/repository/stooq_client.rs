use crate::context::asset::domain::PriceProvider;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::time::Duration;

const STOOQ_URL_TEMPLATE: &str = "https://stooq.com/q/l/?s={symbol}&f=sd2t2ohlcv&h&e=csv";
const REQUEST_TIMEOUT_SECS: u64 = 10;
const MICROS_PER_UNIT: f64 = 1_000_000.0;
const CSV_CLOSE_COLUMN_INDEX: usize = 6;

/// Stooq serves a JavaScript anti-bot challenge page (still as `text/csv`) to
/// clients without a browser-like `User-Agent`, so the request carries one.
const STOOQ_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// First bytes of a genuine Stooq CSV response. The anti-bot challenge page is
/// also served as `text/csv`, so the body — not the content type — distinguishes
/// a real quote from a challenge.
const CSV_HEADER_PREFIX: &str = "Symbol,Date,Time";

/// Production [`PriceProvider`] backed by Stooq's CSV endpoint (ADR-008).
pub struct ReqwestStooqClient {
    client: reqwest::Client,
}

impl Default for ReqwestStooqClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ReqwestStooqClient {
    /// Creates a new client with a 10-second per-request timeout.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .user_agent(STOOQ_USER_AGENT)
            .build()
            .expect("reqwest client build");
        Self { client }
    }
}

#[async_trait]
impl PriceProvider for ReqwestStooqClient {
    async fn fetch_price(&self, symbol: &str) -> Result<Option<i64>> {
        let url = STOOQ_URL_TEMPLATE.replace("{symbol}", symbol);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Stooq fetch request failed for symbol: {symbol}"))?;

        if !resp.status().is_success() {
            anyhow::bail!("Stooq returned {} for symbol {symbol}", resp.status());
        }

        let body = crate::shared::infrastructure::http::read_capped_text(resp)
            .await
            .with_context(|| format!("Stooq response read failed for symbol: {symbol}"))?;

        parse_close_micros(&body)
            .with_context(|| format!("Stooq response parse failed for symbol: {symbol}"))
    }
}

/// Stooq's CSV sentinel for "no data available for this symbol".
const NO_DATA_SENTINEL: &str = "N/D";

fn parse_close_micros(csv: &str) -> Result<Option<i64>> {
    if !csv.trim_start().starts_with(CSV_HEADER_PREFIX) {
        return Err(anyhow!(
            "Stooq returned a non-CSV response (likely an anti-bot challenge page)"
        ));
    }
    let data_row = csv
        .lines()
        .nth(1)
        .ok_or_else(|| anyhow!("missing data row"))?;
    let close = data_row
        .split(',')
        .nth(CSV_CLOSE_COLUMN_INDEX)
        .ok_or_else(|| anyhow!("missing close column"))?
        .trim();
    if close == NO_DATA_SENTINEL {
        return Ok(None);
    }
    let price: f64 = close
        .parse()
        .map_err(|e| anyhow!("close not numeric ({close:?}): {e}"))?;
    if !price.is_finite() || price <= 0.0 {
        return Err(anyhow!("close is non-finite or non-positive: {price}"));
    }
    Ok(Some((price * MICROS_PER_UNIT).round() as i64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_close_from_well_formed_csv() {
        let csv = "Symbol,Date,Time,Open,High,Low,Close,Volume\n\
                   AAPL.US,2026-05-16,21:55:00,189.50,190.20,188.75,189.95,12345678";
        let micros = parse_close_micros(csv).unwrap();
        assert_eq!(micros, Some(189_950_000));
    }

    #[test]
    fn rejects_missing_data_row() {
        let csv = "Symbol,Date,Time,Open,High,Low,Close,Volume\n";
        assert!(parse_close_micros(csv).is_err());
    }

    // Stooq returns the N/D sentinel for symbols it does not recognize. This is a
    // quiet "no data" outcome, not a parse failure — the dispatcher logs at debug
    // level and continues. See `PriceProvider::fetch_price` doc.
    #[test]
    fn returns_ok_none_when_close_is_no_data_sentinel() {
        let csv = "Symbol,Date,Time,Open,High,Low,Close,Volume\n\
                   FR0000120073,N/D,N/D,N/D,N/D,N/D,N/D,N/D";
        let result = parse_close_micros(csv).unwrap();
        assert_eq!(result, None);
    }

    // Stooq serves a JavaScript anti-bot challenge page (HTTP 200, typed
    // `text/csv`) to clients it suspects are bots. The header-prefix guard
    // rejects such a body before CSV parsing begins, with a clear message.
    #[test]
    fn rejects_anti_bot_challenge_page() {
        let challenge = "<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body>\
                         <noscript>This site requires JavaScript to verify your browser.</noscript>\
                         <script>(async()=>{const c=\"AAAA\",d=4,t=\"0\".repeat(d);let n=0;\
                         while(1){const x=(\"\"+n).split(\"\")).join(\"\");if(x.startsWith(t))break;n++}\
                         const r=await fetch(\"/__verify\");})()</script></body></html>";
        let error =
            parse_close_micros(challenge).expect_err("anti-bot challenge page must be rejected");
        assert!(
            error.to_string().contains("non-CSV"),
            "expected a non-CSV challenge error, got: {error}"
        );
    }

    #[test]
    fn rejects_non_numeric_close() {
        let csv = "Symbol,Date,Time,Open,High,Low,Close,Volume\n\
                   AAPL.US,2026-05-16,21:55:00,189.50,190.20,188.75,bogus,0";
        assert!(parse_close_micros(csv).is_err());
    }

    #[test]
    fn rejects_non_positive_close() {
        let csv = "Symbol,Date,Time,Open,High,Low,Close,Volume\n\
                   AAPL.US,2026-05-16,21:55:00,0,0,0,0,0";
        assert!(parse_close_micros(csv).is_err());
    }
}
