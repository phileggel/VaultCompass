use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, SqliteConnection};
use std::str::FromStr;
use std::sync::Arc;

use crate::context::account::domain::{
    Account, AccountChange, AccountRepository, Holding, Transaction, TransactionType,
    UpdateFrequency,
};
use crate::core::logger::BACKEND;
use crate::shared::domain::{ChangeDraft, Operation, Origin, Rank, RecordIdentity, RecordKind};
use crate::shared::infrastructure::change_recorder::{
    ChangeRecorder, NoopChangeRecorder, RankColumns,
};

#[derive(sqlx::FromRow)]
struct AccountRow {
    id: String,
    name: String,
    bank_name: String,
    currency: String,
    update_frequency: String,
    management_fees_enabled: i64,
}

impl From<AccountRow> for Account {
    fn from(row: AccountRow) -> Self {
        let update_frequency = UpdateFrequency::from_str(&row.update_frequency).unwrap_or_else(|_| {
            tracing::warn!(target: BACKEND, value = %row.update_frequency, "unknown update_frequency value, falling back to default");
            UpdateFrequency::default()
        });
        Account::restore(
            row.id,
            row.name,
            row.bank_name,
            row.currency,
            update_frequency,
            row.management_fees_enabled != 0,
        )
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
    change_recorder: Arc<dyn ChangeRecorder>,
}

impl SqliteAccountRepository {
    /// Creates a new SqliteAccountRepository.
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self {
            pool,
            change_recorder: Arc::new(NoopChangeRecorder),
        }
    }

    /// Attaches the change recorder every write appends through (SYN-020).
    pub fn with_change_recorder(mut self, change_recorder: Arc<dyn ChangeRecorder>) -> Self {
        self.change_recorder = change_recorder;
        self
    }

    async fn record_account(
        &self,
        conn: &mut SqliteConnection,
        account_id: &str,
        draft: ChangeDraft,
    ) -> Result<()> {
        let rank = self.change_recorder.record(conn, draft).await?;
        if let Some(rank) = rank {
            let columns = RankColumns::from(rank);
            sqlx::query!(
                r#"UPDATE accounts SET sync_logical_timestamp = ?, sync_origin = ?, sync_device_id = ?
                   WHERE id = ?"#,
                columns.logical_timestamp,
                columns.origin,
                columns.device_id,
                account_id
            )
            .execute(conn)
            .await
            .with_context(|| format!("Failed to stamp rank on account {}", account_id))?;
        }
        Ok(())
    }

    async fn record_transaction(
        &self,
        conn: &mut SqliteConnection,
        transaction_id: &str,
        draft: ChangeDraft,
    ) -> Result<()> {
        let rank = self.change_recorder.record(conn, draft).await?;
        if let Some(rank) = rank {
            let columns = RankColumns::from(rank);
            sqlx::query!(
                r#"UPDATE transactions SET sync_logical_timestamp = ?, sync_origin = ?, sync_device_id = ?
                   WHERE id = ?"#,
                columns.logical_timestamp,
                columns.origin,
                columns.device_id,
                transaction_id
            )
            .execute(conn)
            .await
            .with_context(|| format!("Failed to stamp rank on transaction {}", transaction_id))?;
        }
        Ok(())
    }
}

fn account_identity(account_id: &str) -> RecordIdentity {
    RecordIdentity::canonical(RecordKind::Account, &[account_id])
}

fn transaction_identity(transaction_id: &str) -> RecordIdentity {
    RecordIdentity::canonical(RecordKind::Transaction, &[transaction_id])
}

