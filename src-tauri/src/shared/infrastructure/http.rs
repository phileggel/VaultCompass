//! Outbound HTTP response helpers shared across bounded contexts.

use anyhow::{bail, Context, Result};

/// Upper bound on a provider response body. The largest body any provider
/// returns (the ECB daily XML, ~30 currencies) is a few KiB; 64 KiB leaves
/// generous headroom while bounding memory growth from a malicious or
/// malfunctioning server.
const MAX_BODY_BYTES: usize = 64 * 1024;

/// Reads a response body as UTF-8 text, streaming it in chunks and failing fast
/// once the accumulated size exceeds [`MAX_BODY_BYTES`].
///
/// Unlike [`reqwest::Response::text`], this never buffers an unbounded body: the
/// per-request timeout alone does not bound memory, since a slow drip-feed can
/// grow the buffer until the timeout fires.
pub async fn read_capped_text(mut response: reqwest::Response) -> Result<String> {
    let mut buffer: Vec<u8> = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .context("response chunk read failed")?
    {
        ensure_within_cap(buffer.len(), chunk.len())?;
        buffer.extend_from_slice(&chunk);
    }
    String::from_utf8(buffer).context("response body was not valid UTF-8")
}

/// Fails when appending `incoming_len` bytes to a buffer already holding
/// `current_len` would exceed [`MAX_BODY_BYTES`]. The boundary is inclusive:
/// a total exactly equal to the cap is allowed.
fn ensure_within_cap(current_len: usize, incoming_len: usize) -> Result<()> {
    if current_len.saturating_add(incoming_len) > MAX_BODY_BYTES {
        bail!("response body exceeded the {MAX_BODY_BYTES}-byte cap");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_chunk_well_under_the_cap() {
        assert!(ensure_within_cap(0, 100).is_ok());
    }

    #[test]
    fn accepts_a_total_exactly_at_the_cap() {
        assert!(ensure_within_cap(MAX_BODY_BYTES - 1, 1).is_ok());
    }

    #[test]
    fn rejects_a_total_one_byte_over_the_cap() {
        assert!(ensure_within_cap(MAX_BODY_BYTES - 1, 2).is_err());
        assert!(ensure_within_cap(MAX_BODY_BYTES, 1).is_err());
    }

    #[test]
    fn saturates_instead_of_overflowing_on_huge_lengths() {
        assert!(ensure_within_cap(usize::MAX, 1).is_err());
    }
}
