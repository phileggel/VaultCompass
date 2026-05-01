//! Asset Web Lookup orchestrator — routes queries to OpenFIGI and maps results
//! to `AssetLookupResult` value objects (WEB-014, WEB-022, WEB-023, WEB-024, WEB-046).

use crate::context::asset::AssetClass;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Raw hit from the OpenFIGI API (private — not exported)
// ---------------------------------------------------------------------------

/// Raw fields extracted from a single OpenFIGI result entry.
#[derive(Debug, Clone)]
pub struct RawFigiHit {
    /// Instrument name as returned by OpenFIGI.
    pub name: String,
    /// Ticker symbol, if present.
    pub ticker: Option<String>,
    /// `securityType` field, if present.
    pub security_type: Option<String>,
    /// ISO 4217 currency, if present.
    pub currency: Option<String>,
}

// ---------------------------------------------------------------------------
// OpenFigiClient trait (allows test mocking per B26)
// ---------------------------------------------------------------------------

/// Abstraction over the OpenFIGI HTTP API.  Concrete production implementation
/// is `ReqwestOpenFigiClient`; tests use hand-rolled mocks.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait OpenFigiClient: Send + Sync {
    /// Map a single ISIN to instruments via the `/v3/mapping` endpoint (WEB-014).
    async fn map_isin(&self, isin: &str) -> Result<Vec<RawFigiHit>>;
    /// Search by keyword via the `/v3/search` endpoint (WEB-014).
    async fn search_keyword(&self, query: &str) -> Result<Vec<RawFigiHit>>;
}

// ---------------------------------------------------------------------------
// AssetLookupResult value object
// ---------------------------------------------------------------------------

/// Transient value object returned by `search_asset_web`.  Never persisted
/// (WEB-020). Fields may be absent per WEB-023, WEB-024, WEB-046.
#[derive(Debug, Clone, Serialize, Type)]
pub struct AssetLookupResult {
    /// Full name of the financial instrument.
    pub name: String,
    /// ISIN (on the ISIN path) or ticker (on the keyword path), if available (WEB-046).
    pub reference: Option<String>,
    /// ISO 4217 trading currency, if returned by OpenFIGI (WEB-024).
    pub currency: Option<String>,
    /// Mapped asset class, if the `securityType` is recognised (WEB-023).
    pub asset_class: Option<AssetClass>,
}

// ---------------------------------------------------------------------------
// AssetWebLookupUseCase
// ---------------------------------------------------------------------------

/// Orchestrates the OpenFIGI lookup: routes the query, maps raw hits, truncates
/// to 10 results (WEB-022), and returns `AssetLookupResult` value objects.
pub struct AssetWebLookupUseCase {
    client: Arc<dyn OpenFigiClient>,
}

impl AssetWebLookupUseCase {
    /// Creates a new use case backed by the provided `OpenFigiClient`.
    pub fn new(client: Arc<dyn OpenFigiClient>) -> Self {
        Self { client }
    }

