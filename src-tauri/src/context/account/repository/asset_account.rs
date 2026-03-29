use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::context::account::domain::{AssetAccount, AssetAccountRepository};

#[derive(sqlx::FromRow)]
struct AssetAccountRow {
    account_id: String,
    asset_id: String,
    average_price: f64,
    quantity: f64,
}

impl From<AssetAccountRow> for AssetAccount {
    fn from(row: AssetAccountRow) -> Self {
        AssetAccount {
            account_id: row.account_id,
            asset_id: row.asset_id,
            average_price: row.average_price,
            quantity: row.quantity,
        }
    }
}

/// SQLite implementation of the AssetAccountRepository.
#[derive(Clone)]
pub struct SqliteAssetAccountRepository {
    pool: Pool<Sqlite>,
}

impl SqliteAssetAccountRepository {
    /// Creates a new SqliteAssetAccountRepository.
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl AssetAccountRepository for SqliteAssetAccountRepository {
    async fn get_by_account(&self, account_id: &str) -> Result<Vec<AssetAccount>> {
        let rows = sqlx::query_as!(
            AssetAccountRow,
            r#"SELECT account_id, asset_id, average_price, quantity FROM asset_accounts WHERE account_id = ?"#,
            account_id
        )
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch assets for account {}", account_id))?;

        Ok(rows.into_iter().map(AssetAccount::from).collect())
    }

    async fn upsert(&self, aa: AssetAccount) -> Result<AssetAccount> {
        sqlx::query!(
            r#"
            INSERT INTO asset_accounts (account_id, asset_id, average_price, quantity)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(account_id, asset_id) DO UPDATE SET
                average_price = excluded.average_price,
                quantity = excluded.quantity
            "#,
            aa.account_id,
            aa.asset_id,
            aa.average_price,
            aa.quantity
        )
        .execute(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to upsert asset {} in account {}",
                aa.asset_id, aa.account_id
            )
        })?;

        Ok(aa)
    }

    async fn remove(&self, account_id: &str, asset_id: &str) -> Result<()> {
        sqlx::query!(
            r#"DELETE FROM asset_accounts WHERE account_id = ? AND asset_id = ?"#,
            account_id,
            asset_id
        )
        .execute(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to remove asset {} from account {}",
                asset_id, account_id
            )
        })?;

        Ok(())
    }
}
