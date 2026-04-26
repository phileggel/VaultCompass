use super::super::domain::{AssetPrice, AssetPriceRepository};
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Pool, Sqlite};

/// SQLite implementation of AssetPriceRepository.
pub struct SqliteAssetPriceRepository {
    pool: Pool<Sqlite>,
}

impl SqliteAssetPriceRepository {
    /// Creates a new repository backed by the given connection pool.
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AssetPriceRepository for SqliteAssetPriceRepository {
    async fn upsert(&self, price: AssetPrice) -> Result<()> {
        sqlx::query!(
            "INSERT INTO asset_prices (asset_id, date, price) VALUES (?, ?, ?)
             ON CONFLICT(asset_id, date) DO UPDATE SET price = excluded.price",
            price.asset_id,
            price.date,
            price.price,
        )
        .execute(&self.pool)
        .await
        .context("Failed to upsert asset price")?;
        Ok(())
    }

    async fn get_latest(&self, asset_id: &str) -> Result<Option<AssetPrice>> {
        let row = sqlx::query!(
            "SELECT asset_id, date, price FROM asset_prices WHERE asset_id = ? ORDER BY date DESC LIMIT 1",
            asset_id,
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch latest asset price")?;

        Ok(row.map(|r| AssetPrice::restore(r.asset_id, r.date, r.price)))
    }
}
