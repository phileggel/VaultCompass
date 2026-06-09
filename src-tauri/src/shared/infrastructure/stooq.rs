//! Shared Stooq HTTP plumbing reused across bounded contexts.
//!
//! Stooq gates its endpoints behind a JavaScript proof-of-work browser-verification
//! challenge (L-005): the first request of a session returns an HTML page carrying
//! a challenge token and difficulty; the client must find a nonce whose
//! `SHA-256(token + nonce)` hex digest starts with `difficulty` zeros, POST it to
//! the verification endpoint, and retry. The verification sets an `auth` cookie, so
//! with a cookie store the challenge is solved once per session and reused.
//!
//! A 2026-06-08 live probe established that this gate sits in front of the keyed
//! `q/d/l/` download endpoint too — a BYOK apikey does **not** bypass it. So both
//! the asset price fetcher and the connection key-probe must clear this gate; they
//! share [`StooqGate`] rather than each carrying a copy of the solver.

use crate::shared::infrastructure::http::read_capped_text_with_limit;
use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};
use std::time::Duration;

/// Endpoint that clears the proof-of-work challenge: a successful POST sets the
/// `auth` cookie the data endpoints then honour.
const STOOQ_VERIFY_URL: &str = "https://stooq.com/__verify";
const REQUEST_TIMEOUT_SECS: u64 = 15;

/// Upper bound on a Stooq response body. The daily-download endpoint returns a
/// symbol's full price history (hundreds of KiB for decades of data), far above
/// the shared 64 KiB default; 8 MiB bounds a malicious/runaway body while holding
/// any realistic single-symbol history.
const STOOQ_MAX_BODY_BYTES: usize = 8 * 1024 * 1024;

/// Browser-like `User-Agent` sent on every Stooq request.
const STOOQ_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// Upper bound on the proof-of-work difficulty we will attempt. The token and
/// difficulty come from an untrusted server response; observed difficulty is 4,
/// so 5 leaves headroom while refusing an adversarial value.
const MAX_POW_DIFFICULTY: usize = 5;
/// Hard ceiling on proof-of-work attempts, guaranteeing termination. At difficulty
/// 5 expected work is ~1M iterations, so 50M makes a genuine miss astronomically
/// unlikely.
const MAX_POW_ITERATIONS: u64 = 50_000_000;
/// Upper bound on the challenge token length. Real tokens are ~60 chars; this caps
/// the per-iteration allocation against a hostile multi-KiB token.
const MAX_TOKEN_LEN: usize = 256;

/// Shared client that clears Stooq's proof-of-work gate and returns response
/// bodies. Construct once and reuse: the cookie store carries the `auth` cookie so
/// the challenge is solved at most once per session.
pub struct StooqGate {
    client: reqwest::Client,
}

impl Default for StooqGate {
    fn default() -> Self {
        Self::new()
    }
}

