use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

use crate::context::account::domain::{Account, AccountRepository, UpdateFrequency};
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
}