#[async_trait::async_trait]
impl AccountRepository for SqliteAccountRepository {
    async fn stamp_sync_rank(&self, conn: &mut SqliteConnection, rank: &Rank) -> Result<u64> {
        let columns = RankColumns::from(rank.clone());
        let (timestamp, origin, device_id) = (
            &columns.logical_timestamp,
            &columns.origin,
            &columns.device_id,
        );
        let mut stamped = 0;
        stamped += sqlx::query!(
            "UPDATE accounts SET sync_logical_timestamp = ?, sync_origin = ?, sync_device_id = ?
             WHERE sync_logical_timestamp IS NULL",
            timestamp,
            origin,
            device_id
        )
        .execute(&mut *conn)
        .await
        .context("Failed to stamp unranked accounts")?
        .rows_affected();
        stamped += sqlx::query!(
            "UPDATE transactions SET sync_logical_timestamp = ?, sync_origin = ?, sync_device_id = ?
             WHERE sync_logical_timestamp IS NULL",
            timestamp,
            origin,
            device_id
        )
        .execute(&mut *conn)
        .await
        .context("Failed to stamp unranked transactions")?
        .rows_affected();
        stamped += sqlx::query!(
            "UPDATE fee_schedules SET sync_logical_timestamp = ?, sync_origin = ?, sync_device_id = ?
             WHERE sync_logical_timestamp IS NULL",
            timestamp,
            origin,
            device_id
        )
        .execute(&mut *conn)
        .await
        .context("Failed to stamp unranked fee schedules")?
        .rows_affected();
        stamped += sqlx::query!(
            "UPDATE fee_catch_up_positions SET sync_logical_timestamp = ?, sync_origin = ?, sync_device_id = ?
             WHERE sync_logical_timestamp IS NULL",
            timestamp,
            origin,
            device_id
        )
        .execute(&mut *conn)
        .await
        .context("Failed to stamp unranked fee catch-up positions")?
        .rows_affected();
        stamped += sqlx::query!(
            "UPDATE holding_notes SET sync_logical_timestamp = ?, sync_origin = ?, sync_device_id = ?
             WHERE sync_logical_timestamp IS NULL",
            timestamp,
            origin,
            device_id
        )
        .execute(&mut *conn)
        .await
        .context("Failed to stamp unranked holding notes")?
        .rows_affected();
        Ok(stamped)
    }