impl StooqGate {
    /// Creates a gate with a per-request timeout, browser `User-Agent`, and a
    /// cookie store (so the proof-of-work `auth` cookie persists across requests).
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .user_agent(STOOQ_USER_AGENT)
            .cookie_store(true)
            .build()
            // reviewer-backend FP: reqwest's static-config build is effectively
            // infallible, `.expect` matches every other HTTP client (frankfurter,
            // ecb), and clippy's deny-set permits `expect_used`.
            .expect("reqwest client build");
        Self { client }
    }

    /// Fetches `url`, transparently solving the proof-of-work challenge once if the
    /// gate serves it, and returns the final response body. `label` describes the
    /// caller for error context only — it never carries a secret (KEY-014).
    pub async fn get_text(&self, url: &str, label: &str) -> Result<String> {
        let body = self.fetch_body(url, label).await?;
        // A challenge page rather than the expected payload: solve it, then retry
        // once — the cookie store now carries the `auth` cookie.
        if let Some(challenge) = parse_challenge(&body) {
            self.clear_challenge(challenge).await?;
            return self.fetch_body(url, label).await;
        }
        Ok(body)
    }

    async fn fetch_body(&self, url: &str, label: &str) -> Result<String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            // Drop the reqwest cause: its Display can include the request URL,
            // which carries the apikey as a query param (KEY-014). The label is
            // the symbol only.
            .map_err(|_| anyhow!("Stooq request failed ({label})"))?;
        if !response.status().is_success() {
            anyhow::bail!("Stooq returned {} ({label})", response.status());
        }
        read_capped_text_with_limit(response, STOOQ_MAX_BODY_BYTES)
            .await
            .with_context(|| format!("Stooq response read failed ({label})"))
    }

    async fn clear_challenge(&self, challenge: Challenge) -> Result<()> {
        // The solve is CPU-bound; run it off the async executor thread.
        let (token, nonce) = tokio::task::spawn_blocking(move || {
            solve_proof_of_work(&challenge.token, challenge.difficulty)
                .map(|nonce| (challenge.token, nonce.to_string()))
        })
        .await
        .context("Stooq proof-of-work task panicked")?
        .ok_or_else(|| anyhow!("Stooq proof-of-work exceeded the iteration ceiling"))?;

        let response = self
            .client
            .post(STOOQ_VERIFY_URL)
            .form(&[("c", token.as_str()), ("n", nonce.as_str())])
            .send()
            .await
            .context("Stooq challenge verification request failed")?;
        if !response.status().is_success() {
            anyhow::bail!(
                "Stooq challenge verification returned {}",
                response.status()
            );
        }
        // Drain the body so the connection returns to the pool.
        read_capped_text_with_limit(response, STOOQ_MAX_BODY_BYTES)
            .await
            .context("Stooq challenge verification response read failed")?;
        Ok(())
    }
}

/// A Stooq proof-of-work challenge: find a nonce whose `SHA-256(token + nonce)`
/// hex digest starts with `difficulty` zeros.
struct Challenge {
    token: String,
    difficulty: usize,
}

/// Extracts the proof-of-work parameters from a Stooq challenge page, or `None`
/// when `body` carries no recognizable challenge (e.g. it is the real payload).
/// The page embeds them as `…const c="<token>",d=<difficulty>,…`.
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

/// Brute-forces the smallest nonce `n` such that the hex SHA-256 of `{token}{n}`
/// begins with `difficulty` zero digits. Returns `None` if no nonce is found
/// within [`MAX_POW_ITERATIONS`], guaranteeing termination.
fn solve_proof_of_work(token: &str, difficulty: usize) -> Option<u64> {
    (0..MAX_POW_ITERATIONS).find(|nonce| {
        let digest = Sha256::digest(format!("{token}{nonce}").as_bytes());
        has_leading_hex_zeros(digest.as_slice(), difficulty)
    })
}

/// True when the hex representation of `digest` starts with `count` `0` digits,
/// tested on the bytes to avoid allocating a hex string per attempt.
fn has_leading_hex_zeros(digest: &[u8], count: usize) -> bool {
    let whole_bytes = count / 2;
    if digest.iter().take(whole_bytes).any(|&byte| byte != 0) {
        return false;
    }
    if count % 2 == 1 {
        return digest.get(whole_bytes).is_some_and(|&byte| byte >> 4 == 0);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let csv = "Date,Open,High,Low,Close,Volume\n\
                   2026-05-16,189.50,190.20,188.75,189.95,12345678";
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
        let nonce = solve_proof_of_work(token, 2).expect("solvable at low difficulty");
        let digest = Sha256::digest(format!("{token}{nonce}").as_bytes());
        let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        assert!(
            hex.starts_with("00"),
            "nonce {nonce} produced digest {hex}, expected 2 leading zeros"
        );
    }

    #[test]
    fn has_leading_hex_zeros_handles_even_and_odd_counts() {
        assert!(has_leading_hex_zeros(&[0x00, 0x00, 0x12], 4));
        assert!(!has_leading_hex_zeros(&[0x00, 0x00, 0x12], 5));
        assert!(has_leading_hex_zeros(&[0x00, 0x00, 0x0a], 5));
        assert!(!has_leading_hex_zeros(&[0x00, 0x10, 0x00], 4));
        assert!(has_leading_hex_zeros(&[0xff], 0));
    }
}
