use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

use crate::context::account::domain::{HoldingNote, HoldingNoteRepository, ThresholdDirection};

#[derive(sqlx::FromRow)]
struct HoldingNoteRow {
    account_id: String,
    asset_id: String,
    text: String,
    threshold_price: Option<i64>,
    threshold_direction: Option<String>,
}

impl TryFrom<HoldingNoteRow> for HoldingNote {
    type Error = anyhow::Error;

    fn try_from(row: HoldingNoteRow) -> Result<Self> {
        let threshold_direction = row
            .threshold_direction
            .map(|direction| {
                ThresholdDirection::from_str(&direction).map_err(|_| {
                    anyhow::anyhow!("unknown threshold direction in DB: '{direction}'")
                })
            })
            .transpose()?;
        Ok(HoldingNote::from_storage(
            row.account_id,
            row.asset_id,
            row.text,
            row.threshold_price,
            threshold_direction,
        ))
    }
}

/// SQLite-backed implementation of `HoldingNoteRepository`.
pub struct SqliteHoldingNoteRepository {
    pool: Pool<Sqlite>,
}

impl SqliteHoldingNoteRepository {
    /// Creates a new repository backed by the given connection pool.
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HoldingNoteRepository for SqliteHoldingNoteRepository {
    async fn upsert(&self, note: &HoldingNote) -> Result<()> {
        let threshold_direction = note.threshold_direction.map(|d| d.to_string());
        sqlx::query!(
            r#"INSERT INTO holding_notes (account_id, asset_id, text, threshold_price, threshold_direction)
               VALUES (?, ?, ?, ?, ?)
               ON CONFLICT(account_id, asset_id) DO UPDATE SET
                   text = excluded.text,
                   threshold_price = excluded.threshold_price,
                   threshold_direction = excluded.threshold_direction"#,
            note.account_id,
            note.asset_id,
            note.text,
            note.threshold_price,
            threshold_direction
        )
        .execute(&self.pool)
        .await
        .context("upsert holding note")?;
        Ok(())
    }

    async fn delete(&self, account_id: &str, asset_id: &str) -> Result<()> {
        sqlx::query!(
            r#"DELETE FROM holding_notes WHERE account_id = ? AND asset_id = ?"#,
            account_id,
            asset_id
        )
        .execute(&self.pool)
        .await
        .context("delete holding note")?;
        Ok(())
    }

    async fn get_for_account(&self, account_id: &str) -> Result<Vec<HoldingNote>> {
        let rows = sqlx::query_as!(
            HoldingNoteRow,
            r#"SELECT account_id, asset_id, text, threshold_price, threshold_direction
               FROM holding_notes WHERE account_id = ?"#,
            account_id
        )
        .fetch_all(&self.pool)
        .await
        .context("get_for_account holding notes")?;
        rows.into_iter().map(HoldingNote::try_from).collect()
    }
}
