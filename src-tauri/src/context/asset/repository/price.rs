use super::super::domain::{AssetPrice, PriceRepository};
use anyhow::Result;
use sqlx::{FromRow, Pool, Sqlite};

#[derive(Debug, FromRow)]
pub struct AssetPriceRow {
    pub id: String,
    pub asset_id: String,
    pub price: f64,
    pub date: String,
}

impl From<AssetPriceRow> for AssetPrice {
    fn from(row: AssetPriceRow) -> Self {
        AssetPrice::from_storage(row.id, row.asset_id, row.price, row.date)
    }
}

/// SQLite implementation of the PriceRepository.
pub struct SqlitePriceRepository {
    db: Pool<Sqlite>,
}

impl SqlitePriceRepository {
    /// Creates a new SqlitePriceRepository.
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl PriceRepository for SqlitePriceRepository {
    async fn get_by_asset(&self, asset_id: &str) -> Result<Vec<AssetPrice>> {
        let rows = sqlx::query_as!(
            AssetPriceRow,
            r#"
            SELECT id, asset_id, price, date 
            FROM asset_prices 
            WHERE asset_id = ? 
            ORDER BY date DESC
            "#,
            asset_id
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(AssetPrice::from).collect())
    }

    async fn get_by_asset_and_date_range(
        &self,
        asset_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<AssetPrice>> {
        let rows = sqlx::query_as!(
            AssetPriceRow,
            r#"
            SELECT id, asset_id, price, date 
            FROM asset_prices 
            WHERE asset_id = ? AND date >= ? AND date <= ? 
            ORDER BY date ASC
            "#,
            asset_id,
            start_date,
            end_date
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(AssetPrice::from).collect())
    }

    async fn create(&self, price: AssetPrice) -> Result<AssetPrice> {
        sqlx::query!(
            r#"
            INSERT INTO asset_prices (id, asset_id, price, date) 
            VALUES (?, ?, ?, ?)
            "#,
            price.id,
            price.asset_id,
            price.price,
            price.date
        )
        .execute(&self.db)
        .await?;

        Ok(price)
    }
}
