use crate::context::asset::domain::{PriceProvider, Quote};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::time::Duration;

const STOOQ_URL_TEMPLATE: &str = "https://stooq.com/q/l/?s={symbol}&f=sd2t2ohlcv&h&e=csv";
/// Endpoint that clears the proof-of-work challenge: a successful POST sets the
/// `auth` cookie the CSV endpoint then honours. See [`ReqwestStooqClient`].
const STOOQ_VERIFY_URL: &str = "https://stooq.com/__verify";
const REQUEST_TIMEOUT_SECS: u64 = 10;
const MICROS_PER_UNIT: f64 = 1_000_000.0;
const CSV_DATE_COLUMN_INDEX: usize = 1;
const CSV_CLOSE_COLUMN_INDEX: usize = 6;

/// Browser-like `User-Agent` sent on every Stooq request. Necessary but no
/// longer sufficient on its own — Stooq now gates the CSV behind a JavaScript
/// proof-of-work challenge served to all clients regardless of `User-Agent`
/// (L-005); [`ReqwestStooqClient::clear_challenge`] solves that gate.
const STOOQ_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// First bytes of a genuine Stooq CSV response. The anti-bot challenge page is
/// also served as `text/csv`, so the body — not the content type — distinguishes
/// a real quote from a challenge.
const CSV_HEADER_PREFIX: &str = "Symbol,Date,Time";

/// Upper bound on the proof-of-work difficulty we will attempt. The challenge
/// token and difficulty come from an untrusted server response; observed
/// difficulty is 4, so 5 leaves headroom while refusing an adversarial value
/// that would make the solve infeasible.
const MAX_POW_DIFFICULTY: usize = 5;
/// Hard ceiling on proof-of-work attempts, guaranteeing termination even if a
/// (capped) difficulty never resolves. At difficulty 5 the expected work is
/// ~1M iterations, so 50M makes a genuine miss astronomically unlikely.
const MAX_POW_ITERATIONS: u64 = 50_000_000;
/// Upper bound on the challenge token length. Real tokens are ~20 chars; this
/// caps the per-iteration allocation against a hostile multi-KiB token.
const MAX_TOKEN_LEN: usize = 128;

/// Production [`PriceProvider`] backed by Stooq's CSV endpoint (ADR-008).
///
/// Stooq gates the CSV behind a JavaScript proof-of-work challenge (L-005): the
/// first request of a launch returns an HTML page carrying a challenge token and
/// difficulty, the client must find a nonce whose `SHA-256(token + nonce)` hex
/// digest starts with `difficulty` zeros, POST it to [`STOOQ_VERIFY_URL`], and
/// retry. The verification sets an `auth` cookie; with the cookie store enabled
/// the challenge is solved once per launch and every later symbol reuses it.
pub struct ReqwestStooqClient {
    client: reqwest::Client,
}

impl Default for ReqwestStooqClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ReqwestStooqClient {
    /// Creates a new client with a 10-second per-request timeout and a cookie
    /// store (so the proof-of-work `auth` cookie persists across requests).
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .user_agent(STOOQ_USER_AGENT)
            .cookie_store(true)
            .build()
            .expect("reqwest client build");
        Self { client }
    }

    /// Fetches `url` and returns its body, failing on a non-success status.
    async fn fetch_body(&self, url: &str, symbol: &str) -> Result<String> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Stooq fetch request failed for symbol: {symbol}"))?;

        if !resp.status().is_success() {
            anyhow::bail!("Stooq returned {} for symbol {symbol}", resp.status());
        }

        crate::shared::infrastructure::http::read_capped_text(resp)
            .await
            .with_context(|| format!("Stooq response read failed for symbol: {symbol}"))
    }

    /// Solves the proof-of-work `challenge` and POSTs the nonce to
    /// [`STOOQ_VERIFY_URL`], which sets the `auth` cookie that unlocks the CSV.
    async fn clear_challenge(&self, challenge: Challenge) -> Result<()> {
        // The solve is CPU-bound; run it off the async executor thread.
        let (token, nonce) = tokio::task::spawn_blocking(move || {
            solve_proof_of_work(&challenge.token, challenge.difficulty)
                .map(|nonce| (challenge.token, nonce.to_string()))
        })
        .await
        .context("Stooq proof-of-work task panicked")?
        .ok_or_else(|| anyhow!("Stooq proof-of-work exceeded the iteration ceiling"))?;

        let resp = self
            .client
            .post(STOOQ_VERIFY_URL)
            .form(&[("c", token.as_str()), ("n", nonce.as_str())])
            .send()
            .await
            .context("Stooq challenge verification request failed")?;

        if !resp.status().is_success() {
            anyhow::bail!("Stooq challenge verification returned {}", resp.status());
        }

        // Drain the body so the connection returns to the pool, matching the
        // capped-read pattern used for every other Stooq response.
        crate::shared::infrastructure::http::read_capped_text(resp)
            .await
            .context("Stooq challenge verification response read failed")?;
        Ok(())
    }
}

