//! Asset Web Lookup orchestrator — routes queries to OpenFIGI and maps results
//! to `AssetLookupResult` value objects (WEB-014, WEB-022, WEB-023, WEB-024, WEB-046, WEB-048, WEB-049).

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
    /// Short exchange identifier returned by OpenFIGI (`exchCode` field, e.g. "UW" = NASDAQ, "PA" = Euronext Paris).
    /// Resolved to a human-readable name by `map_exchange_code` before being exposed as `AssetLookupResult::exchange`.
    pub exchange_code: Option<String>,
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

/// Transient value object returned by `lookup_asset`.  Never persisted
/// (WEB-020). Fields may be absent per WEB-023, WEB-024, WEB-046, WEB-049.
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
    /// Human-readable exchange name resolved from `exchCode` (WEB-049). Absent if OpenFIGI returns no exchange code.
    pub exchange: Option<String>,
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
        let trimmed = query.trim();
        let is_isin = trimmed.len() == 12 && trimmed.chars().all(|c| c.is_ascii_alphanumeric());

        let raw_hits = if is_isin {
            self.client.map_isin(trimmed).await?
        } else {
            self.client.search_keyword(trimmed).await?
        };

        // Map all hits first, then sort by priority (WEB-048), then truncate (WEB-022).
        let mut results: Vec<AssetLookupResult> = raw_hits
            .into_iter()
            .map(|hit| {
                let reference = if is_isin {
                    Some(trimmed.to_string())
                } else {
                    hit.ticker.filter(|t| !t.is_empty())
                };
                let asset_class = hit.security_type.as_deref().and_then(map_security_type);
                let exchange = hit.exchange_code.map(|c| map_exchange_code(&c));
                AssetLookupResult {
                    name: hit.name,
                    reference,
                    currency: hit.currency,
                    asset_class,
                    exchange,
                }
            })
            .collect();

        results.sort_by_key(|r| sort_priority(&r.asset_class));
        results.truncate(10);

        Ok(results)
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Maps an OpenFIGI `securityType` string to an `AssetClass` variant (WEB-023).
/// Returns `None` for unrecognised types (e.g. "Structured Product", "Certificate").
fn map_security_type(s: &str) -> Option<AssetClass> {
    match s {
        "Common Stock" => Some(AssetClass::Stocks),
        "ETF" => Some(AssetClass::ETF),
        "Mutual Fund" => Some(AssetClass::MutualFunds),
        "Corporate Bond" | "Government Bond" => Some(AssetClass::Bonds),
        "Cryptocurrency" | "Digital Currency" => Some(AssetClass::DigitalAsset),
        "REIT" | "Real Estate Investment Trust" => Some(AssetClass::RealEstate),
        "Cash" => Some(AssetClass::Cash),
        "Warrant" | "Option" | "Future" | "Rights" => Some(AssetClass::Derivatives),
        _ => None,
    }
}

/// Sort key for WEB-048 priority ordering.
/// 0 = known non-Derivative class, 1 = Derivatives, 2 = absent (unknown type).
fn sort_priority(asset_class: &Option<AssetClass>) -> u8 {
    match asset_class {
        Some(AssetClass::Derivatives) => 1,
        Some(_) => 0,
        None => 2,
    }
}

