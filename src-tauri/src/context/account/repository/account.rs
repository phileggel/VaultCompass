use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

use crate::context::account::domain::{
    Account, AccountChange, AccountRepository, Holding, Transaction, TransactionType,
    UpdateFrequency,
};
use crate::core::logger::BACKEND;

#[derive(sqlx::FromRow)]
struct AccountRow {
    id: String,
    name: String,
    currency: String,
    update_frequency: String,
}

impl From<AccountRow> for Account {
    fn from(row: AccountRow) -> Self {
        let update_frequency = UpdateFrequency::from_str(&row.update_frequency).unwrap_or_else(|_| {
            tracing::warn!(target: BACKEND, value = %row.update_frequency, "unknown update_frequency value, falling back to default");
            UpdateFrequency::default()
        });
        Account::restore(row.id, row.name, row.currency, update_frequency)
    }
}

#[derive(sqlx::FromRow)]
struct HoldingRow {
    id: String,
    account_id: String,
    asset_id: String,
    quantity: i64,
    average_price: i64,
    total_realized_pnl: i64,
    last_sold_date: Option<String>,
}

impl From<HoldingRow> for Holding {
    fn from(row: HoldingRow) -> Self {
        Holding::restore(
            row.id,
            row.account_id,
            row.asset_id,
            row.quantity,
            row.average_price,
            row.total_realized_pnl,
            row.last_sold_date,
        )
    }
}

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

impl TryFrom<TransactionRow> for Transaction {
    type Error = anyhow::Error;

    fn try_from(row: TransactionRow) -> Result<Self> {
        let transaction_type = TransactionType::from_str(&row.transaction_type).map_err(|_| {
            anyhow::anyhow!("unknown transaction_type in DB: '{}'", row.transaction_type)
        })?;
        Ok(Transaction::restore(
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
        ))
    }
}

/// SQLite implementation of the AccountRepository.
#[derive(Clone)]
pub struct SqliteAccountRepository {
    pool: Pool<Sqlite>,
}