    /// Searches OpenFIGI for instruments matching `query`.
    ///
    /// Routing rule (WEB-014): exactly 12 ASCII-alphanumeric chars → ISIN path;
    /// everything else → keyword path.
    ///
    /// Results are truncated to 10 (WEB-022).  Any client error is surfaced as
    /// `anyhow::Err` (WEB-025).
    pub async fn search(&self, query: String) -> Result<Vec<AssetLookupResult>> {
        let trimmed = query.trim().to_string();
        let is_isin = trimmed.len() == 12 && trimmed.chars().all(|c| c.is_ascii_alphanumeric());

        let raw_hits = if is_isin {
            self.client.map_isin(&trimmed).await?
        } else {
            self.client.search_keyword(&trimmed).await?
        };

        let results = raw_hits
            .into_iter()
            .take(10)
            .map(|hit| {
                let reference = if is_isin {
                    Some(trimmed.clone())
                } else {
                    hit.ticker.filter(|t| !t.is_empty())
                };
                AssetLookupResult {
                    name: hit.name,
                    reference,
                    currency: hit.currency,
                    asset_class: hit.security_type.as_deref().and_then(map_security_type),
                }
            })
            .collect();

        Ok(results)
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Maps an OpenFIGI `securityType` string to an `AssetClass` variant (WEB-023).
/// Returns `None` for unrecognised types.
fn map_security_type(s: &str) -> Option<AssetClass> {
    match s {
        "Common Stock" => Some(AssetClass::Stocks),
        "ETF" => Some(AssetClass::ETF),
        "Mutual Fund" => Some(AssetClass::MutualFunds),
        "Corporate Bond" | "Government Bond" => Some(AssetClass::Bonds),
        "Cryptocurrency" | "Digital Currency" => Some(AssetClass::DigitalAsset),
        "REIT" | "Real Estate Investment Trust" => Some(AssetClass::RealEstate),
        "Cash" => Some(AssetClass::Cash),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// OpenFIGI HTTP response types (private deserialization structs)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct OpenFigiHit {
    name: String,
    ticker: Option<String>,
    #[serde(rename = "securityType")]
    security_type: Option<String>,
    currency: Option<String>,
}

/// One item in the `/v3/mapping` response array.
#[derive(Deserialize)]
struct MappingResultItem {
    data: Option<Vec<OpenFigiHit>>,
}

/// The `/v3/search` response envelope.
#[derive(Deserialize)]
struct SearchResponse {
    data: Vec<OpenFigiHit>,
}

// ---------------------------------------------------------------------------
// ReqwestOpenFigiClient — production HTTP implementation
// ---------------------------------------------------------------------------

const MAP_URL: &str = "https://api.openfigi.com/v3/mapping";
const SEARCH_URL: &str = "https://api.openfigi.com/v3/search";

/// Production `OpenFigiClient` backed by `reqwest` with rustls (WEB-021 — no API key).
pub struct ReqwestOpenFigiClient {
    client: reqwest::Client,
}

impl Default for ReqwestOpenFigiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ReqwestOpenFigiClient {
    /// Creates a new client using the system's default TLS configuration.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl OpenFigiClient for ReqwestOpenFigiClient {
    async fn map_isin(&self, isin: &str) -> Result<Vec<RawFigiHit>> {
        let body = serde_json::json!([{"idType": "ID_ISIN", "idValue": isin}]);
        let resp = self
            .client
            .post(MAP_URL)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("OpenFIGI ISIN mapping request failed for ISIN: {isin}"))?;

        if !resp.status().is_success() {
            anyhow::bail!("OpenFIGI mapping returned {}", resp.status());
        }

        let items: Vec<MappingResultItem> = resp
            .json()
            .await
            .context("failed to deserialize OpenFIGI mapping response")?;
        Ok(items
            .into_iter()
            .flat_map(|item| item.data.unwrap_or_default())
            .map(hit_to_raw)
            .collect())
    }

    async fn search_keyword(&self, query: &str) -> Result<Vec<RawFigiHit>> {
        let body = serde_json::json!({"query": query});
        let resp = self
            .client
            .post(SEARCH_URL)
            .json(&body)
            .send()
            .await
            .context("OpenFIGI keyword search request failed")?;

        if !resp.status().is_success() {
            anyhow::bail!("OpenFIGI search returned {}", resp.status());
        }

        let search_resp: SearchResponse = resp
            .json()
            .await
            .context("failed to deserialize OpenFIGI search response")?;
        Ok(search_resp.data.into_iter().map(hit_to_raw).collect())
    }
}

fn hit_to_raw(h: OpenFigiHit) -> RawFigiHit {
    RawFigiHit {
        name: h.name,
        ticker: h.ticker,
        security_type: h.security_type,
        currency: h.currency,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::eq;

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// Constructs a minimal `RawFigiHit` with sensible defaults for tests that
    /// do not care about optional fields.
    fn raw_hit(name: &str) -> RawFigiHit {
        RawFigiHit {
            name: name.to_string(),
            ticker: None,
            security_type: None,
            currency: None,
        }
    }

    fn raw_hit_full(
        name: &str,
        ticker: Option<&str>,
        security_type: Option<&str>,
        currency: Option<&str>,
    ) -> RawFigiHit {
        RawFigiHit {
            name: name.to_string(),
            ticker: ticker.map(str::to_string),
            security_type: security_type.map(str::to_string),
            currency: currency.map(str::to_string),
        }
    }

    // ------------------------------------------------------------------
    // WEB-014 routing — ISIN path
    // ------------------------------------------------------------------

    /// A 12-char all-alphanumeric query must be routed to map_isin, not
    /// search_keyword (WEB-014).
    #[tokio::test]
    async fn routes_12_alphanumeric_query_to_map_isin() {
        let isin = "US0378331005"; // exactly 12 alphanumeric chars
        assert_eq!(isin.len(), 12);
        assert!(isin.chars().all(|c| c.is_ascii_alphanumeric()));

        let mut mock = MockOpenFigiClient::new();
        mock.expect_map_isin()
            .with(eq(isin))
            .times(1)
            .returning(|_| Ok(vec![]));
        mock.expect_search_keyword().times(0);

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let result = uc.search(isin.to_string()).await;
        assert!(result.is_ok());
    }

    /// A short query (fewer than 12 chars) must use the keyword path (WEB-014).
    #[tokio::test]
    async fn routes_short_query_to_search_keyword() {
        let query = "AAPL"; // 4 chars — keyword path
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .with(eq(query))
            .times(1)
            .returning(|_| Ok(vec![]));
        mock.expect_map_isin().times(0);

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let result = uc.search(query.to_string()).await;
        assert!(result.is_ok());
    }

    /// A 13-char alphanumeric query (one too long) must use the keyword path
    /// (WEB-014 — wrong length).
    #[tokio::test]
    async fn routes_13_char_alphanumeric_to_search_keyword() {
        let query = "US03783310051"; // 13 chars
        assert_eq!(query.len(), 13);

        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .with(eq(query))
            .times(1)
            .returning(|_| Ok(vec![]));
        mock.expect_map_isin().times(0);

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let result = uc.search(query.to_string()).await;
        assert!(result.is_ok());
    }

    /// A 12-char query that contains a non-alphanumeric character (dash) must
    /// use the keyword path (WEB-014 — non-alphanumeric).
    #[tokio::test]
    async fn routes_query_with_dash_to_search_keyword() {
        let query = "US037833-005"; // 12 chars but contains '-'
        assert_eq!(query.len(), 12);
        assert!(!query.chars().all(|c| c.is_ascii_alphanumeric()));

        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .with(eq(query))
            .times(1)
            .returning(|_| Ok(vec![]));
        mock.expect_map_isin().times(0);

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let result = uc.search(query.to_string()).await;
        assert!(result.is_ok());
    }

    // ------------------------------------------------------------------
    // WEB-022 — result truncation
    // ------------------------------------------------------------------

    /// When OpenFIGI returns more than 10 results, only the first 10 are
    /// forwarded (WEB-022).
    #[tokio::test]
    async fn truncates_results_to_ten() {
        let hits: Vec<RawFigiHit> = (0..15).map(|i| raw_hit(&format!("Fund {i}"))).collect();

        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(hits.clone()));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("fund".to_string()).await.unwrap();
        assert_eq!(
            results.len(),
            10,
            "expected exactly 10 results, got {}",
            results.len()
        );
    }

    // ------------------------------------------------------------------
    // WEB-023 — securityType → AssetClass mapping
    // ------------------------------------------------------------------

    /// "Common Stock" maps to `AssetClass::Stocks` (WEB-023).
    #[tokio::test]
    async fn maps_security_type_common_stock_to_stocks() {
        let hit = raw_hit_full(
            "Apple Inc.",
            Some("AAPL"),
            Some("Common Stock"),
            Some("USD"),
        );
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("AAPL".to_string()).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].asset_class, Some(AssetClass::Stocks));
    }

