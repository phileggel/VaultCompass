use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

use crate::context::account::domain::{FeeFrequency, FeeSchedule, FeeScheduleRepository};

#[derive(sqlx::FromRow)]
struct FeeScheduleRow {
    id: String,
    account_id: String,
    asset_id: String,
    annual_rate_micros: i64,
    frequency: String,
    start_date: String,
    end_date: Option<String>,
    active: i64,
    last_applied_period: Option<String>,
}

impl TryFrom<FeeScheduleRow> for FeeSchedule {
    type Error = anyhow::Error;

    fn try_from(row: FeeScheduleRow) -> Result<Self> {
        let frequency = FeeFrequency::from_str(&row.frequency)
            .map_err(|_| anyhow::anyhow!("unknown fee frequency in DB: '{}'", row.frequency))?;
        Ok(FeeSchedule::restore(
            row.id,
            row.account_id,
            row.asset_id,
            row.annual_rate_micros,
            frequency,
            row.start_date,
            row.end_date,
            row.active != 0,
            row.last_applied_period,
        ))
    }
}

/// SQLite-backed implementation of `FeeScheduleRepository`.
pub struct SqliteFeeScheduleRepository {
    pool: Pool<Sqlite>,
}

impl SqliteFeeScheduleRepository {
    /// Creates a new repository backed by the given connection pool.
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FeeScheduleRepository for SqliteFeeScheduleRepository {
    async fn get_by_account_asset(
        &self,
        account_id: &str,
        asset_id: &str,
    ) -> Result<Option<FeeSchedule>> {
        let row = sqlx::query_as!(
            FeeScheduleRow,
            r#"SELECT id, account_id, asset_id, annual_rate_micros, frequency, start_date, end_date, active, last_applied_period
               FROM fee_schedules WHERE account_id = ? AND asset_id = ?"#,
            account_id,
            asset_id
        )
        .fetch_optional(&self.pool)
        .await
        .context("get_by_account_asset fee schedule")?;
        row.map(FeeSchedule::try_from).transpose()
    }

    async fn get_all_active(&self) -> Result<Vec<FeeSchedule>> {
        let rows = sqlx::query_as!(
            FeeScheduleRow,
            r#"SELECT id, account_id, asset_id, annual_rate_micros, frequency, start_date, end_date, active, last_applied_period
               FROM fee_schedules WHERE active = 1"#
        )
        .fetch_all(&self.pool)
        .await
        .context("get_all_active fee schedules")?;
        rows.into_iter().map(FeeSchedule::try_from).collect()
    }

    async fn get_active_by_account(&self, account_id: &str) -> Result<Vec<FeeSchedule>> {
        let rows = sqlx::query_as!(
            FeeScheduleRow,
            r#"SELECT id, account_id, asset_id, annual_rate_micros, frequency, start_date, end_date, active, last_applied_period
               FROM fee_schedules WHERE active = 1 AND account_id = ?"#,
            account_id
        )
        .fetch_all(&self.pool)
        .await
        .context("get_active_by_account fee schedules")?;
        rows.into_iter().map(FeeSchedule::try_from).collect()
    }

    async fn insert(&self, schedule: &FeeSchedule) -> Result<()> {
        let frequency = schedule.frequency.to_string();
        let active = schedule.active as i64;
        sqlx::query!(
            r#"INSERT INTO fee_schedules (id, account_id, asset_id, annual_rate_micros, frequency, start_date, end_date, active, last_applied_period)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            schedule.id,
            schedule.account_id,
            schedule.asset_id,
            schedule.annual_rate_percent_micros,
            frequency,
            schedule.start_date,
            schedule.end_date,
            active,
            schedule.last_applied_period
        )
        .execute(&self.pool)
        .await
        .context("insert fee schedule")?;
        Ok(())
    }

    async fn update(&self, schedule: &FeeSchedule) -> Result<()> {
        let frequency = schedule.frequency.to_string();
        let active = schedule.active as i64;
        sqlx::query!(
            r#"UPDATE fee_schedules
               SET annual_rate_micros = ?, frequency = ?, start_date = ?, end_date = ?, active = ?, last_applied_period = ?
               WHERE id = ?"#,
            schedule.annual_rate_percent_micros,
            frequency,
            schedule.start_date,
            schedule.end_date,
            active,
            schedule.last_applied_period,
            schedule.id
        )
        .execute(&self.pool)
        .await
        .context("update fee schedule")?;
        Ok(())
    }

    async fn delete_by_account_asset(&self, account_id: &str, asset_id: &str) -> Result<()> {
        sqlx::query!(
            r#"DELETE FROM fee_schedules WHERE account_id = ? AND asset_id = ?"#,
            account_id,
            asset_id
        )
        .execute(&self.pool)
        .await
        .context("delete fee schedule")?;
        Ok(())
    }
}
