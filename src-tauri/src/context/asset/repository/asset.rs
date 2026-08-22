use std::str::FromStr;
use std::sync::Arc;

use super::super::domain::{exchange, Asset, AssetCategory, AssetClass, AssetRepository};
use crate::shared::domain::{ChangeDraft, Operation, Origin, Rank, RecordIdentity, RecordKind};
use crate::shared::infrastructure::change_recorder::{
    ChangeRecorder, NoopChangeRecorder, RankColumns,
};
use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, SqliteConnection};

#[derive(sqlx::FromRow)]
struct AssetRow {
    id: String,
    name: String,
    reference: String,
    isin: Option<String>,
    asset_class: String,
    currency: String,
    risk_level: i64,
    category_id: String,
    category_name: String,
    is_archived: bool,
    exchange_code: Option<String>,
    price_refresh_blocked: bool,
    interest_bearing: bool,
}

impl From<AssetRow> for Asset {
    fn from(row: AssetRow) -> Self {
        let asset_class = AssetClass::from_str(&row.asset_class).unwrap_or_default();
        let exchange = row.exchange_code.as_deref().and_then(exchange::lookup);
        Asset::restore(
            row.id,
            row.name,
            asset_class,
            AssetCategory::from_storage(row.category_id, row.category_name),
            row.currency,
            row.risk_level.try_into().unwrap_or(0),
            row.reference,
            row.isin,
            row.is_archived,
            exchange,
            row.price_refresh_blocked,
            row.interest_bearing,
        )
    }
}

/// Loads one asset by id on the given connection, whatever its deleted or archived state —
/// the full record state a change's content carries (SYN-020).
pub(super) async fn fetch_asset(conn: &mut SqliteConnection, id: &str) -> Result<Option<Asset>> {
    let row = sqlx::query_as!(
        AssetRow,
        r#"
        SELECT
            a.id, a.name, a.reference, a.isin, a.asset_class, a.currency, a.risk_level,
            c.id as category_id,
            c.name as category_name,
            a.is_archived as "is_archived: bool",
            a.exchange_code,
            a.price_refresh_blocked as "price_refresh_blocked: bool",
            a.interest_bearing as "interest_bearing: bool"
        FROM assets a
        JOIN categories c ON a.category_id = c.id
        WHERE a.id = ?
        "#,
        id
    )
    .fetch_optional(conn)
    .await
    .with_context(|| format!("Failed to fetch asset with id: {}", id))?;
    Ok(row.map(Asset::from))
}

/// Stamps the CFR-014 rank columns on one asset row.
pub(super) async fn stamp_asset_rank(
    conn: &mut SqliteConnection,
    id: &str,
    rank: Rank,
) -> Result<()> {
    let columns = RankColumns::from(rank);
    sqlx::query!(
        r#"UPDATE assets SET sync_logical_timestamp = ?, sync_origin = ?, sync_device_id = ? WHERE id = ?"#,
        columns.logical_timestamp,
        columns.origin,
        columns.device_id,
        id
    )
    .execute(conn)
    .await
    .with_context(|| format!("Failed to stamp rank on asset with id: {}", id))?;
    Ok(())
}

pub(super) fn asset_identity(id: &str) -> RecordIdentity {
    RecordIdentity::canonical(RecordKind::Asset, &[id])
}

/// SQLite implementation of the AssetRepository.
#[derive(Clone)]
pub struct SqliteAssetRepository {
    pool: Pool<Sqlite>,
    change_recorder: Arc<dyn ChangeRecorder>,
}

impl SqliteAssetRepository {
    /// Creates a new SqliteAssetRepository.
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

    async fn record_asset_state(
        &self,
        conn: &mut SqliteConnection,
        asset: &Asset,
        operation: Operation,
    ) -> Result<()> {
        let draft = ChangeDraft::new(
            RecordKind::Asset,
            asset_identity(&asset.id),
            operation,
            Origin::User,
            None,
            Some(serde_json::to_string(asset)?),
        );
        let rank = self.change_recorder.record(conn, draft).await?;
        if let Some(rank) = rank {
            stamp_asset_rank(conn, &asset.id, rank).await?;
        }
        Ok(())
    }