    /// "ETF" maps to `AssetClass::ETF` (WEB-023).
    #[tokio::test]
    async fn maps_security_type_etf_to_etf() {
        let hit = raw_hit_full("SPDR S&P 500", Some("SPY"), Some("ETF"), Some("USD"));
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("SPY".to_string()).await.unwrap();
        assert_eq!(results[0].asset_class, Some(AssetClass::ETF));
    }

    /// "Mutual Fund" maps to `AssetClass::MutualFunds` (WEB-023).
    #[tokio::test]
    async fn maps_security_type_mutual_fund_to_mutual_funds() {
        let hit = raw_hit_full("Vanguard 500", None, Some("Mutual Fund"), Some("USD"));
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("vanguard".to_string()).await.unwrap();
        assert_eq!(results[0].asset_class, Some(AssetClass::MutualFunds));
    }

    /// "Corporate Bond" maps to `AssetClass::Bonds` (WEB-023).
    #[tokio::test]
    async fn maps_security_type_corporate_bond_to_bonds() {
        let hit = raw_hit_full("Apple Bond 2030", None, Some("Corporate Bond"), Some("USD"));
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("apple bond".to_string()).await.unwrap();
        assert_eq!(results[0].asset_class, Some(AssetClass::Bonds));
    }

    /// "Government Bond" also maps to `AssetClass::Bonds` (WEB-023).
    #[tokio::test]
    async fn maps_security_type_government_bond_to_bonds() {
        let hit = raw_hit_full(
            "US Treasury 10Y",
            None,
            Some("Government Bond"),
            Some("USD"),
        );
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("treasury".to_string()).await.unwrap();
        assert_eq!(results[0].asset_class, Some(AssetClass::Bonds));
    }

