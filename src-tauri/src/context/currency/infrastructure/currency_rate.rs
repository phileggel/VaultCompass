use crate::context::currency::domain::{CurrencyRate, CurrencyRateRepository, CurrencyRateSource};
use crate::core::logger::BACKEND;
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::SqlitePool;
use std::str::FromStr;

/// SQLite-backed implementation of `CurrencyRateRepository`.
pub struct SqliteCurrencyRateRepository {
    pool: SqlitePool,
}

impl SqliteCurrencyRateRepository {
    /// Creates a new repository backed by the given connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct RateRow {
    from_currency: String,
    to_currency: String,
    date: String,
    rate: i64,
    source: String,
}

impl From<RateRow> for CurrencyRate {
    fn from(row: RateRow) -> Self {
        let source = CurrencyRateSource::from_str(&row.source).unwrap_or_else(|_| {
            tracing::warn!(
                target: BACKEND,
                value = %row.source,
                "unknown currency_rates.source value, falling back to Manual"
            );
            CurrencyRateSource::Manual
        });
        CurrencyRate::from_storage(
            row.from_currency,
            row.to_currency,
            row.date,
            row.rate,
            source,
        )
    }
}

#[async_trait]
impl CurrencyRateRepository for SqliteCurrencyRateRepository {
    async fn upsert_rate(&self, rate: CurrencyRate) -> Result<CurrencyRate> {
        let source = rate.source.to_string();
        sqlx::query!(
            "INSERT INTO currency_rates (from_currency, to_currency, date, rate, source)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(from_currency, to_currency, date)
             DO UPDATE SET rate = excluded.rate, source = excluded.source",
            rate.from_currency,
            rate.to_currency,
            rate.date,
            rate.rate,
            source,
        )
        .execute(&self.pool)
        .await
        .context("Failed to upsert currency rate")?;
        Ok(rate)
    }