    async fn get_all(&self) -> Result<Vec<Account>> {
        let rows = sqlx::query_as!(
            AccountRow,
            r#"SELECT id, name, bank_name, currency, update_frequency, management_fees_enabled FROM accounts"#
        )
        .fetch_all(&self.pool)
        .await
        .with_context(|| "Failed to fetch accounts")?;

        Ok(rows.into_iter().map(Account::from).collect())
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<Account>> {
        let row = sqlx::query_as!(
            AccountRow,
            r#"SELECT id, name, bank_name, currency, update_frequency, management_fees_enabled FROM accounts WHERE id = ?"#,
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
            r#"SELECT id, name, bank_name, currency, update_frequency, management_fees_enabled FROM accounts WHERE LOWER(name) = LOWER(?)"#,
            name
        )
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to find account by name {}", name))?;

        Ok(row.map(Account::from))
    }

    async fn create(&self, account: Account) -> Result<Account> {
        let update_freq_str = account.update_frequency.to_string();
        let management_fees_enabled = account.management_fees_enabled as i64;
        let mut db_tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin DB transaction for account create")?;
        sqlx::query!(
            r#"INSERT INTO accounts (id, name, bank_name, currency, update_frequency, management_fees_enabled)
               VALUES (?, ?, ?, ?, ?, ?)"#,
            account.id,
            account.name,
            account.bank_name,
            account.currency,
            update_freq_str,
            management_fees_enabled
        )
        .execute(&mut *db_tx)
        .await
        .with_context(|| format!("Failed to create account {}", account.name))?;
        let draft = ChangeDraft::new(
            RecordKind::Account,
            account_identity(&account.id),
            Operation::Created,
            Origin::User,
            None,
            Some(serde_json::to_string(&account)?),
        );
        self.record_account(&mut db_tx, &account.id, draft).await?;
        db_tx
            .commit()
            .await
            .context("Failed to commit account create")?;

        Ok(account)
    }

    async fn update(&self, account: Account) -> Result<Account> {
        let update_freq_str = account.update_frequency.to_string();
        let management_fees_enabled = account.management_fees_enabled as i64;
        let mut db_tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin DB transaction for account update")?;
        let written = sqlx::query!(
            r#"UPDATE accounts
               SET name = ?, bank_name = ?, currency = ?, update_frequency = ?, management_fees_enabled = ?
               WHERE id = ?"#,
            account.name,
            account.bank_name,
            account.currency,
            update_freq_str,
            management_fees_enabled,
            account.id
        )
        .execute(&mut *db_tx)
        .await
        .with_context(|| format!("Failed to update account {}", account.id))?;
        if written.rows_affected() > 0 {
            let draft = ChangeDraft::new(
                RecordKind::Account,
                account_identity(&account.id),
                Operation::Updated,
                Origin::User,
                None,
                Some(serde_json::to_string(&account)?),
            );
            self.record_account(&mut db_tx, &account.id, draft).await?;
        }
        db_tx
            .commit()
            .await
            .context("Failed to commit account update")?;

        Ok(account)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut db_tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin DB transaction for account delete")?;
        let deleted = sqlx::query!(r#"DELETE FROM accounts WHERE id = ?"#, id)
            .execute(&mut *db_tx)
            .await
            .with_context(|| format!("Failed to delete account {}", id))?;
        if deleted.rows_affected() > 0 {
            let draft = ChangeDraft::new(
                RecordKind::Account,
                account_identity(id),
                Operation::Removed,
                Origin::User,
                None,
                None,
            );
            self.change_recorder.record(&mut db_tx, draft).await?;
        }
        db_tx
            .commit()
            .await
            .context("Failed to commit account delete")?;

        Ok(())
    }

    async fn get_with_holdings_and_transactions(&self, id: &str) -> Result<Option<Account>> {
        let account_row = sqlx::query_as!(
            AccountRow,
            r#"SELECT id, name, bank_name, currency, update_frequency, management_fees_enabled FROM accounts WHERE id = ?"#,
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
               ORDER BY date ASC, created_at ASC, id ASC"#,
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
            base.bank_name,
            base.currency,
            base.update_frequency,
            base.management_fees_enabled,
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
                    let draft = ChangeDraft::new(
                        RecordKind::Transaction,
                        transaction_identity(&tx.id),
                        Operation::Created,
                        Origin::User,
                        None,
                        Some(serde_json::to_string(tx)?),
                    );
                    self.record_transaction(&mut db_tx, &tx.id, draft).await?;
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
                    let draft = ChangeDraft::new(
                        RecordKind::Transaction,
                        transaction_identity(&tx.id),
                        Operation::Updated,
                        Origin::User,
                        None,
                        Some(serde_json::to_string(tx)?),
                    );
                    self.record_transaction(&mut db_tx, &tx.id, draft).await?;
                }
                AccountChange::TransactionDeleted(id) => {
                    let deleted = sqlx::query!(r#"DELETE FROM transactions WHERE id = ?"#, id)
                        .execute(&mut *db_tx)
                        .await
                        .with_context(|| format!("Failed to delete transaction {}", id))?;
                    if deleted.rows_affected() > 0 {
                        let draft = ChangeDraft::new(
                            RecordKind::Transaction,
                            transaction_identity(id),
                            Operation::Removed,
                            Origin::User,
                            None,
                            None,
                        );
                        self.change_recorder.record(&mut db_tx, draft).await?;
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::account::domain::{Account, AccountChange, UpdateFrequency};
    use crate::context::sync::SqliteChangeRecorder;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::Arc;

    async fn make_pool() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test pool");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");
        pool
    }

    async fn seed_sync_device(pool: &Pool<Sqlite>) {
        sqlx::query!(
            r#"INSERT INTO sync_device
               (id, device_id, device_name, folder, joined_at, paused, portfolio_created_at,
                logical_clock, derived_key, data_format_version)
               VALUES (1, 'desktop-device', 'Desktop', '/tmp/sync', '2026-08-22T00:00:00Z', 0,
                       '2026-08-22T00:00:00Z', 0, X'00', 1)"#
        )
        .execute(pool)
        .await
        .expect("seed sync_device");
    }

    async fn seed_asset(pool: &Pool<Sqlite>, id: &str) {
        sqlx::query!(
            "INSERT INTO assets (id, name, reference, asset_class, currency, risk_level, category_id, is_archived)
             VALUES (?, 'Test Asset', 'REF', 'Stocks', 'EUR', 3, 'default-uncategorized', 0)",
            id,
        )
        .execute(pool)
        .await
        .expect("seed asset");
    }

    async fn changes_with_operation(pool: &Pool<Sqlite>, operation: &str) -> i64 {
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM changes WHERE operation = ?",
            operation
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    fn new_account() -> Account {
        Account::new(
            "CTO".to_string(),
            "Boursorama".to_string(),
            "EUR".to_string(),
            UpdateFrequency::ManualMonth,
            false,
        )
        .unwrap()
    }

    fn recorded_tx(id: &str, account_id: &str, asset_id: &str) -> Transaction {
        Transaction::restore(
            id.to_string(),
            account_id.to_string(),
            asset_id.to_string(),
            TransactionType::Purchase,
            "2026-08-01".to_string(),
            10,
            100_000_000,
            1_000_000,
            0,
            1_000_000_000,
            None,
            None,
            "2026-08-01T00:00:00Z".to_string(),
        )
    }

    // SYN-020/021 — create records exactly one Created change, rank-stamped (CFR-014).
    #[tokio::test]
    async fn create_records_one_created_change_with_rank_stamped() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAccountRepository::new(pool.clone()).with_change_recorder(recorder);

        let account = new_account();
        repo.create(account.clone()).await.unwrap();

        assert_eq!(changes_with_operation(&pool, "Created").await, 1);
        let row = sqlx::query!(
            "SELECT sync_logical_timestamp, sync_origin FROM accounts WHERE id = ?",
            account.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            row.sync_logical_timestamp.is_some(),
            "CFR-014: the row's rank columns are stamped"
        );
        assert_eq!(row.sync_origin.as_deref(), Some("User"));
    }

    // CFR-014/D6 — stamp_sync_rank ranks only the rows that were never ranked; a row the
    // recorder already ranked keeps its own rank.
    #[tokio::test]
    async fn stamp_sync_rank_stamps_only_unranked_rows() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let unranked = new_account();
        SqliteAccountRepository::new(pool.clone())
            .create(unranked.clone())
            .await
            .unwrap();
        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAccountRepository::new(pool.clone()).with_change_recorder(recorder);
        let mut ranked = new_account();
        ranked.id = "acc-ranked".to_string();
        ranked.name = "Ranked".to_string();
        repo.create(ranked.clone()).await.unwrap();

        let rank = crate::shared::domain::Rank {
            origin: Origin::User,
            logical_timestamp: crate::shared::domain::LogicalTimestamp::new(99),
            device_id: "desktop-device".to_string(),
        };
        let mut conn = pool.acquire().await.unwrap();
        let stamped = repo.stamp_sync_rank(&mut conn, &rank).await.unwrap();
        drop(conn);
        assert_eq!(stamped, 1, "only the unranked account is stamped");

        let rows = sqlx::query!("SELECT id, sync_logical_timestamp FROM accounts ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
        let stamp_of = |id: &str| {
            rows.iter()
                .find(|row| row.id == id)
                .and_then(|row| row.sync_logical_timestamp.clone())
        };
        assert_eq!(
            stamp_of(&unranked.id).as_deref(),
            Some("00000000000000000099")
        );
        assert_ne!(
            stamp_of(&ranked.id).as_deref(),
            Some("00000000000000000099"),
            "a row the recorder ranked keeps its rank"
        );
    }

    // SYN-020 — update records exactly one Updated change.
    #[tokio::test]
    async fn update_records_one_updated_change() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let setup_repo = SqliteAccountRepository::new(pool.clone());
        let account = new_account();
        setup_repo.create(account.clone()).await.unwrap();

        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAccountRepository::new(pool.clone()).with_change_recorder(recorder);
        let mut renamed = account.clone();
        renamed.name = "CTO Fortuneo".to_string();
        repo.update(renamed).await.unwrap();

        assert_eq!(changes_with_operation(&pool, "Updated").await, 1);
    }

    // SYN-020/024 — delete records exactly one Removed change and leaves a tombstone (CFR-015).
    #[tokio::test]
    async fn delete_records_one_removed_change_and_tombstone() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let setup_repo = SqliteAccountRepository::new(pool.clone());
        let account = new_account();
        setup_repo.create(account.clone()).await.unwrap();

        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAccountRepository::new(pool.clone()).with_change_recorder(recorder);
        repo.delete(&account.id).await.unwrap();

        assert_eq!(changes_with_operation(&pool, "Removed").await, 1);
        let tombstone = sqlx::query!(
            "SELECT record_identity FROM tombstones WHERE record_kind = 'Account' AND record_identity = ?",
            account.id
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert!(tombstone.is_some(), "CFR-015: a removal leaves a tombstone");
    }

    // SYN-020 — a failed write records no change (rollback: the row and its change exist
    // together or not at all).
    #[tokio::test]
    async fn create_rolls_back_change_when_the_write_fails() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAccountRepository::new(pool.clone()).with_change_recorder(recorder);
        let account = new_account();
        repo.create(account.clone()).await.unwrap();

        // Same id — PRIMARY KEY violation, the whole write must fail atomically.
        let result = repo.create(account.clone()).await;
        assert!(result.is_err());

        assert_eq!(
            changes_with_operation(&pool, "Created").await,
            1,
            "only the first (successful) create recorded a change; the failed second create recorded none"
        );
    }

    // SYN-020 — AccountChange::TransactionInserted (via save) records one Created change.
    #[tokio::test]
    async fn save_transaction_inserted_records_one_created_change() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let setup_repo = SqliteAccountRepository::new(pool.clone());
        let account = new_account();
        setup_repo.create(account.clone()).await.unwrap();
        seed_asset(&pool, "asset-1").await;

        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAccountRepository::new(pool.clone()).with_change_recorder(recorder);
        let mut loaded = account.clone();
        loaded
            .pending_changes
            .push(AccountChange::TransactionInserted(recorded_tx(
                "tx-1",
                &account.id,
                "asset-1",
            )));
        repo.save(&mut loaded).await.unwrap();

        assert_eq!(changes_with_operation(&pool, "Created").await, 1);
    }

    // SYN-020 — AccountChange::TransactionUpdated (via save) records one Updated change.
    #[tokio::test]
    async fn save_transaction_updated_records_one_updated_change() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let setup_repo = SqliteAccountRepository::new(pool.clone());
        let account = new_account();
        setup_repo.create(account.clone()).await.unwrap();
        seed_asset(&pool, "asset-1").await;
        let mut seeded = account.clone();
        seeded
            .pending_changes
            .push(AccountChange::TransactionInserted(recorded_tx(
                "tx-1",
                &account.id,
                "asset-1",
            )));
        setup_repo.save(&mut seeded).await.unwrap();

        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAccountRepository::new(pool.clone()).with_change_recorder(recorder);
        let mut updated = account.clone();
        updated
            .pending_changes
            .push(AccountChange::TransactionUpdated(recorded_tx(
                "tx-1",
                &account.id,
                "asset-1",
            )));
        repo.save(&mut updated).await.unwrap();

        assert_eq!(changes_with_operation(&pool, "Updated").await, 1);
    }