    /// "Cryptocurrency" maps to `AssetClass::DigitalAsset` (WEB-023).
    #[tokio::test]
    async fn maps_security_type_cryptocurrency_to_digital_asset() {
        let hit = raw_hit_full("Bitcoin", Some("BTC"), Some("Cryptocurrency"), Some("USD"));
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("bitcoin".to_string()).await.unwrap();
        assert_eq!(results[0].asset_class, Some(AssetClass::DigitalAsset));
    }

    /// "Digital Currency" also maps to `AssetClass::DigitalAsset` (WEB-023).
    #[tokio::test]
    async fn maps_security_type_digital_currency_to_digital_asset() {
        let hit = raw_hit_full(
            "Ethereum",
            Some("ETH"),
            Some("Digital Currency"),
            Some("USD"),
        );
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("ethereum".to_string()).await.unwrap();
        assert_eq!(results[0].asset_class, Some(AssetClass::DigitalAsset));
    }

    /// "REIT" maps to `AssetClass::RealEstate` (WEB-023).
    #[tokio::test]
    async fn maps_security_type_reit_to_real_estate() {
        let hit = raw_hit_full("Prologis", Some("PLD"), Some("REIT"), Some("USD"));
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("prologis".to_string()).await.unwrap();
        assert_eq!(results[0].asset_class, Some(AssetClass::RealEstate));
    }

    /// "Cash" maps to `AssetClass::Cash` (WEB-023).
    #[tokio::test]
    async fn maps_security_type_cash_to_cash() {
        let hit = raw_hit_full("USD Cash", None, Some("Cash"), Some("USD"));
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("usd cash".to_string()).await.unwrap();
        assert_eq!(results[0].asset_class, Some(AssetClass::Cash));
    }

