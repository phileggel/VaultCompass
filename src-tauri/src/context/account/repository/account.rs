use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

use crate::context::account::domain::{Account, AccountRepository, UpdateFrequency};

#[derive(sqlx::FromRow)]
struct AccountRow {
    id: String,
    name: String,
    update_frequency: String,
}

impl From<AccountRow> for Account {
    fn from(row: AccountRow) -> Self {
        Account::from_storage(
            row.id,
            row.name,
            UpdateFrequency::from_str(&row.update_frequency).unwrap_or_default(),
        )
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
            r#"SELECT id, name, update_frequency FROM accounts WHERE is_deleted = 0"#
        )
        .fetch_all(&self.pool)
        .await
        .with_context(|| "Failed to fetch accounts")?;

        Ok(rows.into_iter().map(Account::from).collect())
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<Account>> {
        let row = sqlx::query_as!(
            AccountRow,
            r#"SELECT id, name, update_frequency FROM accounts WHERE id = ? AND is_deleted = 0"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch account {}", id))?;

        Ok(row.map(Account::from))
    }

    async fn create(&self, account: Account) -> Result<Account> {
        let update_freq_str = account.update_frequency.to_string();
        sqlx::query!(
            r#"INSERT INTO accounts (id, name, update_frequency, is_deleted) VALUES (?, ?, ?, 0)"#,
            account.id,
            account.name,
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
            r#"UPDATE accounts SET name = ?, update_frequency = ? WHERE id = ?"#,
            account.name,
            update_freq_str,
            account.id
        )
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to update account {}", account.id))?;

        Ok(account)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query!(r#"UPDATE accounts SET is_deleted = 1 WHERE id = ?"#, id)
            .execute(&self.pool)
            .await
            .with_context(|| format!("Failed to delete account {}", id))?;

        Ok(())
    }
}