#[async_trait]
impl PriceProvider for ReqwestStooqClient {
    async fn fetch_price(&self, symbol: &str) -> Result<Option<Quote>> {
        let url = STOOQ_URL_TEMPLATE.replace("{symbol}", symbol);
        let mut body = self.fetch_body(&url, symbol).await?;

        // Cold start (or expired cookie): the body is the proof-of-work
        // challenge page rather than CSV. Solve it, then retry once — the
        // cookie store now carries the `auth` cookie (L-005).
        if !is_csv(&body) {
            if let Some(challenge) = parse_challenge(&body) {
                self.clear_challenge(challenge).await?;
                body = self.fetch_body(&url, symbol).await?;
            }
        }

        parse_quote(&body)
            .with_context(|| format!("Stooq response parse failed for symbol: {symbol}"))
    }
}

/// A Stooq proof-of-work anti-bot challenge: find a nonce whose
/// `SHA-256(token + nonce)` hex digest starts with `difficulty` zeros.
struct Challenge {
    token: String,
    difficulty: usize,
}

/// True when `body` is a genuine Stooq CSV quote rather than a challenge page.
fn is_csv(body: &str) -> bool {
    body.trim_start().starts_with(CSV_HEADER_PREFIX)
}

/// Extracts the proof-of-work parameters from a Stooq challenge page, or `None`
/// when `body` carries no recognizable challenge. The page embeds them as
/// `…const c="<token>",d=<difficulty>,…` inside the verification script.
fn parse_challenge(body: &str) -> Option<Challenge> {
    let token = extract_between(body, "c=\"", "\"")?;
    if token.len() > MAX_TOKEN_LEN {
        return None;
    }
    let difficulty: usize = extract_between(body, ",d=", ",")?.parse().ok()?;
    if difficulty > MAX_POW_DIFFICULTY {
        return None;
    }
    Some(Challenge {
        token: token.to_string(),
        difficulty,
    })
}

/// Returns the substring of `haystack` between the first occurrence of `start`
/// and the next occurrence of `end` after it.
fn extract_between<'a>(haystack: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let after_start = &haystack[haystack.find(start)? + start.len()..];
    let length = after_start.find(end)?;
    Some(&after_start[..length])
}

/// Brute-forces the smallest nonce `n` such that the hex SHA-256 of
/// `{token}{n}` begins with `difficulty` zero digits (Stooq uses difficulty 4,
/// ≈ 100k iterations — sub-millisecond). Returns `None` if no nonce is found
/// within [`MAX_POW_ITERATIONS`], guaranteeing termination.
fn solve_proof_of_work(token: &str, difficulty: usize) -> Option<u64> {
    (0..MAX_POW_ITERATIONS).find(|nonce| {
        let digest = Sha256::digest(format!("{token}{nonce}").as_bytes());
        has_leading_hex_zeros(digest.as_slice(), difficulty)
    })
}

/// True when the hex representation of `digest` starts with `count` `0` digits,
/// tested directly on the bytes to avoid allocating a hex string per attempt.
fn has_leading_hex_zeros(digest: &[u8], count: usize) -> bool {
    let whole_bytes = count / 2;
    if digest.iter().take(whole_bytes).any(|&byte| byte != 0) {
        return false;
    }
    if count % 2 == 1 {
        // An odd count needs the high nibble of the next byte to be zero too.
        return digest.get(whole_bytes).is_some_and(|&byte| byte >> 4 == 0);
    }
    true
}

/// Stooq's CSV sentinel for "no data available for this symbol".
const NO_DATA_SENTINEL: &str = "N/D";