    async fn delete_rate(&self, from_currency: &str, to_currency: &str, date: &str) -> Result<()> {
        sqlx::query!(
            "DELETE FROM currency_rates
             WHERE from_currency = ? AND to_currency = ? AND date = ?",
            from_currency,
            to_currency,
            date,
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete currency rate")?;
        Ok(())
    }

    async fn list_rates_for_pair(
        &self,
        from_currency: &str,
        to_currency: &str,
    ) -> Result<Vec<CurrencyRate>> {
        let rows = sqlx::query_as!(
            RateRow,
            "SELECT from_currency, to_currency, date, rate, source FROM currency_rates
             WHERE from_currency = ? AND to_currency = ?
             ORDER BY date DESC",
            from_currency,
            to_currency,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list currency rates for pair")?;

        Ok(rows.into_iter().map(CurrencyRate::from).collect())
    }

    async fn get_by_key(
        &self,
        from_currency: &str,
        to_currency: &str,
        date: &str,
    ) -> Result<Option<CurrencyRate>> {
        let row = sqlx::query_as!(
            RateRow,
            "SELECT from_currency, to_currency, date, rate, source FROM currency_rates
             WHERE from_currency = ? AND to_currency = ? AND date = ?",
            from_currency,
            to_currency,
            date,
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch currency rate by key")?;

        Ok(row.map(CurrencyRate::from))
    }

    async fn latest_rate_on_or_before(
        &self,
        from_currency: &str,
        to_currency: &str,
        as_of: &str,
    ) -> Result<Option<CurrencyRate>> {
        let row = sqlx::query_as!(
            RateRow,
            "SELECT from_currency, to_currency, date, rate, source FROM currency_rates
             WHERE from_currency = ? AND to_currency = ? AND date <= ?
             ORDER BY date DESC
             LIMIT 1",
            from_currency,
            to_currency,
            as_of,
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch latest currency rate on or before date")?;

        Ok(row.map(CurrencyRate::from))
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
        // Seed the parent pair so the currency_rates FK is satisfied.
        sqlx::query!(
            "INSERT INTO currency_pairs (from_currency, to_currency) VALUES ('USD', 'EUR')"
        )
        .execute(&pool)
        .await
        .expect("seed pair");
        pool
    }

    fn rate(date: &str, micros: i64, source: CurrencyRateSource) -> CurrencyRate {
        CurrencyRate::from_storage("USD".into(), "EUR".into(), date.into(), micros, source)
    }

    // FXR-025 — upsert_rate inserts a new row, read back via get_by_key
    #[tokio::test]
    async fn upsert_rate_inserts_and_reads_back() {
        let repo = SqliteCurrencyRateRepository::new(setup_pool().await);
        repo.upsert_rate(rate("2026-01-01", 920_000, CurrencyRateSource::Manual))
            .await
            .unwrap();

        let got = repo
            .get_by_key("USD", "EUR", "2026-01-01")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.rate, 920_000);
        assert_eq!(got.source, CurrencyRateSource::Manual);
    }

    // FXR-025/ADR-012 — upsert_rate overwrites by (from, to, date) regardless of source
    #[tokio::test]
    async fn upsert_rate_overwrites_same_key() {
        let repo = SqliteCurrencyRateRepository::new(setup_pool().await);
        repo.upsert_rate(rate("2026-01-01", 920_000, CurrencyRateSource::Frankfurter))
            .await
            .unwrap();
        repo.upsert_rate(rate("2026-01-01", 950_000, CurrencyRateSource::Manual))
            .await
            .unwrap();

        let got = repo
            .get_by_key("USD", "EUR", "2026-01-01")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.rate, 950_000);
        assert_eq!(got.source, CurrencyRateSource::Manual);
        let all = repo.list_rates_for_pair("USD", "EUR").await.unwrap();
        assert_eq!(all.len(), 1, "overwrite, not append");
    }

    // FXR-053 — delete_rate removes the row
    #[tokio::test]
    async fn delete_rate_removes_row() {
        let repo = SqliteCurrencyRateRepository::new(setup_pool().await);
        repo.upsert_rate(rate("2026-01-01", 920_000, CurrencyRateSource::Manual))
            .await
            .unwrap();
        repo.delete_rate("USD", "EUR", "2026-01-01").await.unwrap();

        assert!(repo
            .get_by_key("USD", "EUR", "2026-01-01")
            .await
            .unwrap()
            .is_none());
    }

    // delete_rate is a no-op (no error) when the row is absent
    #[tokio::test]
    async fn delete_rate_noop_when_absent() {
        let repo = SqliteCurrencyRateRepository::new(setup_pool().await);
        assert!(repo.delete_rate("USD", "EUR", "2026-01-01").await.is_ok());
    }

    // FXR-050 — list_rates_for_pair returns rows ordered by date descending
    #[tokio::test]
    async fn list_rates_for_pair_orders_date_desc() {
        let repo = SqliteCurrencyRateRepository::new(setup_pool().await);
        repo.upsert_rate(rate("2026-01-01", 910_000, CurrencyRateSource::Manual))
            .await
            .unwrap();
        repo.upsert_rate(rate("2026-01-03", 930_000, CurrencyRateSource::Manual))
            .await
            .unwrap();
        repo.upsert_rate(rate("2026-01-02", 920_000, CurrencyRateSource::Manual))
            .await
            .unwrap();

        let rows = repo.list_rates_for_pair("USD", "EUR").await.unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2026-01-03");
        assert_eq!(rows[1].date, "2026-01-02");
        assert_eq!(rows[2].date, "2026-01-01");
    }

    // FXR-050 — list_rates_for_pair returns an empty list for an unknown pair
    #[tokio::test]
    async fn list_rates_for_pair_empty_for_unknown() {
        let repo = SqliteCurrencyRateRepository::new(setup_pool().await);
        let rows = repo.list_rates_for_pair("JPY", "EUR").await.unwrap();
        assert!(rows.is_empty());
    }

    // get_by_key returns None when the row does not exist
    #[tokio::test]
    async fn get_by_key_none_when_absent() {
        let repo = SqliteCurrencyRateRepository::new(setup_pool().await);
        assert!(repo
            .get_by_key("USD", "EUR", "2026-01-01")
            .await
            .unwrap()
            .is_none());
    }

    // -------------------------------------------------------------------------
    // latest_rate_on_or_before — FXR-035
    // -------------------------------------------------------------------------

    // FXR-035 — returns the most-recent rate whose date is ≤ as_of when multiple
    // rates exist across different dates
    #[tokio::test]
    async fn latest_rate_on_or_before_returns_most_recent_le_as_of() {
        let repo = SqliteCurrencyRateRepository::new(setup_pool().await);
        // Seed three rates: 2026-01-01, 2026-01-10, 2026-01-20
        repo.upsert_rate(rate("2026-01-01", 900_000, CurrencyRateSource::Manual))
            .await
            .unwrap();
        repo.upsert_rate(rate("2026-01-10", 920_000, CurrencyRateSource::Manual))
            .await
            .unwrap();
        repo.upsert_rate(rate("2026-01-20", 950_000, CurrencyRateSource::Manual))
            .await
            .unwrap();

        // as_of = 2026-01-15: the 2026-01-10 row is the latest ≤ 2026-01-15
        let got = repo
            .latest_rate_on_or_before("USD", "EUR", "2026-01-15")
            .await
            .unwrap()
            .expect("should find a rate");
        assert_eq!(got.date, "2026-01-10");
        assert_eq!(got.rate, 920_000);
    }

    // FXR-035 — returns None when all seeded rates are strictly AFTER as_of
    #[tokio::test]
    async fn latest_rate_on_or_before_returns_none_when_all_rates_are_future() {
        let repo = SqliteCurrencyRateRepository::new(setup_pool().await);
        // All rates dated after the query date
        repo.upsert_rate(rate("2026-06-01", 920_000, CurrencyRateSource::Manual))
            .await
            .unwrap();
        repo.upsert_rate(rate("2026-07-01", 930_000, CurrencyRateSource::Manual))
            .await
            .unwrap();

        let got = repo
            .latest_rate_on_or_before("USD", "EUR", "2026-01-01")
            .await
            .unwrap();
        assert!(
            got.is_none(),
            "expected None when all rates are after as_of"
        );
    }

    // FXR-035 — returns None when the pair has no rates at all
    #[tokio::test]
    async fn latest_rate_on_or_before_returns_none_when_pair_has_no_rates() {
        let repo = SqliteCurrencyRateRepository::new(setup_pool().await);
        // No rates inserted at all

        let got = repo
            .latest_rate_on_or_before("USD", "EUR", "2026-06-01")
            .await
            .unwrap();
        assert!(
            got.is_none(),
            "expected None when the pair has no rates at all"
        );
    }

    // FXR-035 — exact-date match: a rate dated exactly as_of is returned
    #[tokio::test]
    async fn latest_rate_on_or_before_returns_exact_date_match() {
        let repo = SqliteCurrencyRateRepository::new(setup_pool().await);
        repo.upsert_rate(rate("2026-05-15", 940_000, CurrencyRateSource::Manual))
            .await
            .unwrap();

        let got = repo
            .latest_rate_on_or_before("USD", "EUR", "2026-05-15")
            .await
            .unwrap()
            .expect("exact-date match should be returned");
        assert_eq!(got.date, "2026-05-15");
        assert_eq!(got.rate, 940_000);
    }
}
