use std::{fs, path::PathBuf};

use anyhow::Context;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    ConnectOptions, Pool, Sqlite,
};

use crate::core::logger::BACKEND;

const DATABASE_FILENAME: &str = "portfolio";

/// Manages the SQLite database connection and migrations.
pub struct Database {
    /// The underlying SQLx connection pool.
    pub pool: Pool<Sqlite>,
}

impl Database {
    /// Initializes the database at the specified path and runs pending migrations.
    pub async fn new(app_data_dir: PathBuf) -> anyhow::Result<Self> {
        // Check if database reset is requested
        let is_db_reset = std::env::var("RESET_DATABASE")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or_default();

        let db_path = app_data_dir.join(DATABASE_FILENAME);
        if !db_path.exists() {
            fs::File::create(&db_path)
                .with_context(|| format!("Failed to create database file {:?}", db_path))?;
        }

        // Handle database reset if requested
        if is_db_reset {
            tracing::warn!("RESET_DATABASE is set - deleting existing database");
            if db_path.exists() {
                fs::remove_file(&db_path).with_context(|| "Failed to delete database")?;
                tracing::info!("Database deleted successfully");
            } else {
                tracing::info!("Database does not exist, skipping delete");
            }
        }

        tracing::trace!(target: BACKEND, "Connecting to database: {}", db_path.to_string_lossy());

        let connect_options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .foreign_keys(true)
            .disable_statement_logging();

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connect_options)
            .await
            .with_context(|| format!("Failed to connect to SQLite at {:?}", db_path))?;

        let db = Database { pool };

        // Initialize tables via migrations
        sqlx::migrate!("./migrations")
            .run(&db.pool)
            .await
            .with_context(|| "Failed to run database migrations")?;

        Ok(db)
    }
}