fn parse_quote(csv: &str) -> Result<Option<Quote>> {
    if !is_csv(csv) {
        return Err(anyhow!(
            "Stooq returned a non-CSV response (likely an anti-bot challenge page)"
        ));
    }
    let columns: Vec<&str> = csv
        .lines()
        .nth(1)
        .ok_or_else(|| anyhow!("missing data row"))?
        .split(',')
        .collect();
    let close = columns
        .get(CSV_CLOSE_COLUMN_INDEX)
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
    // Observation date (MKT-117). Validation/fallback is the dispatcher's job
    // (MKT-118), so this only forwards the raw value, dropping an empty/N/D cell.
    let date = columns
        .get(CSV_DATE_COLUMN_INDEX)
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty() && *cell != NO_DATA_SENTINEL)
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
    fn parses_close_and_date_from_well_formed_csv() {
        let csv = "Symbol,Date,Time,Open,High,Low,Close,Volume\n\
                   AAPL.US,2026-05-16,21:55:00,189.50,190.20,188.75,189.95,12345678";
        let quote = parse_quote(csv).unwrap().expect("a usable quote");
        assert_eq!(quote.price, 189_950_000);
        assert_eq!(quote.date.as_deref(), Some("2026-05-16"));
    }

    #[test]
    fn rejects_missing_data_row() {
        let csv = "Symbol,Date,Time,Open,High,Low,Close,Volume\n";
        assert!(parse_quote(csv).is_err());
    }

    // Stooq returns the N/D sentinel for symbols it does not recognize. This is a
    // quiet "no data" outcome, not a parse failure — the dispatcher logs at debug
    // level and continues. See `PriceProvider::fetch_price` doc.
    #[test]
    fn returns_ok_none_when_close_is_no_data_sentinel() {
        let csv = "Symbol,Date,Time,Open,High,Low,Close,Volume\n\
                   FR0000120073,N/D,N/D,N/D,N/D,N/D,N/D,N/D";
        let result = parse_quote(csv).unwrap();
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
        let error = parse_quote(challenge).expect_err("anti-bot challenge page must be rejected");
        assert!(
            error.to_string().contains("non-CSV"),
            "expected a non-CSV challenge error, got: {error}"
        );
    }

    #[test]
    fn rejects_non_numeric_close() {
        let csv = "Symbol,Date,Time,Open,High,Low,Close,Volume\n\
                   AAPL.US,2026-05-16,21:55:00,189.50,190.20,188.75,bogus,0";
        assert!(parse_quote(csv).is_err());
    }

    #[test]
    fn rejects_non_positive_close() {
        let csv = "Symbol,Date,Time,Open,High,Low,Close,Volume\n\
                   AAPL.US,2026-05-16,21:55:00,0,0,0,0,0";
        assert!(parse_quote(csv).is_err());
    }

    // A real challenge page embeds the proof-of-work parameters as
    // `const c="<token>",d=<difficulty>,…` inside the verification script.
    #[test]
    fn parse_challenge_extracts_token_and_difficulty() {
        let page = "<!DOCTYPE html><html><body><script nonce=\"x\">(async()=>{\
                    const c=\"AAAAAGokdPX2wj0knuoQ\",d=4,t=\"0\".repeat(d),e=new TextEncoder;\
                    let n=0;})()</script></body></html>";
        let challenge = parse_challenge(page).expect("challenge must parse");
        assert_eq!(challenge.token, "AAAAAGokdPX2wj0knuoQ");
        assert_eq!(challenge.difficulty, 4);
    }

    #[test]
    fn parse_challenge_returns_none_for_genuine_csv() {
        let csv = "Symbol,Date,Time,Open,High,Low,Close,Volume\n\
                   AAPL.US,2026-05-16,21:55:00,189.50,190.20,188.75,189.95,12345678";
        assert!(parse_challenge(csv).is_none());
    }

    #[test]
    fn parse_challenge_rejects_excessive_difficulty() {
        let page = "<script>const c=\"AAAA\",d=9,t=\"0\".repeat(d);</script>";
        assert!(parse_challenge(page).is_none());
    }

    #[test]
    fn parse_challenge_rejects_oversized_token() {
        let token = "A".repeat(MAX_TOKEN_LEN + 1);
        let page = format!("<script>const c=\"{token}\",d=4,t=\"0\".repeat(d);</script>");
        assert!(parse_challenge(&page).is_none());
    }

    #[test]
    fn solve_proof_of_work_finds_a_valid_nonce() {
        let token = "stooq-test-challenge";
        let difficulty = 2;
        let nonce = solve_proof_of_work(token, difficulty).expect("solvable at low difficulty");
        // Re-hash independently and assert the hex digest leads with the zeros.
        let digest = Sha256::digest(format!("{token}{nonce}").as_bytes());
        let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        assert!(
            hex.starts_with("00"),
            "nonce {nonce} produced digest {hex}, expected 2 leading zeros"
        );
    }

    #[test]
    fn has_leading_hex_zeros_handles_even_and_odd_counts() {
        // Two zero bytes == four leading hex zeros.
        assert!(has_leading_hex_zeros(&[0x00, 0x00, 0x12], 4));
        // Five hex zeros also needs the high nibble of the third byte to be 0.
        assert!(!has_leading_hex_zeros(&[0x00, 0x00, 0x12], 5));
        assert!(has_leading_hex_zeros(&[0x00, 0x00, 0x0a], 5));
        // A non-zero within the required whole bytes fails.
        assert!(!has_leading_hex_zeros(&[0x00, 0x10, 0x00], 4));
        // Zero difficulty is vacuously satisfied.
        assert!(has_leading_hex_zeros(&[0xff], 0));
    }
}