/// Resolves an OpenFIGI `exchCode` to a human-readable market name (WEB-049).
/// Falls back to the raw code string for unknown exchanges.
fn map_exchange_code(code: &str) -> String {
    let resolved = match code {
        "PA" => "Euronext Paris",
        "UN" => "NYSE",
        "UW" => "NASDAQ",
        "UA" => "NYSE MKT",
        "UP" => "OTC Pink Sheets",
        "LN" => "London Stock Exchange",
        "GY" => "Deutsche Börse XETRA",
        "SW" => "SIX Swiss Exchange",
        "FP" => "Euronext Paris", // same venue as "PA" — Bloomberg ticker suffix vs. OpenFIGI exchCode
        "NA" => "Euronext Amsterdam",
        "BB" => "Euronext Brussels",
        "ID" => "Euronext Dublin",
        "LS" => "Euronext Lisbon",
        "IM" => "Borsa Italiana",
        "SM" => "Bolsa de Madrid",
        "HK" => "Hong Kong Stock Exchange",
        "JP" => "Tokyo Stock Exchange",
        "AU" => "Australian Securities Exchange",
        "CN" => "Canadian Securities Exchange",
        "TO" => "Toronto Stock Exchange",
        _ => code,
    };
    resolved.to_string()
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
    #[serde(rename = "exchCode")]
    exchange_code: Option<String>,
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
        exchange_code: h.exchange_code,
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
            exchange_code: None,
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
            exchange_code: None,
        }
    }

    fn raw_hit_with_exchange(
        name: &str,
        security_type: Option<&str>,
        exchange_code: Option<&str>,
    ) -> RawFigiHit {
        RawFigiHit {
            name: name.to_string(),
            ticker: None,
            security_type: security_type.map(str::to_string),
            currency: None,
            exchange_code: exchange_code.map(str::to_string),
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

    // ------------------------------------------------------------------
    // WEB-023 — Derivatives mapping
    // ------------------------------------------------------------------

    /// "Warrant" maps to `AssetClass::Derivatives` (WEB-023).
    #[tokio::test]
    async fn maps_security_type_warrant_to_derivatives() {
        let hit = raw_hit_full(
            "Air Liquide Warrant",
            Some("AWRNT"),
            Some("Warrant"),
            Some("EUR"),
        );
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("air liquide".to_string()).await.unwrap();
        assert_eq!(results[0].asset_class, Some(AssetClass::Derivatives));
    }

    /// "Option", "Future", and "Rights" also map to `AssetClass::Derivatives` (WEB-023).
    #[tokio::test]
    async fn maps_option_future_rights_to_derivatives() {
        assert_eq!(map_security_type("Option"), Some(AssetClass::Derivatives));
        assert_eq!(map_security_type("Future"), Some(AssetClass::Derivatives));
        assert_eq!(map_security_type("Rights"), Some(AssetClass::Derivatives));
    }

    // ------------------------------------------------------------------
    // WEB-048 — sort by priority before truncation
    // ------------------------------------------------------------------

    /// Results are sorted: known non-Derivative classes first (priority 0),
    /// Derivatives second (priority 1), unknown/absent last (priority 2) (WEB-048).
    /// The sort happens before the 10-item truncation so high-priority results
    /// survive the cut.
    #[tokio::test]
    async fn sorts_results_by_priority_before_truncation() {
        // Build 15 hits: 5 unknown, 5 Derivatives, 5 Stocks — expect Stocks first
        let mut hits: Vec<RawFigiHit> = Vec::new();
        for i in 0..5 {
            hits.push(raw_hit_full(
                &format!("Unknown {i}"),
                None,
                Some("Structured Product"),
                None,
            ));
        }
        for i in 0..5 {
            hits.push(raw_hit_full(
                &format!("Warrant {i}"),
                None,
                Some("Warrant"),
                None,
            ));
        }
        for i in 0..5 {
            hits.push(raw_hit_full(
                &format!("Stock {i}"),
                Some(&format!("STK{i}")),
                Some("Common Stock"),
                Some("EUR"),
            ));
        }

        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(hits.clone()));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("test".to_string()).await.unwrap();
        assert_eq!(results.len(), 10);
        // First 5 should be Stocks (priority 0)
        for r in &results[..5] {
            assert_eq!(
                r.asset_class,
                Some(AssetClass::Stocks),
                "expected Stocks in top 5, got {:?}",
                r.asset_class
            );
        }
        // Next 5 should be Derivatives (priority 1)
        for r in &results[5..] {
            assert_eq!(
                r.asset_class,
                Some(AssetClass::Derivatives),
                "expected Derivatives in slots 6–10, got {:?}",
                r.asset_class
            );
        }
    }

    // ------------------------------------------------------------------
    // WEB-049 — exchange code resolution
    // ------------------------------------------------------------------

    /// Known exchange codes are resolved to human-readable names (WEB-049).
    #[tokio::test]
    async fn resolves_known_exchange_code_to_readable_name() {
        let hit = raw_hit_with_exchange("Air Liquide", Some("Common Stock"), Some("PA"));
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("air liquide".to_string()).await.unwrap();
        assert_eq!(results[0].exchange, Some("Euronext Paris".to_string()));
    }

    /// Unknown exchange codes fall back to the raw code string (WEB-049).
    #[tokio::test]
    async fn falls_back_to_raw_code_for_unknown_exchange() {
        let hit = raw_hit_with_exchange("Mystery Corp", Some("Common Stock"), Some("XY"));
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("mystery".to_string()).await.unwrap();
        assert_eq!(results[0].exchange, Some("XY".to_string()));
    }

    /// When OpenFIGI returns no `exchCode`, `exchange` is `None` (WEB-049).
    #[tokio::test]
    async fn exchange_absent_when_no_exchange_code() {
        let hit = raw_hit_with_exchange("No Exchange Corp", Some("Common Stock"), None);
        let mut mock = MockOpenFigiClient::new();
        mock.expect_search_keyword()
            .times(1)
            .returning(move |_| Ok(vec![hit.clone()]));

        let uc = AssetWebLookupUseCase::new(Arc::new(mock));
        let results = uc.search("test".to_string()).await.unwrap();
        assert_eq!(results[0].exchange, None);
    }
}
