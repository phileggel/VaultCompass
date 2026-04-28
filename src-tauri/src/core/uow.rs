//! Unit of Work infrastructure for atomic cross-aggregate writes.
//!
//! Phase 5 lays the foundation. The `run` method and per-use-case
//! `AppUnitOfWork` integration are added when the first use case needs them.

use std::future::Future;
use std::pin::Pin;

use anyhow::Result;
use sqlx::{Pool, Sqlite};

/// Pinned boxed future for UoW operation closures.
pub type UoWFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// Runs operations atomically inside a database transaction.
/// Commits on `Ok`, rolls back on `Err`.
/// Use cases define their own `AppUnitOfWork` super-trait; the `run` method
/// will be added here when the first cross-aggregate use case requires it.
pub trait TransactionManager: Send + Sync {}

/// SQLite-backed implementation of [`TransactionManager`].
/// Created once at startup and injected into use cases that need cross-aggregate atomicity.
pub struct SqlxTransactionManager {
    /// Connection pool used to begin transactions when `run` is implemented.
    #[allow(dead_code)]
    pub(crate) pool: Pool<Sqlite>,
}

impl SqlxTransactionManager {
    /// Create a new manager backed by `pool`.
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

impl TransactionManager for SqlxTransactionManager {}
