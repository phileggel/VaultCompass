use crate::context::asset::domain::{PriceProvider, Quote};
use crate::shared::infrastructure::stooq::StooqGate;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;

/// Base URL of Stooq's keyed daily-download endpoint (ADR-015). The light `q/l/`
/// single-quote endpoint was withdrawn (404s even with a key), so the full daily
/// history is downloaded and the latest row taken. A BYOK apikey is required and,
/// per a 2026-06-08 live probe, does NOT bypass the proof-of-work gate — the
/// shared [`StooqGate`] clears that before this endpoint returns CSV.
const STOOQ_DOWNLOAD_URL: &str = "https://stooq.com/q/d/l/";
const MICROS_PER_UNIT: f64 = 1_000_000.0;

/// First bytes of a genuine Stooq daily-download CSV. A rejected key or an
/// anti-bot page does not start this way, so the body (not the status) is the
/// discriminator.
const CSV_HEADER_PREFIX: &str = "Date,";
/// Column layout of the daily-download CSV: `Date,Open,High,Low,Close,Volume`.
const CSV_DATE_COLUMN_INDEX: usize = 0;
const CSV_CLOSE_COLUMN_INDEX: usize = 4;
/// Stooq's CSV sentinel for "no data available for this symbol".
const NO_DATA_SENTINEL: &str = "N/D";

/// Production [`PriceProvider`] backed by Stooq's keyed daily-download endpoint
/// (ADR-015). Holds a [`StooqGate`] so the proof-of-work `auth` cookie is solved
/// once per session and reused across symbols.
pub struct ReqwestStooqClient {
    gate: StooqGate,
}

impl Default for ReqwestStooqClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ReqwestStooqClient {
    /// Creates a new client backed by a fresh proof-of-work gate.
    pub fn new() -> Self {
        Self {
            gate: StooqGate::new(),
        }
    }
}

#[async_trait]
impl PriceProvider for ReqwestStooqClient {
    async fn fetch_price(&self, symbol: &str, api_key: &str) -> Result<Option<Quote>> {
        // The key is in the URL but never in the error label (KEY-014): the label
        // carries only the symbol.
        let url = format!("{STOOQ_DOWNLOAD_URL}?s={symbol}&i=d&apikey={api_key}");
        let body = self.gate.get_text(&url, symbol).await?;
        parse_quote(&body)
            .with_context(|| format!("Stooq response parse failed for symbol: {symbol}"))
    }
}

/// Parses the latest quote from a Stooq daily-download CSV (`Date,Open,High,Low,
/// Close,Volume`, date-ascending). Takes the last data row — the most recent
/// trading day. `Ok(None)` when the symbol has no data; `Err` when the body is not
/// CSV (a rejected key or anti-bot challenge survived the gate).
fn parse_quote(csv: &str) -> Result<Option<Quote>> {
    let trimmed = csv.trim_start();
    if !trimmed.starts_with(CSV_HEADER_PREFIX) {
        return Err(anyhow!(
            "Stooq returned a non-CSV response (likely a rejected key or anti-bot challenge page)"
        ));
    }
    let Some(row) = trimmed
        .lines()
        .skip(1)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .last()
    else {
        // Header only — the symbol exists to Stooq but has no rows.
        return Ok(None);
    };
    let columns: Vec<&str> = row.split(',').collect();
    let close = columns
        .get(CSV_CLOSE_COLUMN_INDEX)
        .ok_or_else(|| anyhow!("missing close column"))?
        .trim();
    if close.is_empty() || close.eq_ignore_ascii_case(NO_DATA_SENTINEL) {
        return Ok(None);
    }
    let price: f64 = close
        .parse()
        .map_err(|e| anyhow!("close not numeric ({close:?}): {e}"))?;
    if !price.is_finite() || price <= 0.0 {
        return Err(anyhow!("close is non-finite or non-positive: {price}"));
    }
    // Observation date (MKT-117). Validation/fallback is the dispatcher's job
    // (MKT-118); this only forwards the raw value, dropping an empty / N/D cell.
    let date = columns
        .get(CSV_DATE_COLUMN_INDEX)
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty() && !cell.eq_ignore_ascii_case(NO_DATA_SENTINEL))
        .map(str::to_string);
    Ok(Some(Quote {
        price: (price * MICROS_PER_UNIT).round() as i64,
        date,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_latest_row_close_and_date_from_daily_csv() {
        let csv = "Date,Open,High,Low,Close,Volume\n\
                   2026-05-15,188.10,189.00,187.50,188.40,10000000\n\
                   2026-05-16,189.50,190.20,188.75,189.95,12345678";
        let quote = parse_quote(csv).unwrap().expect("a usable quote");
        // Latest row (last line) is taken, not the first.
        assert_eq!(quote.price, 189_950_000);
        assert_eq!(quote.date.as_deref(), Some("2026-05-16"));
    }

    #[test]
    fn returns_ok_none_when_header_only() {
        let csv = "Date,Open,High,Low,Close,Volume\n";
        assert_eq!(parse_quote(csv).unwrap(), None);
    }

    #[test]
    fn returns_ok_none_when_close_is_no_data_sentinel() {
        let csv = "Date,Open,High,Low,Close,Volume\n\
                   2026-05-16,N/D,N/D,N/D,N/D,N/D";
        assert_eq!(parse_quote(csv).unwrap(), None);
    }

    // A rejected key or an anti-bot challenge page that survived the gate does not
    // start with the CSV header; it must be rejected before parsing.
    #[test]
    fn rejects_non_csv_body() {
        let body = "<!DOCTYPE html><html><body><noscript>This site requires JavaScript\
                    </noscript></body></html>";
        let error = parse_quote(body).expect_err("non-CSV body must be rejected");
        assert!(
            error.to_string().contains("non-CSV"),
            "expected a non-CSV error, got: {error}"
        );
    }

    #[test]
    fn rejects_non_numeric_close() {
        let csv = "Date,Open,High,Low,Close,Volume\n\
                   2026-05-16,189.50,190.20,188.75,bogus,0";
        assert!(parse_quote(csv).is_err());
    }

    #[test]
    fn rejects_non_positive_close() {
        let csv = "Date,Open,High,Low,Close,Volume\n\
                   2026-05-16,0,0,0,0,0";
        assert!(parse_quote(csv).is_err());
    }
}
