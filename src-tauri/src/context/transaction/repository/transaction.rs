use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

use crate::context::transaction::domain::{Transaction, TransactionRepository, TransactionType};
use crate::core::logger::BACKEND;

#[derive(sqlx::FromRow)]
struct TransactionRow {
    id: String,
    account_id: String,
    asset_id: String,
    transaction_type: String,
    date: String,
    quantity: i64,
    unit_price: i64,
    exchange_rate: i64,
    fees: i64,
    total_amount: i64,
    note: Option<String>,
    realized_pnl: Option<i64>,
    created_at: String,
}

impl From<TransactionRow> for Transaction {
    fn from(row: TransactionRow) -> Self {
        let transaction_type =
            TransactionType::from_str(&row.transaction_type).unwrap_or_else(|_| {
                tracing::warn!(
                    target: BACKEND,
                    value = %row.transaction_type,
                    "unknown transaction_type, falling back to Purchase"
                );
                TransactionType::Purchase
            });
        Transaction::restore(
            row.id,
            row.account_id,
            row.asset_id,
            transaction_type,
            row.date,
            row.quantity,
            row.unit_price,
            row.exchange_rate,
            row.fees,
            row.total_amount,
            row.note,
            row.realized_pnl,
            row.created_at,
        )
    }
}

/// SQLite implementation of the TransactionRepository.
#[derive(Clone)]
pub struct SqliteTransactionRepository {
    pool: Pool<Sqlite>,
}

impl SqliteTransactionRepository {
    /// Creates a new SqliteTransactionRepository.
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TransactionRepository for SqliteTransactionRepository {
    async fn get_by_id(&self, id: &str) -> Result<Option<Transaction>> {
        let row = sqlx::query_as!(
            TransactionRow,
            r#"SELECT id, account_id, asset_id, transaction_type, date, quantity, unit_price, exchange_rate, fees, total_amount, note, realized_pnl, created_at FROM transactions WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch transaction {}", id))?;

        Ok(row.map(Transaction::from))
    }

    async fn get_by_account_asset(
        &self,
        account_id: &str,
        asset_id: &str,
    ) -> Result<Vec<Transaction>> {
        let rows = sqlx::query_as!(
            TransactionRow,
            r#"SELECT id, account_id, asset_id, transaction_type, date, quantity, unit_price, exchange_rate, fees, total_amount, note, realized_pnl, created_at
               FROM transactions
               WHERE account_id = ? AND asset_id = ?
               ORDER BY date ASC, created_at ASC"#,
            account_id,
            asset_id
        )
        .fetch_all(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to fetch transactions for account {} asset {}",
                account_id, asset_id
            )
        })?;

        Ok(rows.into_iter().map(Transaction::from).collect())
    }

    async fn create(&self, tx: Transaction) -> Result<Transaction> {
        let transaction_type = tx.transaction_type.to_string();
        sqlx::query!(
            r#"INSERT INTO transactions (id, account_id, asset_id, transaction_type, date, quantity, unit_price, exchange_rate, fees, total_amount, note, realized_pnl, created_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            tx.id,
            tx.account_id,
            tx.asset_id,
            transaction_type,
            tx.date,
            tx.quantity,
            tx.unit_price,
            tx.exchange_rate,
            tx.fees,
            tx.total_amount,
            tx.note,
            tx.realized_pnl,
            tx.created_at
        )
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to create transaction {}", tx.id))?;

        Ok(tx)
    }

    async fn update(&self, tx: Transaction) -> Result<Transaction> {
        let transaction_type = tx.transaction_type.to_string();
        sqlx::query!(
            r#"UPDATE transactions SET account_id = ?, asset_id = ?, transaction_type = ?, date = ?, quantity = ?, unit_price = ?, exchange_rate = ?, fees = ?, total_amount = ?, note = ?, realized_pnl = ? WHERE id = ?"#,
            tx.account_id,
            tx.asset_id,
            transaction_type,
            tx.date,
            tx.quantity,
            tx.unit_price,
            tx.exchange_rate,
            tx.fees,
            tx.total_amount,
            tx.note,
            tx.realized_pnl,
            tx.id
        )
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to update transaction {}", tx.id))?;

        Ok(tx)
    }

    async fn get_realized_pnl_by_account(&self, account_id: &str) -> Result<Vec<(String, i64)>> {
        #[derive(sqlx::FromRow)]
        struct PnlRow {
            asset_id: String,
            total_pnl: Option<i64>,
        }
        let rows = sqlx::query_as!(
            PnlRow,
            r#"SELECT asset_id, SUM(realized_pnl) as "total_pnl: i64"
               FROM transactions
               WHERE account_id = ? AND transaction_type = 'Sell'
               GROUP BY asset_id"#,
            account_id
        )
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch realized P&L for account {}", account_id))?;

        Ok(rows
            .into_iter()
            .map(|r| (r.asset_id, r.total_pnl.unwrap_or(0)))
            .collect())
    }

    async fn get_asset_ids_for_account(&self, account_id: &str) -> Result<Vec<String>> {
        let rows: Vec<String> = sqlx::query_scalar!(
            r#"SELECT DISTINCT asset_id as "asset_id: String" FROM transactions WHERE account_id = ? ORDER BY asset_id"#,
            account_id
        )
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch asset IDs for account {}", account_id))?;

        Ok(rows)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query!(r#"DELETE FROM transactions WHERE id = ?"#, id)
            .execute(&self.pool)
            .await
            .with_context(|| format!("Failed to delete transaction {}", id))?;

        Ok(())
    }
}