    /// Re-reads the asset an id-only update touched and records it as Updated; a
    /// statement that matched no row records nothing (SYN-020).
    async fn record_updated_by_id(
        &self,
        conn: &mut SqliteConnection,
        id: &str,
        rows_affected: u64,
    ) -> Result<()> {
        if rows_affected == 0 {
            return Ok(());
        }
        if let Some(asset) = fetch_asset(conn, id).await? {
            self.record_asset_state(conn, &asset, Operation::Updated)
                .await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl AssetRepository for SqliteAssetRepository {
    async fn stamp_sync_rank(&self, conn: &mut SqliteConnection, rank: &Rank) -> Result<u64> {
        let columns = RankColumns::from(rank.clone());
        let (timestamp, origin, device_id) = (
            &columns.logical_timestamp,
            &columns.origin,
            &columns.device_id,
        );
        let mut stamped = 0;
        stamped += sqlx::query!(
            "UPDATE assets SET sync_logical_timestamp = ?, sync_origin = ?, sync_device_id = ?
             WHERE sync_logical_timestamp IS NULL",
            timestamp,
            origin,
            device_id
        )
        .execute(&mut *conn)
        .await
        .context("Failed to stamp unranked assets")?
        .rows_affected();
        stamped += sqlx::query!(
            "UPDATE categories SET sync_logical_timestamp = ?, sync_origin = ?, sync_device_id = ?
             WHERE sync_logical_timestamp IS NULL",
            timestamp,
            origin,
            device_id
        )
        .execute(&mut *conn)
        .await
        .context("Failed to stamp unranked categories")?
        .rows_affected();
        stamped += sqlx::query!(
            "UPDATE asset_prices SET sync_logical_timestamp = ?, sync_origin = ?, sync_device_id = ?
             WHERE sync_logical_timestamp IS NULL",
            timestamp,
            origin,
            device_id
        )
        .execute(&mut *conn)
        .await
        .context("Failed to stamp unranked asset prices")?
        .rows_affected();
        Ok(stamped)
    }

    async fn get_all(&self) -> Result<Vec<Asset>> {
        let rows = sqlx::query_as!(
            AssetRow,
            r#"
            SELECT
                a.id, a.name, a.reference, a.isin, a.asset_class, a.currency, a.risk_level,
                c.id as category_id,
                c.name as category_name,
                a.is_archived as "is_archived: bool",
                a.exchange_code,
                a.price_refresh_blocked as "price_refresh_blocked: bool",
                a.interest_bearing as "interest_bearing: bool"
            FROM assets a
            JOIN categories c ON a.category_id = c.id
            WHERE a.is_deleted = 0 AND a.is_archived = 0 AND c.is_deleted = 0
            "#
        )
        .fetch_all(&self.pool)
        .await
        .with_context(|| "Failed to fetch assets from database")?;

        Ok(rows.into_iter().map(Asset::from).collect())
    }

    async fn get_all_including_archived(&self) -> Result<Vec<Asset>> {
        let rows = sqlx::query_as!(
            AssetRow,
            r#"
            SELECT
                a.id, a.name, a.reference, a.isin, a.asset_class, a.currency, a.risk_level,
                c.id as category_id,
                c.name as category_name,
                a.is_archived as "is_archived: bool",
                a.exchange_code,
                a.price_refresh_blocked as "price_refresh_blocked: bool",
                a.interest_bearing as "interest_bearing: bool"
            FROM assets a
            JOIN categories c ON a.category_id = c.id
            WHERE a.is_deleted = 0 AND c.is_deleted = 0
            "#
        )
        .fetch_all(&self.pool)
        .await
        .with_context(|| "Failed to fetch assets including archived from database")?;

        Ok(rows.into_iter().map(Asset::from).collect())
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<Asset>> {
        let row = sqlx::query_as!(
            AssetRow,
            r#"
            SELECT
                a.id, a.name, a.reference, a.isin, a.asset_class, a.currency, a.risk_level,
                c.id as category_id,
                c.name as category_name,
                a.is_archived as "is_archived: bool",
                a.exchange_code,
                a.price_refresh_blocked as "price_refresh_blocked: bool",
                a.interest_bearing as "interest_bearing: bool"
            FROM assets a
            JOIN categories c ON a.category_id = c.id
            WHERE a.id = ?
                AND a.is_deleted = 0
                AND c.is_deleted = 0
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch asset with id: {}", id))?;

        Ok(row.map(Asset::from))
    }

    async fn create(&self, asset: Asset) -> Result<Asset> {
        let asset_class_str = asset.class.to_string();
        let exchange_code = asset.exchange.as_ref().map(|e| e.code.clone());
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin asset create")?;
        sqlx::query!(
            r#"INSERT INTO assets (id, name, reference, isin, asset_class, currency, risk_level, is_deleted, is_archived, category_id, exchange_code, interest_bearing) VALUES (?, ?, ?, ?, ?, ?, ?, 0, 0, ?, ?, ?)"#,
            asset.id,
            asset.name,
            asset.reference,
            asset.isin,
            asset_class_str,
            asset.currency,
            asset.risk_level,
            asset.category.id,
            exchange_code,
            asset.interest_bearing
        )
        .execute(&mut *tx)
        .await
        .with_context(|| format!("Failed to create asset: {}", asset.name))?;
        self.record_asset_state(&mut tx, &asset, Operation::Created)
            .await?;
        tx.commit().await.context("Failed to commit asset create")?;
        Ok(asset)
    }

    async fn update(&self, asset: Asset) -> Result<Asset> {
        let asset_class_str = asset.class.to_string();
        let exchange_code = asset.exchange.as_ref().map(|e| e.code.clone());
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin asset update")?;
        let written = sqlx::query!(
            r#"UPDATE assets SET name = ?, reference = ?, isin = ?, asset_class = ?, currency = ?, risk_level = ?, category_id = ?, exchange_code = ?, interest_bearing = ? WHERE id = ? AND is_archived = 0"#,
            asset.name,
            asset.reference,
            asset.isin,
            asset_class_str,
            asset.currency,
            asset.risk_level,
            asset.category.id,
            exchange_code,
            asset.interest_bearing,
            asset.id
        )
        .execute(&mut *tx)
        .await
        .with_context(|| format!("Failed to update asset with id: {}", asset.id))?;
        if written.rows_affected() > 0 {
            self.record_asset_state(&mut tx, &asset, Operation::Updated)
                .await?;
        }
        tx.commit().await.context("Failed to commit asset update")?;
        Ok(asset)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin asset delete")?;
        let deleted = sqlx::query!(r#"UPDATE assets SET is_deleted = 1 WHERE id = ?"#, id)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("Failed to soft delete asset with id: {}", id))?;
        if deleted.rows_affected() > 0 {
            let draft = ChangeDraft::new(
                RecordKind::Asset,
                asset_identity(id),
                Operation::Removed,
                Origin::User,
                None,
                None,
            );
            self.change_recorder.record(&mut tx, draft).await?;
        }
        tx.commit().await.context("Failed to commit asset delete")?;
        Ok(())
    }

    async fn archive(&self, id: &str) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin asset archive")?;
        let written = sqlx::query!(
            r#"UPDATE assets SET is_archived = 1 WHERE id = ? AND is_deleted = 0"#,
            id
        )
        .execute(&mut *tx)
        .await
        .with_context(|| format!("Failed to archive asset with id: {}", id))?;
        self.record_updated_by_id(&mut tx, id, written.rows_affected())
            .await?;
        tx.commit()
            .await
            .context("Failed to commit asset archive")?;
        Ok(())
    }

    async fn unarchive(&self, id: &str) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin asset unarchive")?;
        let written = sqlx::query!(
            r#"UPDATE assets SET is_archived = 0 WHERE id = ? AND is_deleted = 0"#,
            id
        )
        .execute(&mut *tx)
        .await
        .with_context(|| format!("Failed to unarchive asset with id: {}", id))?;
        self.record_updated_by_id(&mut tx, id, written.rows_affected())
            .await?;
        tx.commit()
            .await
            .context("Failed to commit asset unarchive")?;
        Ok(())
    }

    async fn block_price_refresh(&self, id: &str) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin asset price-refresh block")?;
        let written = sqlx::query!(
            r#"UPDATE assets SET price_refresh_blocked = 1 WHERE id = ? AND is_deleted = 0"#,
            id
        )
        .execute(&mut *tx)
        .await
        .with_context(|| format!("Failed to block price refresh for asset with id: {}", id))?;
        self.record_updated_by_id(&mut tx, id, written.rows_affected())
            .await?;
        tx.commit()
            .await
            .context("Failed to commit asset price-refresh block")?;
        Ok(())
    }

    async fn unblock_price_refresh(&self, id: &str) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin asset price-refresh unblock")?;
        let written = sqlx::query!(
            r#"UPDATE assets SET price_refresh_blocked = 0 WHERE id = ? AND is_deleted = 0"#,
            id
        )
        .execute(&mut *tx)
        .await
        .with_context(|| format!("Failed to unblock price refresh for asset with id: {}", id))?;
        self.record_updated_by_id(&mut tx, id, written.rows_affected())
            .await?;
        tx.commit()
            .await
            .context("Failed to commit asset price-refresh unblock")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_pool() -> Pool<Sqlite> {
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

    /// Inserts a minimal active asset (FK-satisfied by the migration-seeded
    /// `default-uncategorized` category) so the column-update methods have a row.
    async fn seed_asset(pool: &Pool<Sqlite>, id: &str) {
        sqlx::query!(
            "INSERT INTO assets (id, name, reference, asset_class, currency, risk_level, category_id, is_archived)
             VALUES (?, 'Test Asset', 'REF', 'Stocks', 'USD', 3, 'default-uncategorized', 0)",
            id,
        )
        .execute(pool)
        .await
        .expect("seed asset");
    }

    // MKT-150 — block_price_refresh sets the flag; get_by_id reflects it.
    #[tokio::test]
    async fn block_price_refresh_sets_flag_and_round_trips() {
        let pool = setup_pool().await;
        seed_asset(&pool, "a1").await;
        let repo = SqliteAssetRepository::new(pool);

        let before = repo.get_by_id("a1").await.unwrap().unwrap();
        assert!(
            !before.price_refresh_blocked,
            "seeded asset starts unlocked"
        );

        repo.block_price_refresh("a1").await.unwrap();

        let after = repo.get_by_id("a1").await.unwrap().unwrap();
        assert!(after.price_refresh_blocked);
    }

    /// Builds an active asset for the round-trip tests, FK-satisfied by the
    /// migration-seeded `default-uncategorized` category.
    fn asset_with_interest_bearing(id: &str, interest_bearing: bool) -> Asset {
        Asset::restore(
            id.to_string(),
            "Euro Fund".to_string(),
            AssetClass::MutualFunds,
            AssetCategory::from_storage(
                "default-uncategorized".to_string(),
                "Uncategorized".to_string(),
            ),
            "EUR".to_string(),
            2,
            "EUROFUND".to_string(),
            None,
            false,
            None,
            false,
            interest_bearing,
        )
    }

    // AST-024 — interest_bearing round-trips through create → get_by_id → update.
    #[tokio::test]
    async fn interest_bearing_round_trips_through_create_and_update() {
        let pool = setup_pool().await;
        let repo = SqliteAssetRepository::new(pool);

        repo.create(asset_with_interest_bearing("a1", true))
            .await
            .unwrap();
        let created = repo.get_by_id("a1").await.unwrap().unwrap();
        assert!(created.interest_bearing);

        repo.update(asset_with_interest_bearing("a1", false))
            .await
            .unwrap();
        let updated = repo.get_by_id("a1").await.unwrap().unwrap();
        assert!(!updated.interest_bearing);
    }

    // AST-024 — a raw seeded row (no interest_bearing column value) loads with
    // the migration default of false.
    #[tokio::test]
    async fn seeded_asset_defaults_to_not_interest_bearing() {
        let pool = setup_pool().await;
        seed_asset(&pool, "a1").await;
        let repo = SqliteAssetRepository::new(pool);

        let loaded = repo.get_by_id("a1").await.unwrap().unwrap();
        assert!(!loaded.interest_bearing);
    }

    // MKT-156 — unblock_price_refresh clears the flag set by block.
    #[tokio::test]
    async fn unblock_price_refresh_clears_flag() {
        let pool = setup_pool().await;
        seed_asset(&pool, "a1").await;
        let repo = SqliteAssetRepository::new(pool);

        repo.block_price_refresh("a1").await.unwrap();
        repo.unblock_price_refresh("a1").await.unwrap();

        let after = repo.get_by_id("a1").await.unwrap().unwrap();
        assert!(!after.price_refresh_blocked);
    }

    use crate::context::sync::SqliteChangeRecorder;
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

    async fn changes_with_operation(pool: &Pool<Sqlite>, operation: &str) -> i64 {
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM changes WHERE operation = ?",
            operation
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    fn test_asset(id: &str) -> Asset {
        Asset::restore(
            id.to_string(),
            "Euro Fund".to_string(),
            AssetClass::MutualFunds,
            AssetCategory::from_storage(
                "default-uncategorized".to_string(),
                "Uncategorized".to_string(),
            ),
            "EUR".to_string(),
            2,
            "EUROFUND".to_string(),
            None,
            false,
            None,
            false,
            false,
        )
    }

    // SYN-020/021 — create records exactly one Created change, rank-stamped.
    #[tokio::test]
    async fn create_records_one_created_change_with_rank_stamped() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAssetRepository::new(pool.clone()).with_change_recorder(recorder);

        repo.create(test_asset("a1")).await.unwrap();

        assert_eq!(changes_with_operation(&pool, "Created").await, 1);
        let row = sqlx::query!("SELECT sync_logical_timestamp FROM assets WHERE id = 'a1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(row.sync_logical_timestamp.is_some());
    }

    // CFR-014/D6 — stamp_sync_rank ranks only the rows that were never ranked; a row the
    // recorder already ranked keeps its own rank.
    #[tokio::test]
    async fn stamp_sync_rank_stamps_only_unranked_rows() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        SqliteAssetRepository::new(pool.clone())
            .create(test_asset("a-unranked"))
            .await
            .unwrap();
        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAssetRepository::new(pool.clone()).with_change_recorder(recorder);
        repo.create(test_asset("a-ranked")).await.unwrap();

        let rank = Rank {
            origin: Origin::User,
            logical_timestamp: crate::shared::domain::LogicalTimestamp::new(99),
            device_id: "desktop-device".to_string(),
        };
        let mut conn = pool.acquire().await.unwrap();
        let stamped = repo.stamp_sync_rank(&mut conn, &rank).await.unwrap();
        drop(conn);
        assert!(stamped >= 1, "the unranked asset is stamped: {stamped}");

        let rows = sqlx::query!("SELECT id, sync_logical_timestamp FROM assets")
            .fetch_all(&pool)
            .await
            .unwrap();
        let stamp_of = |id: &str| {
            rows.iter()
                .find(|row| row.id == id)
                .and_then(|row| row.sync_logical_timestamp.clone())
        };
        assert_eq!(
            stamp_of("a-unranked").as_deref(),
            Some("00000000000000000099")
        );
        assert_ne!(
            stamp_of("a-ranked").as_deref(),
            Some("00000000000000000099"),
            "a row the recorder ranked keeps its rank"
        );
        let unranked_categories: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM categories WHERE sync_logical_timestamp IS NULL"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(unranked_categories, 0, "categories are stamped too");
    }

    // SYN-020 — update records exactly one Updated change.
    #[tokio::test]
    async fn update_records_one_updated_change() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let setup_repo = SqliteAssetRepository::new(pool.clone());
        setup_repo.create(test_asset("a1")).await.unwrap();

        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAssetRepository::new(pool.clone()).with_change_recorder(recorder);
        let mut renamed = test_asset("a1");
        renamed.name = "Euro Fund Renamed".to_string();
        repo.update(renamed).await.unwrap();

        assert_eq!(changes_with_operation(&pool, "Updated").await, 1);
    }

    // SYN-020/024 — delete (soft-delete) records exactly one Removed change + tombstone.
    #[tokio::test]
    async fn delete_records_one_removed_change_and_tombstone() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let setup_repo = SqliteAssetRepository::new(pool.clone());
        setup_repo.create(test_asset("a1")).await.unwrap();

        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAssetRepository::new(pool.clone()).with_change_recorder(recorder);
        repo.delete("a1").await.unwrap();

        assert_eq!(changes_with_operation(&pool, "Removed").await, 1);
        let tombstone = sqlx::query!(
            "SELECT record_identity FROM tombstones WHERE record_kind = 'Asset' AND record_identity = 'a1'"
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert!(tombstone.is_some());
    }

    // SYN-020 — archive records exactly one Updated change (a state field, not a removal).
    #[tokio::test]
    async fn archive_records_one_updated_change() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let setup_repo = SqliteAssetRepository::new(pool.clone());
        setup_repo.create(test_asset("a1")).await.unwrap();

        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAssetRepository::new(pool.clone()).with_change_recorder(recorder);
        repo.archive("a1").await.unwrap();

        assert_eq!(changes_with_operation(&pool, "Updated").await, 1);
    }

    // SYN-020 — unarchive records exactly one Updated change.
    #[tokio::test]
    async fn unarchive_records_one_updated_change() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let setup_repo = SqliteAssetRepository::new(pool.clone());
        setup_repo.create(test_asset("a1")).await.unwrap();
        setup_repo.archive("a1").await.unwrap();

        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAssetRepository::new(pool.clone()).with_change_recorder(recorder);
        repo.unarchive("a1").await.unwrap();

        assert_eq!(changes_with_operation(&pool, "Updated").await, 1);
    }

    // SYN-020 — block_price_refresh records exactly one Updated change.
    #[tokio::test]
    async fn block_price_refresh_records_one_updated_change() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let setup_repo = SqliteAssetRepository::new(pool.clone());
        setup_repo.create(test_asset("a1")).await.unwrap();

        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAssetRepository::new(pool.clone()).with_change_recorder(recorder);
        repo.block_price_refresh("a1").await.unwrap();

        assert_eq!(changes_with_operation(&pool, "Updated").await, 1);
    }

    // SYN-020 — unblock_price_refresh records exactly one Updated change.
    #[tokio::test]
    async fn unblock_price_refresh_records_one_updated_change() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let setup_repo = SqliteAssetRepository::new(pool.clone());
        setup_repo.create(test_asset("a1")).await.unwrap();
        setup_repo.block_price_refresh("a1").await.unwrap();

        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAssetRepository::new(pool.clone()).with_change_recorder(recorder);
        repo.unblock_price_refresh("a1").await.unwrap();

        assert_eq!(changes_with_operation(&pool, "Updated").await, 1);
    }

    // SYN-020 — a failed write records no change (rollback).
    #[tokio::test]
    async fn create_rolls_back_change_when_the_write_fails() {
        let pool = make_pool().await;
        seed_sync_device(&pool).await;
        let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
        let repo = SqliteAssetRepository::new(pool.clone()).with_change_recorder(recorder);
        repo.create(test_asset("a1")).await.unwrap();

        // Same id — PRIMARY KEY violation, the whole write must fail atomically.
        let result = repo.create(test_asset("a1")).await;
        assert!(result.is_err());

        assert_eq!(
            changes_with_operation(&pool, "Created").await,
            1,
            "only the first (successful) create recorded a change"
        );
    }
}
