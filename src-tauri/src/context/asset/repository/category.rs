use super::super::domain::{AssetCategory, AssetCategoryRepository};
use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, Transaction};

#[derive(sqlx::FromRow)]
struct CategoryRow {
    id: String,
    name: String,
}

impl From<CategoryRow> for AssetCategory {
    fn from(row: CategoryRow) -> Self {
        AssetCategory::from_storage(row.id, row.name)
    }
}

/// SQLite implementation of the AssetCategoryRepository.
#[derive(Clone)]
pub struct SqliteAssetCategoryRepository {
    pool: Pool<Sqlite>,
}

impl SqliteAssetCategoryRepository {
    /// Creates a new SqliteAssetCategoryRepository.
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl AssetCategoryRepository for SqliteAssetCategoryRepository {
    async fn get_all(&self) -> Result<Vec<AssetCategory>> {
        let categories = sqlx::query_as!(
            CategoryRow,
            r#"
            SELECT id, name
            FROM categories 
            WHERE is_deleted = 0
            "#
        )
        .fetch_all(&self.pool)
        .await
        .with_context(|| "Failed to fetch all categories")?;

        Ok(categories.into_iter().map(AssetCategory::from).collect())
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<AssetCategory>> {
        let row = sqlx::query_as!(
            CategoryRow,
            r#"
            SELECT id, name
            FROM categories
            WHERE LOWER(name) = LOWER(?) AND is_deleted = 0
            "#,
            name
        )
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to find category by name: {}", name))?;

        Ok(row.map(AssetCategory::from))
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<AssetCategory>> {
        let row = sqlx::query_as!(
            CategoryRow,
            r#"
            SELECT id, name
            FROM categories 
            WHERE id = ? AND is_deleted = 0
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch category with id {}", id))?;

        Ok(row.map(AssetCategory::from))
    }

    async fn create(&self, category: AssetCategory) -> Result<AssetCategory> {
        sqlx::query!(
            r#"INSERT INTO categories (id, name, is_deleted) VALUES (?, ?, 0)"#,
            category.id,
            category.name,
        )
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to create category: {}", category.name))?;

        Ok(category)
    }

    async fn update(&self, category: AssetCategory) -> Result<AssetCategory> {
        sqlx::query!(
            r#"UPDATE categories SET name = ? WHERE id = ?"#,
            category.name,
            category.id
        )
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to update category: {}", category.id))?;

        Ok(category)
    }

    async fn reassign_assets_and_delete(&self, category_id: &str, fallback_id: &str) -> Result<()> {
        let mut tx: Transaction<Sqlite> = self.pool.begin().await?;

        sqlx::query!(
            r#"UPDATE assets SET category_id = ? WHERE category_id = ? AND is_deleted = 0"#,
            fallback_id,
            category_id
        )
        .execute(&mut *tx)
        .await
        .with_context(|| format!("Failed to reassign assets from category: {}", category_id))?;

        sqlx::query!(
            r#"UPDATE categories SET is_deleted = 1 WHERE id = ?"#,
            category_id
        )
        .execute(&mut *tx)
        .await
        .with_context(|| format!("Failed to delete category: {}", category_id))?;

        tx.commit().await?;
        Ok(())
    }
}
