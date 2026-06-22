use crate::context::asset::domain::{
    HistoricalPriceProvider, HistoricalPriceRequest, PriceProvider, Quote,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, NaiveDate};
use std::time::Duration;

/// Yahoo Finance chart endpoint (ADR-017). Keyless, no proof-of-work; returns
/// JSON with the latest price in `chart.result[0].meta`. The `/v8/chart/` path
/// does not require the cookie/"crumb" handshake of Yahoo's `/v7/quote` endpoint.
const YAHOO_CHART_URL: &str = "https://query1.finance.yahoo.com/v8/finance/chart/";
/// Yahoo rejects the default reqwest user agent; present a browser-like one.
const USER_AGENT: &str = "Mozilla/5.0";
const REQUEST_TIMEOUT_SECS: u64 = 10;
const MICROS_PER_UNIT: f64 = 1_000_000.0;
/// Minor-unit divisor for pence/cents/agorot quotes (MKT-125).
const MINOR_UNIT_DIVISOR: f64 = 100.0;

/// Production [`PriceProvider`] backed by the keyless Yahoo Finance chart endpoint
/// (ADR-017).
pub struct ReqwestYahooClient {
    client: reqwest::Client,
}

impl ReqwestYahooClient {
    /// Creates a new client with a 10-second per-request timeout.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .expect("reqwest client build");
        Self { client }
    }
}