    /// An unrecognised `securityType` string results in `asset_class` being
    /// `None` (WEB-023).
    #[tokio::test]
    async fn maps_security_type_unknown_results_in_none_asset_class() {
        let hit = raw_hit_full("Mystery Instrument", None, Some("ExoticDerivative"), None);
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("mystery".to_string()).await.unwrap();
        assert_eq!(results[0].asset_class, None);
    }

    /// When `securityType` is absent altogether, `asset_class` is `None` (WEB-023).
    #[tokio::test]
    async fn maps_missing_security_type_to_none_asset_class() {
        let hit = raw_hit_full("Unknown Instrument", None, None, Some("EUR"));
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("unknown".to_string()).await.unwrap();
        assert_eq!(results[0].asset_class, None);
    }

    // ------------------------------------------------------------------
    // WEB-024 — currency passthrough
    // ------------------------------------------------------------------

    /// When OpenFIGI returns a currency, it is forwarded unchanged (WEB-024).
    #[tokio::test]
    async fn passes_currency_through_when_present() {
        let hit = raw_hit_full(
            "Apple Inc.",
            Some("AAPL"),
            Some("Common Stock"),
            Some("USD"),
        );
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("AAPL".to_string()).await.unwrap();
        assert_eq!(results[0].currency, Some("USD".to_string()));
    }

    /// When OpenFIGI omits the currency, `currency` is `None` (WEB-024).
    #[tokio::test]
    async fn currency_absent_when_openfigi_omits_it() {
        let hit = raw_hit_full("Some Fund", None, Some("Mutual Fund"), None);
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("some fund".to_string()).await.unwrap();
        assert_eq!(results[0].currency, None);
    }

    // ------------------------------------------------------------------
    // WEB-046 — reference field source
    // ------------------------------------------------------------------

    /// On the ISIN path, `reference` equals the trimmed input ISIN (WEB-046).
    #[tokio::test]
    async fn reference_is_input_isin_on_isin_path() {
        let isin = "US0378331005";
        let hit = raw_hit_full(
            "Apple Inc.",
            Some("AAPL"),
            Some("Common Stock"),
            Some("USD"),
        );
        let mut mock = MockOpenFigiClient::new();
        mock.expect_map_isin()
            .with(eq(isin))
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search(isin.to_string()).await.unwrap();
        assert_eq!(results[0].reference, Some(isin.to_string()));
    }

    /// On the keyword path, when OpenFIGI provides a ticker, `reference` is
    /// that ticker (WEB-046).
    #[tokio::test]
    async fn reference_is_ticker_on_keyword_path_when_present() {
        let hit = raw_hit_full(
            "Apple Inc.",
            Some("AAPL"),
            Some("Common Stock"),
            Some("USD"),
        );
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("apple".to_string()).await.unwrap();
        assert_eq!(results[0].reference, Some("AAPL".to_string()));
    }

    /// On the keyword path, when OpenFIGI returns no ticker, `reference` is
    /// `None` (WEB-046).
    #[tokio::test]
    async fn reference_absent_on_keyword_path_when_no_ticker() {
        let hit = raw_hit_full("Some Mutual Fund", None, Some("Mutual Fund"), Some("EUR"));
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("some fund".to_string()).await.unwrap();
        assert_eq!(results[0].reference, None);
    }

    // ------------------------------------------------------------------
    // WEB-025 — client error propagation
    // ------------------------------------------------------------------

    /// A client error (network failure, non-2xx response, etc.) is surfaced
    /// as an `anyhow::Err` at the orchestrator boundary (WEB-025).
    /// The `api.rs` adapter maps this to `WebLookupCommandError::NetworkError`.
    #[tokio::test]
    async fn propagates_client_error_as_anyhow() {
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(|_| Err(anyhow::anyhow!("connection refused")));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let result = uc.search("AAPL".to_string()).await;
        assert!(result.is_err(), "expected Err, got Ok");
    }
}