    // SYN-020/024 — AccountChange::TransactionDeleted (via save) records one Removed change
    // and leaves a tombstone.
    #[tokio::test]
    async fn save_transaction_deleted_records_one_removed_change_and_tombstone() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let setup_repo = SqliteAccountRepository::new(pool.clone());
        let account = new_account();
        setup_repo.create(account.clone()).await.unwrap();
        seed_asset(&pool, "asset-1").await;
        let mut seeded = account.clone();
        seeded
            .pending_changes
            .push(AccountChange::TransactionInserted(recorded_tx(
                "tx-1",
                &account.id,
                "asset-1",
            )));
        setup_repo.save(&mut seeded).await.unwrap();

        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAccountRepository::new(pool.clone()).with_change_recorder(recorder);
        let mut deleted = account.clone();
        deleted
            .pending_changes
            .push(AccountChange::TransactionDeleted("tx-1".to_string()));
        repo.save(&mut deleted).await.unwrap();

        assert_eq!(changes_with_operation(&pool, "Removed").await, 1);
        let tombstone = sqlx::query!(
            "SELECT record_identity FROM tombstones WHERE record_kind = 'Transaction' AND record_identity = 'tx-1'"
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert!(tombstone.is_some());
    }

    // CFR-041 — transactions load in `date, created_at, id` order: two transactions dated
    // and created identically load in id order (the total-order fix this PR adds).
    #[tokio::test]
    async fn get_with_holdings_and_transactions_orders_by_date_created_at_id() {
        let pool = make_pool().await;
        let setup_repo = SqliteAccountRepository::new(pool.clone());
        let account = new_account();
        setup_repo.create(account.clone()).await.unwrap();
        seed_asset(&pool, "asset-1").await;

        // Same date, same created_at — only `id` decides the order (CFR-041).
        for id in ["tx-b", "tx-a"] {
            sqlx::query!(
                r#"INSERT INTO transactions (id, account_id, asset_id, transaction_type, date, quantity,
                       unit_price, exchange_rate, fees, total_amount, note, realized_pnl, created_at)
                   VALUES (?, ?, 'asset-1', 'Purchase', '2026-08-20', 10, 100000000, 1000000, 0, 1000000000, NULL, NULL, '2026-08-20T00:00:00Z')"#,
                id,
                account.id,
            )
            .execute(&pool)
            .await
            .unwrap();
        }

        let loaded = setup_repo
            .get_with_holdings_and_transactions(&account.id)
            .await
            .unwrap()
            .unwrap();
        let ids: Vec<&str> = loaded.transactions.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["tx-a", "tx-b"],
            "CFR-041: equal date and created_at fall back to transaction id order"
        );
    }
}