impl Default for ReqwestYahooClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PriceProvider for ReqwestYahooClient {
    async fn fetch_price(&self, symbol: &str) -> Result<Option<Quote>> {
        let url = format!("{YAHOO_CHART_URL}{symbol}?interval=1d&range=1d");
        let response = self
            .client
            .get(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .with_context(|| format!("Yahoo fetch request failed for symbol: {symbol}"))?;
        // Yahoo returns 200 with a JSON `chart.error` for an unknown symbol; only a
        // genuine transport failure is a non-success status.
        if !response.status().is_success() {
            anyhow::bail!("Yahoo returned status {} for {symbol}", response.status());
        }
        let body = crate::shared::infrastructure::http::read_capped_text(response)
            .await
            .with_context(|| format!("Yahoo response read failed for symbol: {symbol}"))?;
        parse_quote(&body)
            .with_context(|| format!("Yahoo response parse failed for symbol: {symbol}"))
    }
}

/// Days of look-back included before the target date so a target falling on a
/// closed market day (weekend / holiday) carries back to the most recent prior
/// close. A week comfortably spans any exchange holiday run.
const HISTORICAL_LOOKBACK_DAYS: i64 = 7;

#[async_trait]
impl HistoricalPriceProvider for ReqwestYahooClient {
    async fn fetch_price_on_date(&self, request: HistoricalPriceRequest) -> Result<Option<Quote>> {
        let HistoricalPriceRequest { symbol, date } = request;
        let target = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
            .with_context(|| format!("invalid target date for historical fetch: {date}"))?;
        // Window: [target − 7d, target + 2d). period2 is exclusive at the day after
        // the target so the target day's candle is always included regardless of the
        // exchange timezone; candles after the target are filtered out in parsing.
        let day_start = |day: NaiveDate| {
            day.and_hms_opt(0, 0, 0)
                .map(|datetime| datetime.and_utc().timestamp())
        };
        let (Some(period1), Some(period2)) = (
            day_start(target - ChronoDuration::days(HISTORICAL_LOOKBACK_DAYS)),
            day_start(target + ChronoDuration::days(2)),
        ) else {
            return Err(anyhow!("could not build fetch window for date: {date}"));
        };
        let url =
            format!("{YAHOO_CHART_URL}{symbol}?interval=1d&period1={period1}&period2={period2}");
        let response = self
            .client
            .get(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .with_context(|| format!("Yahoo historical request failed for symbol: {symbol}"))?;
        if !response.status().is_success() {
            anyhow::bail!("Yahoo returned status {} for {symbol}", response.status());
        }
        let body = crate::shared::infrastructure::http::read_capped_text(response)
            .await
            .with_context(|| format!("Yahoo historical read failed for symbol: {symbol}"))?;
        parse_historical_quote(&body, target)
            .with_context(|| format!("Yahoo historical parse failed for symbol: {symbol}"))
    }
}

/// Yahoo `/v8/chart/` response shape — only the fields we consume.
#[derive(serde::Deserialize)]
struct ChartEnvelope {
    chart: Chart,
}

#[derive(serde::Deserialize)]
struct Chart {
    result: Option<Vec<ChartResult>>,
    // Present (non-null) for an unknown / delisted symbol; its presence alone is
    // the skip signal (MKT-114), so the inner `{code, description}` is not modeled.
    error: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct ChartResult {
    meta: Meta,
}

#[derive(serde::Deserialize)]
struct Meta {
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    currency: Option<String>,
    #[serde(rename = "regularMarketTime")]
    regular_market_time: Option<i64>,
    gmtoffset: Option<i64>,
}

/// Parses the latest quote from a Yahoo chart JSON body.
///
/// - `Ok(Some(quote))` — usable price (normalized to the major ISO unit per
///   MKT-125) with its observation date.
/// - `Ok(None)` — symbol not found (`chart.error` present) or no price in the
///   response; a quiet per-asset skip (MKT-114).
/// - `Err(_)` — malformed JSON or an anomalous (non-finite / non-positive) price.
fn parse_quote(body: &str) -> Result<Option<Quote>> {
    let envelope: ChartEnvelope =
        serde_json::from_str(body).context("Yahoo chart JSON parse failed")?;
    // Unknown / delisted symbol — Yahoo populates `error` and nulls `result`.
    if envelope.chart.error.is_some() {
        return Ok(None);
    }
    let Some(meta) = envelope
        .chart
        .result
        .and_then(|mut results| results.drain(..).next())
        .map(|r| r.meta)
    else {
        return Ok(None);
    };
    let Some(raw_price) = meta.regular_market_price else {
        return Ok(None);
    };
    // MKT-125 — pence/cents/agorot quotes are divided by 100 to the major ISO unit.
    let price = normalize_minor_unit(raw_price, meta.currency.as_deref());
    if !price.is_finite() || price <= 0.0 {
        return Err(anyhow!(
            "Yahoo price is non-finite or non-positive: {price}"
        ));
    }
    // Observation date (MKT-117): the regular-market timestamp shifted to the
    // exchange-local calendar date via gmtoffset. Validation/fallback is the
    // dispatcher's job (MKT-118); this forwards the raw date or `None`.
    let date = match meta.regular_market_time {
        Some(ts) => chrono::DateTime::from_timestamp(ts + meta.gmtoffset.unwrap_or(0), 0)
            .map(|dt| dt.naive_utc().date().format("%Y-%m-%d").to_string()),
        None => None,
    };
    Ok(Some(Quote {
        price: (price * MICROS_PER_UNIT).round() as i64,
        date,
    }))
}

/// Yahoo `/v8/chart/` historical response shape (period1/period2 query) — only
/// the fields the carry-back picker consumes.
#[derive(serde::Deserialize)]
struct HistoricalEnvelope {
    chart: HistoricalChart,
}

#[derive(serde::Deserialize)]
struct HistoricalChart {
    result: Option<Vec<HistoricalResult>>,
    error: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct HistoricalResult {
    meta: HistoricalMeta,
    timestamp: Option<Vec<i64>>,
    indicators: Indicators,
}

#[derive(serde::Deserialize)]
struct HistoricalMeta {
    currency: Option<String>,
    gmtoffset: Option<i64>,
}

#[derive(serde::Deserialize)]
struct Indicators {
    quote: Vec<QuoteSeries>,
}

#[derive(serde::Deserialize)]
struct QuoteSeries {
    close: Option<Vec<Option<f64>>>,
}

/// Picks the most recent daily close at or before `target` from a Yahoo historical
/// chart body (MKT-125 minor-unit normalization applied).
///
/// - `Ok(Some(quote))` — a usable close was found; `quote.date` is the candle's own
///   date (may predate `target` when the target was a closed market day).
/// - `Ok(None)` — unknown symbol (`chart.error`), empty series, or no usable close at
///   or before the target. A quiet per-asset skip.
/// - `Err(_)` — malformed JSON.
fn parse_historical_quote(body: &str, target: NaiveDate) -> Result<Option<Quote>> {
    let envelope: HistoricalEnvelope =
        serde_json::from_str(body).context("Yahoo historical JSON parse failed")?;
    if envelope.chart.error.is_some() {
        return Ok(None);
    }
    let Some(result) = envelope
        .chart
        .result
        .and_then(|mut results| results.drain(..).next())
    else {
        return Ok(None);
    };
    let (Some(timestamps), Some(series)) =
        (result.timestamp, result.indicators.quote.into_iter().next())
    else {
        return Ok(None);
    };
    let Some(closes) = series.close else {
        return Ok(None);
    };
    let gmtoffset = result.meta.gmtoffset.unwrap_or(0);
    let currency = result.meta.currency.as_deref();
    // Walk every (timestamp, close) candle and keep the one with the latest local
    // date that is still at or before the target — the carry-back close.
    let mut best: Option<(NaiveDate, f64)> = None;
    for (timestamp, raw_close) in timestamps.iter().zip(closes.iter()) {
        let Some(raw_close) = raw_close else {
            continue;
        };
        let Some(observed) = chrono::DateTime::from_timestamp(timestamp + gmtoffset, 0) else {
            continue;
        };
        let candle_date = observed.naive_utc().date();
        if candle_date > target {
            continue;
        }
        // MKT-125 — pence/cents/agorot quotes are divided by 100 to the major ISO unit.
        let price = normalize_minor_unit(*raw_close, currency);
        if !price.is_finite() || price <= 0.0 {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|(best_date, _)| candle_date >= *best_date)
        {
            best = Some((candle_date, price));
        }
    }
    let Some((candle_date, price)) = best else {
        return Ok(None);
    };
    Ok(Some(Quote {
        price: (price * MICROS_PER_UNIT).round() as i64,
        date: Some(candle_date.format("%Y-%m-%d").to_string()),
    }))
}

/// MKT-125 — converts a price quoted in a currency's minor unit to its major ISO
/// unit. Yahoo reports London in `GBp` (pence), Johannesburg in `ZAc` (cents),
/// Tel Aviv in `ILA` (agorot). Any other currency is treated as already major.
fn normalize_minor_unit(price: f64, currency: Option<&str>) -> f64 {
    match currency {
        Some("GBp") | Some("ZAc") | Some("ILA") => price / MINOR_UNIT_DIVISOR,
        _ => price,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // MKT-102/117 — a US quote: price to micros, date from the regular-market timestamp.
    #[test]
    fn parses_us_quote_price_and_date() {
        // regularMarketTime 1781294401 + gmtoffset -14400 → 2026-06-12 (EDT date).
        let body = r#"{"chart":{"result":[{"meta":{"currency":"USD","symbol":"AAPL","regularMarketPrice":291.13,"regularMarketTime":1781294401,"gmtoffset":-14400}}],"error":null}}"#;
        let quote = parse_quote(body).unwrap().expect("a usable quote");
        assert_eq!(quote.price, 291_130_000);
        assert_eq!(quote.date.as_deref(), Some("2026-06-12"));
    }

    // MKT-125 — a London (LSE) quote in GBp is divided by 100 to GBP.
    #[test]
    fn normalizes_gbp_pence_to_pounds() {
        let body = r#"{"chart":{"result":[{"meta":{"currency":"GBp","symbol":"VOD.L","regularMarketPrice":115.75,"regularMarketTime":1781287549,"gmtoffset":3600}}],"error":null}}"#;
        let quote = parse_quote(body).unwrap().expect("a usable quote");
        // 115.75 GBp = 1.1575 GBP → 1_157_500 micros.
        assert_eq!(quote.price, 1_157_500);
    }

    // MKT-125 — a major-ISO currency (USD) is stored unchanged (no division).
    #[test]
    fn major_currency_unchanged() {
        assert_eq!(normalize_minor_unit(291.13, Some("USD")), 291.13);
        assert_eq!(normalize_minor_unit(115.75, Some("GBp")), 1.1575);
        assert_eq!(normalize_minor_unit(50.0, Some("ZAc")), 0.5);
    }

    // MKT-114 — an unknown symbol (chart.error present) is a quiet skip (Ok(None)).
    #[test]
    fn unknown_symbol_returns_ok_none() {
        let body = r#"{"chart":{"result":null,"error":{"code":"Not Found","description":"No data found, symbol may be delisted"}}}"#;
        assert_eq!(parse_quote(body).unwrap(), None);
    }

    // No price in the response (null regularMarketPrice) is a quiet skip.
    #[test]
    fn missing_price_returns_ok_none() {
        let body =
            r#"{"chart":{"result":[{"meta":{"currency":"USD","symbol":"AAPL"}}],"error":null}}"#;
        assert_eq!(parse_quote(body).unwrap(), None);
    }

    // A timestamp without gmtoffset still yields a date (offset defaults to 0/UTC).
    #[test]
    fn timestamp_without_gmtoffset_uses_utc() {
        let body = r#"{"chart":{"result":[{"meta":{"currency":"USD","symbol":"AAPL","regularMarketPrice":100.0,"regularMarketTime":1781294401}}],"error":null}}"#;
        let quote = parse_quote(body).unwrap().expect("a usable quote");
        assert!(quote.date.is_some());
    }

    // Malformed JSON is an Err.
    #[test]
    fn malformed_json_returns_err() {
        assert!(parse_quote("not json {{").is_err());
    }

    // A non-positive price from an anomalous feed is rejected.
    #[test]
    fn rejects_non_positive_price() {
        let body = r#"{"chart":{"result":[{"meta":{"currency":"USD","symbol":"AAPL","regularMarketPrice":0.0,"regularMarketTime":1781294401,"gmtoffset":0}}],"error":null}}"#;
        assert!(parse_quote(body).is_err());
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
    }

    // Historical: an exact-date candle is returned at its own date.
    #[test]
    fn historical_picks_exact_target_date_close() {
        // 1717977600 = 2024-06-10 00:00 UTC, 1718064000 = 2024-06-11 00:00 UTC.
        let body = r#"{"chart":{"result":[{"meta":{"currency":"USD","gmtoffset":0},"timestamp":[1717977600,1718064000],"indicators":{"quote":[{"close":[100.0,110.0]}]}}],"error":null}}"#;
        let quote = parse_historical_quote(body, date(2024, 6, 11))
            .unwrap()
            .expect("a usable quote");
        assert_eq!(quote.price, 110_000_000);
        assert_eq!(quote.date.as_deref(), Some("2024-06-11"));
    }

    // Historical carry-back: a target after the last candle returns that last candle.
    #[test]
    fn historical_carries_back_to_most_recent_prior_close() {
        // Last candle is 2024-06-11; target 2024-06-15 (a closed market day) carries back.
        let body = r#"{"chart":{"result":[{"meta":{"currency":"USD","gmtoffset":0},"timestamp":[1717977600,1718064000],"indicators":{"quote":[{"close":[100.0,110.0]}]}}],"error":null}}"#;
        let quote = parse_historical_quote(body, date(2024, 6, 15))
            .unwrap()
            .expect("a usable quote");
        assert_eq!(quote.price, 110_000_000);
        assert_eq!(quote.date.as_deref(), Some("2024-06-11"));
    }

    // Historical: candles strictly after the target are ignored.
    #[test]
    fn historical_ignores_candles_after_target() {
        let body = r#"{"chart":{"result":[{"meta":{"currency":"USD","gmtoffset":0},"timestamp":[1717977600,1718064000],"indicators":{"quote":[{"close":[100.0,110.0]}]}}],"error":null}}"#;
        let quote = parse_historical_quote(body, date(2024, 6, 10))
            .unwrap()
            .expect("a usable quote");
        assert_eq!(quote.price, 100_000_000);
        assert_eq!(quote.date.as_deref(), Some("2024-06-10"));
    }

    // Historical: null closes (no trade that day) are skipped; the prior real close wins.
    #[test]
    fn historical_skips_null_closes() {
        let body = r#"{"chart":{"result":[{"meta":{"currency":"USD","gmtoffset":0},"timestamp":[1717977600,1718064000],"indicators":{"quote":[{"close":[100.0,null]}]}}],"error":null}}"#;
        let quote = parse_historical_quote(body, date(2024, 6, 11))
            .unwrap()
            .expect("a usable quote");
        assert_eq!(quote.price, 100_000_000);
        assert_eq!(quote.date.as_deref(), Some("2024-06-10"));
    }

    // MKT-125 — a London (GBp) historical close is divided by 100 to GBP.
    #[test]
    fn historical_normalizes_gbp_pence() {
        let body = r#"{"chart":{"result":[{"meta":{"currency":"GBp","gmtoffset":0},"timestamp":[1718064000],"indicators":{"quote":[{"close":[115.75]}]}}],"error":null}}"#;
        let quote = parse_historical_quote(body, date(2024, 6, 11))
            .unwrap()
            .expect("a usable quote");
        assert_eq!(quote.price, 1_157_500);
    }

    // Historical: an unknown symbol (chart.error present) is a quiet skip.
    #[test]
    fn historical_unknown_symbol_returns_ok_none() {
        let body = r#"{"chart":{"result":null,"error":{"code":"Not Found","description":"No data found"}}}"#;
        assert_eq!(
            parse_historical_quote(body, date(2024, 6, 11)).unwrap(),
            None
        );
    }

    // Historical: no candle at or before the target (target predates history) is a skip.
    #[test]
    fn historical_no_candle_before_target_returns_ok_none() {
        let body = r#"{"chart":{"result":[{"meta":{"currency":"USD","gmtoffset":0},"timestamp":[1718064000],"indicators":{"quote":[{"close":[110.0]}]}}],"error":null}}"#;
        assert_eq!(
            parse_historical_quote(body, date(2024, 6, 1)).unwrap(),
            None
        );
    }

    // Historical: malformed JSON is an Err.
    #[test]
    fn historical_malformed_json_returns_err() {
        assert!(parse_historical_quote("not json {{", date(2024, 6, 11)).is_err());
    }
}