impl SqliteAccountRepository {
    /// Creates a new SqliteAccountRepository.
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl AccountRepository for SqliteAccountRepository {
    async fn get_all(&self) -> Result<Vec<Account>> {
        let rows = sqlx::query_as!(
            AccountRow,
            r#"SELECT id, name, currency, update_frequency FROM accounts"#
        )
        .fetch_all(&self.pool)
        .await
        .with_context(|| "Failed to fetch accounts")?;

        Ok(rows.into_iter().map(Account::from).collect())
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<Account>> {
        let row = sqlx::query_as!(
            AccountRow,
            r#"SELECT id, name, currency, update_frequency FROM accounts WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch account {}", id))?;

        Ok(row.map(Account::from))
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Account>> {
        let row = sqlx::query_as!(
            AccountRow,
            r#"SELECT id, name, currency, update_frequency FROM accounts WHERE LOWER(name) = LOWER(?)"#,
            name
        )
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to find account by name {}", name))?;

        Ok(row.map(Account::from))
    }

    async fn create(&self, account: Account) -> Result<Account> {
        let update_freq_str = account.update_frequency.to_string();
        sqlx::query!(
            r#"INSERT INTO accounts (id, name, currency, update_frequency) VALUES (?, ?, ?, ?)"#,
            account.id,
            account.name,
            account.currency,
            update_freq_str
        )
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to create account {}", account.name))?;

        Ok(account)
    }

    async fn update(&self, account: Account) -> Result<Account> {
        let update_freq_str = account.update_frequency.to_string();
        sqlx::query!(
            r#"UPDATE accounts SET name = ?, currency = ?, update_frequency = ? WHERE id = ?"#,
            account.name,
            account.currency,
            update_freq_str,
            account.id
        )
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to update account {}", account.id))?;

        Ok(account)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query!(r#"DELETE FROM accounts WHERE id = ?"#, id)
            .execute(&self.pool)
            .await
            .with_context(|| format!("Failed to delete account {}", id))?;

        Ok(())
    }

    async fn get_with_holdings_and_transactions(&self, id: &str) -> Result<Option<Account>> {
        let account_row = sqlx::query_as!(
            AccountRow,
            r#"SELECT id, name, currency, update_frequency FROM accounts WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch account {}", id))?;

        let account_row = match account_row {
            Some(r) => r,
            None => return Ok(None),
        };

        let base = Account::from(account_row);

        let holding_rows = sqlx::query_as!(
            HoldingRow,
            r#"SELECT id, account_id, asset_id, quantity, average_price, total_realized_pnl, last_sold_date
               FROM holdings WHERE account_id = ?"#,
            id
        )
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch holdings for account {}", id))?;

        let tx_rows = sqlx::query_as!(
            TransactionRow,
            r#"SELECT id, account_id, asset_id, transaction_type, date, quantity, unit_price,
                      exchange_rate, fees, total_amount, note, realized_pnl, created_at
               FROM transactions WHERE account_id = ?
               ORDER BY date ASC, created_at ASC"#,
            id
        )
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch transactions for account {}", id))?;

        let holdings = holding_rows.into_iter().map(Holding::from).collect();
        let transactions = tx_rows
            .into_iter()
            .map(Transaction::try_from)
            .collect::<Result<Vec<_>>>()?;

        Ok(Some(Account::restore_with_positions(
            base.id,
            base.name,
            base.currency,
            base.update_frequency,
            holdings,
            transactions,
        )))
    }

    async fn save(&self, account: &mut Account) -> Result<()> {
        let mut db_tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin DB transaction for account save")?;

        for change in account.pending_changes() {
            match change {
                AccountChange::TransactionInserted(tx) => {
                    let tx_type = tx.transaction_type.to_string();
                    sqlx::query!(
                        r#"INSERT INTO transactions (id, account_id, asset_id, transaction_type, date, quantity,
                               unit_price, exchange_rate, fees, total_amount, note, realized_pnl, created_at)
                           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                        tx.id, tx.account_id, tx.asset_id, tx_type, tx.date,
                        tx.quantity, tx.unit_price, tx.exchange_rate, tx.fees,
                        tx.total_amount, tx.note, tx.realized_pnl, tx.created_at
                    )
                    .execute(&mut *db_tx)
                    .await
                    .with_context(|| format!("Failed to insert transaction {}", tx.id))?;
                }
                AccountChange::TransactionUpdated(tx) => {
                    let tx_type = tx.transaction_type.to_string();
                    // created_at is immutable after creation (SEL-024)
                    sqlx::query!(
                        r#"UPDATE transactions SET account_id = ?, asset_id = ?, transaction_type = ?,
                               date = ?, quantity = ?, unit_price = ?, exchange_rate = ?, fees = ?,
                               total_amount = ?, note = ?, realized_pnl = ?
                           WHERE id = ?"#,
                        tx.account_id, tx.asset_id, tx_type, tx.date, tx.quantity,
                        tx.unit_price, tx.exchange_rate, tx.fees, tx.total_amount,
                        tx.note, tx.realized_pnl, tx.id
                    )
                    .execute(&mut *db_tx)
                    .await
                    .with_context(|| format!("Failed to update transaction {}", tx.id))?;
                }
                AccountChange::TransactionDeleted(id) => {
                    sqlx::query!(r#"DELETE FROM transactions WHERE id = ?"#, id)
                        .execute(&mut *db_tx)
                        .await
                        .with_context(|| format!("Failed to delete transaction {}", id))?;
                }
                AccountChange::HoldingUpserted(h) => {
                    sqlx::query!(
                        r#"INSERT INTO holdings (id, account_id, asset_id, quantity, average_price,
                               total_realized_pnl, last_sold_date)
                           VALUES (?, ?, ?, ?, ?, ?, ?)
                           ON CONFLICT(account_id, asset_id) DO UPDATE SET
                               quantity = excluded.quantity,
                               average_price = excluded.average_price,
                               total_realized_pnl = excluded.total_realized_pnl,
                               last_sold_date = excluded.last_sold_date"#,
                        h.id,
                        h.account_id,
                        h.asset_id,
                        h.quantity,
                        h.average_price,
                        h.total_realized_pnl,
                        h.last_sold_date
                    )
                    .execute(&mut *db_tx)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to upsert holding for account {} asset {}",
                            h.account_id, h.asset_id
                        )
                    })?;
                }
                AccountChange::HoldingDeleted {
                    account_id,
                    asset_id,
                } => {
                    sqlx::query!(
                        r#"DELETE FROM holdings WHERE account_id = ? AND asset_id = ?"#,
                        account_id,
                        asset_id
                    )
                    .execute(&mut *db_tx)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to delete holding for account {} asset {}",
                            account_id, asset_id
                        )
                    })?;
                }
            }
        }

        db_tx
            .commit()
            .await
            .context("Failed to commit account save")?;

        account.pending_changes.clear();
        Ok(())
    }
}
