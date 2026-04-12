use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Pool, Sqlite};

use crate::context::account::domain::{Holding, HoldingRepository};

#[derive(sqlx::FromRow)]
struct HoldingRow {
    id: String,
    account_id: String,
    asset_id: String,
    quantity: i64,
    average_price: i64,
}

impl From<HoldingRow> for Holding {
    fn from(row: HoldingRow) -> Self {
        Holding::restore(
            row.id,
            row.account_id,
            row.asset_id,
            row.quantity,
            row.average_price,
        )
    }
}

/// SQLite implementation of the HoldingRepository.
#[derive(Clone)]
pub struct SqliteHoldingRepository {
    pool: Pool<Sqlite>,
}

impl SqliteHoldingRepository {
    /// Creates a new SqliteHoldingRepository.
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HoldingRepository for SqliteHoldingRepository {
    async fn get_by_account(&self, account_id: &str) -> Result<Vec<Holding>> {
        let rows = sqlx::query_as!(
            HoldingRow,
            r#"SELECT id, account_id, asset_id, quantity, average_price FROM holdings WHERE account_id = ?"#,
            account_id
        )
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch holdings for account {}", account_id))?;

        Ok(rows.into_iter().map(Holding::from).collect())
    }

    async fn get_by_account_asset(
        &self,
        account_id: &str,
        asset_id: &str,
    ) -> Result<Option<Holding>> {
        let row = sqlx::query_as!(
            HoldingRow,
            r#"SELECT id, account_id, asset_id, quantity, average_price FROM holdings WHERE account_id = ? AND asset_id = ?"#,
            account_id,
            asset_id
        )
        .fetch_optional(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to fetch holding for account {} asset {}",
                account_id, asset_id
            )
        })?;

        Ok(row.map(Holding::from))
    }

    async fn upsert(&self, holding: Holding) -> Result<Holding> {
        sqlx::query!(
            r#"INSERT INTO holdings (id, account_id, asset_id, quantity, average_price)
               VALUES (?, ?, ?, ?, ?)
               ON CONFLICT(account_id, asset_id) DO UPDATE SET
                   quantity = excluded.quantity,
                   average_price = excluded.average_price"#,
            holding.id,
            holding.account_id,
            holding.asset_id,
            holding.quantity,
            holding.average_price
        )
        .execute(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to upsert holding for account {} asset {}",
                holding.account_id, holding.asset_id
            )
        })?;

        Ok(holding)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query!(r#"DELETE FROM holdings WHERE id = ?"#, id)
            .execute(&self.pool)
            .await
            .with_context(|| format!("Failed to delete holding {}", id))?;

        Ok(())
    }

    async fn delete_by_account_asset(&self, account_id: &str, asset_id: &str) -> Result<()> {
        sqlx::query!(
            r#"DELETE FROM holdings WHERE account_id = ? AND asset_id = ?"#,
            account_id,
            asset_id
        )
        .execute(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to delete holding for account {} asset {}",
                account_id, asset_id
            )
        })?;

        Ok(())
    }
}
