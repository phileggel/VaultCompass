use crate::context::currency::domain::{
    CurrencyPair, CurrencyPairRepository, CurrencyPairSummary, CurrencyRateSource,
};
use crate::core::logger::BACKEND;
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::SqlitePool;
use std::str::FromStr;

/// SQLite-backed implementation of `CurrencyPairRepository`.
pub struct SqliteCurrencyPairRepository {
    pool: SqlitePool,
}

impl SqliteCurrencyPairRepository {
    /// Creates a new repository backed by the given connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct PairSummaryRow {
    from_currency: String,
    to_currency: String,
    latest_rate: Option<i64>,
    latest_rate_date: Option<String>,
    latest_rate_source: Option<String>,
}

#[async_trait]
impl CurrencyPairRepository for SqliteCurrencyPairRepository {
    async fn upsert_pair(&self, pair: CurrencyPair) -> Result<CurrencyPair> {
        sqlx::query!(
            "INSERT INTO currency_pairs (from_currency, to_currency) VALUES (?, ?)
             ON CONFLICT(from_currency, to_currency) DO NOTHING",
            pair.from_currency,
            pair.to_currency,
        )
        .execute(&self.pool)
        .await
        .context("Failed to upsert currency pair")?;
        Ok(pair)
    }

    async fn list_pairs_with_latest_rate(&self) -> Result<Vec<CurrencyPairSummary>> {
        let rows = sqlx::query_as!(
            PairSummaryRow,
            r#"SELECT
                 p.from_currency AS "from_currency!: String",
                 p.to_currency   AS "to_currency!: String",
                 r.rate          AS "latest_rate?: i64",
                 r.date          AS "latest_rate_date?: String",
                 r.source        AS "latest_rate_source?: String"
               FROM currency_pairs p
               LEFT JOIN currency_rates r
                 ON r.from_currency = p.from_currency
                AND r.to_currency = p.to_currency
                AND r.date = (
                    SELECT MAX(r2.date) FROM currency_rates r2
                    WHERE r2.from_currency = p.from_currency
                      AND r2.to_currency = p.to_currency
                )
               ORDER BY p.from_currency, p.to_currency"#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list currency pairs with latest rate")?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let latest_rate_source = row.latest_rate_source.map(|s| {
                    CurrencyRateSource::from_str(&s).unwrap_or_else(|_| {
                        tracing::warn!(
                            target: BACKEND,
                            value = %s,
                            "unknown currency_rates.source value, falling back to Manual"
                        );
                        CurrencyRateSource::Manual
                    })
                });
                CurrencyPairSummary {
                    from_currency: row.from_currency,
                    to_currency: row.to_currency,
                    latest_rate: row.latest_rate,
                    latest_rate_date: row.latest_rate_date,
                    latest_rate_source,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test pool");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");
        pool
    }

    /// Inserts a rate row directly so latest-rate resolution can be exercised
    /// without depending on the rate repository.
    async fn insert_rate(pool: &SqlitePool, from: &str, to: &str, date: &str, rate: i64) {
        sqlx::query!(
            "INSERT INTO currency_rates (from_currency, to_currency, date, rate, source)
             VALUES (?, ?, ?, ?, 'Manual')",
            from,
            to,
            date,
            rate,
        )
        .execute(pool)
        .await
        .expect("seed rate");
    }

    // FXR-054 — upsert_pair persists a new pair and returns it
    #[tokio::test]
    async fn upsert_pair_inserts_and_returns() {
        let repo = SqliteCurrencyPairRepository::new(setup_pool().await);
        let pair = repo
            .upsert_pair(CurrencyPair::from_storage("USD".into(), "EUR".into()))
            .await
            .unwrap();
        assert_eq!(pair.from_currency, "USD");
        assert_eq!(pair.to_currency, "EUR");

        let pairs = repo.list_pairs_with_latest_rate().await.unwrap();
        assert_eq!(pairs.len(), 1);
    }

    // FXR-054 — upsert_pair is idempotent: a second insert of the same key is a no-op
    #[tokio::test]
    async fn upsert_pair_is_idempotent() {
        let repo = SqliteCurrencyPairRepository::new(setup_pool().await);
        repo.upsert_pair(CurrencyPair::from_storage("USD".into(), "EUR".into()))
            .await
            .unwrap();
        repo.upsert_pair(CurrencyPair::from_storage("USD".into(), "EUR".into()))
            .await
            .unwrap();

        let pairs = repo.list_pairs_with_latest_rate().await.unwrap();
        assert_eq!(pairs.len(), 1, "no duplicate pair");
    }

    // FXR-051 — list_pairs_with_latest_rate returns an empty list when no pair exists
    #[tokio::test]
    async fn list_pairs_with_latest_rate_empty() {
        let repo = SqliteCurrencyPairRepository::new(setup_pool().await);
        let pairs = repo.list_pairs_with_latest_rate().await.unwrap();
        assert!(pairs.is_empty());
    }

    // FXR-051 — a pair with no recorded rate has None latest_* fields
    #[tokio::test]
    async fn list_pairs_with_latest_rate_pair_without_rate() {
        let repo = SqliteCurrencyPairRepository::new(setup_pool().await);
        repo.upsert_pair(CurrencyPair::from_storage("GBP".into(), "EUR".into()))
            .await
            .unwrap();

        let pairs = repo.list_pairs_with_latest_rate().await.unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].from_currency, "GBP");
        assert!(pairs[0].latest_rate.is_none());
        assert!(pairs[0].latest_rate_date.is_none());
        assert!(pairs[0].latest_rate_source.is_none());
    }

    // FXR-035/051 — latest_* reflect the most recently dated rate for the pair
    #[tokio::test]
    async fn list_pairs_with_latest_rate_resolves_most_recent() {
        let pool = setup_pool().await;
        let repo = SqliteCurrencyPairRepository::new(pool.clone());
        repo.upsert_pair(CurrencyPair::from_storage("USD".into(), "EUR".into()))
            .await
            .unwrap();
        insert_rate(&pool, "USD", "EUR", "2026-01-01", 910_000).await;
        insert_rate(&pool, "USD", "EUR", "2026-01-03", 930_000).await;
        insert_rate(&pool, "USD", "EUR", "2026-01-02", 920_000).await;

        let pairs = repo.list_pairs_with_latest_rate().await.unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].latest_rate, Some(930_000));
        assert_eq!(pairs[0].latest_rate_date.as_deref(), Some("2026-01-03"));
        assert_eq!(
            pairs[0].latest_rate_source,
            Some(CurrencyRateSource::Manual)
        );
    }
}
