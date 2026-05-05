use crate::context::account::{AccountService, OpeningBalanceDomainError, Transaction};
use crate::context::asset::AssetService;
use anyhow::Result;
use std::sync::Arc;

/// Orchestrates the Opening Balance operation across the asset and account bounded contexts.
///
/// Validates asset existence and archived status (asset BC) before delegating the
/// actual holding creation to AccountService (account BC) — TRX-050, TRX-056.
pub struct OpenHoldingUseCase {
    account_service: Arc<AccountService>,
    asset_service: Arc<AssetService>,
}

impl OpenHoldingUseCase {
    /// Creates a new OpenHoldingUseCase.
    pub fn new(account_service: Arc<AccountService>, asset_service: Arc<AssetService>) -> Self {
        Self {
            account_service,
            asset_service,
        }
    }

    /// Seeds a holding from a known quantity and total cost (TRX-042).
    ///
    /// Cross-BC guard: rejects the request if the asset does not exist or is archived.
    /// Delegates the account-side write to AccountService::open_holding.
    pub async fn open_holding(
        &self,
        account_id: &str,
        asset_id: String,
        date: String,
        quantity: i64,
        total_cost: i64,
    ) -> Result<Transaction> {
        match self.asset_service.get_asset_by_id(&asset_id).await? {
            None => return Err(OpeningBalanceDomainError::AssetNotFound.into()),
            Some(a) if a.is_archived => return Err(OpeningBalanceDomainError::ArchivedAsset.into()),
            Some(_) => {}
        }
        self.account_service
            .open_holding(account_id, asset_id, date, quantity, total_cost)
            .await
    }
}

// AssetService field is reserved for the upcoming Cash Asset seeding step (CSH-010)
// in the buy/sell/correct/cancel use cases; the post-refactor cash-tracking work will
// invoke `ensure_cash_asset` here before delegating to AccountService.

/// Orchestrates the Buy operation. Currently delegates straight to `AccountService`;
/// the cross-BC `ensure_cash_asset` step is wired in by the cash-tracking spec (CSH-040).
pub struct BuyHoldingUseCase {
    account_service: Arc<AccountService>,
    #[allow(dead_code)]
    asset_service: Arc<AssetService>,
}

impl BuyHoldingUseCase {
    /// Creates a new BuyHoldingUseCase.
    pub fn new(account_service: Arc<AccountService>, asset_service: Arc<AssetService>) -> Self {
        Self {
            account_service,
            asset_service,
        }
    }

    /// Records a purchase of an asset into an account (TRX-027).
    #[allow(clippy::too_many_arguments)]
    pub async fn buy_holding(
        &self,
        account_id: &str,
        asset_id: String,
        date: String,
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        fees: i64,
        note: Option<String>,
    ) -> Result<Transaction> {
        self.account_service
            .buy_holding(
                account_id,
                asset_id,
                date,
                quantity,
                unit_price,
                exchange_rate,
                fees,
                note,
            )
            .await
    }
}

/// Orchestrates the Sell operation. Currently delegates straight to `AccountService`;
/// the cross-BC `ensure_cash_asset` step is wired in by the cash-tracking spec (CSH-050).
pub struct SellHoldingUseCase {
    account_service: Arc<AccountService>,
    #[allow(dead_code)]
    asset_service: Arc<AssetService>,
}

impl SellHoldingUseCase {
    /// Creates a new SellHoldingUseCase.
    pub fn new(account_service: Arc<AccountService>, asset_service: Arc<AssetService>) -> Self {
        Self {
            account_service,
            asset_service,
        }
    }

    /// Records a sale of an asset from an account (SEL-012, SEL-021, SEL-023, SEL-024).
    #[allow(clippy::too_many_arguments)]
    pub async fn sell_holding(
        &self,
        account_id: &str,
        asset_id: String,
        date: String,
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        fees: i64,
        note: Option<String>,
    ) -> Result<Transaction> {
        self.account_service
            .sell_holding(
                account_id,
                asset_id,
                date,
                quantity,
                unit_price,
                exchange_rate,
                fees,
                note,
            )
            .await
    }
}

/// Orchestrates the Correct (edit) operation. Currently delegates straight to `AccountService`;
/// the cross-BC `ensure_cash_asset` step is wired in by the cash-tracking spec (CSH-042).
pub struct CorrectTransactionUseCase {
    account_service: Arc<AccountService>,
    #[allow(dead_code)]
    asset_service: Arc<AssetService>,
}

impl CorrectTransactionUseCase {
    /// Creates a new CorrectTransactionUseCase.
    pub fn new(account_service: Arc<AccountService>, asset_service: Arc<AssetService>) -> Self {
        Self {
            account_service,
            asset_service,
        }
    }

    /// Corrects an existing transaction and recalculates the affected holding (TRX-031).
    #[allow(clippy::too_many_arguments)]
    pub async fn correct_transaction(
        &self,
        account_id: &str,
        transaction_id: &str,
        date: String,
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        fees: i64,
        note: Option<String>,
    ) -> Result<Transaction> {
        self.account_service
            .correct_transaction(
                account_id,
                transaction_id,
                date,
                quantity,
                unit_price,
                exchange_rate,
                fees,
                note,
            )
            .await
    }
}

/// Orchestrates the Cancel (delete) operation. Currently delegates straight to `AccountService`;
/// the cash replay-eligibility step is wired in by the cash-tracking spec (CSH-024 / CSH-051).
pub struct CancelTransactionUseCase {
    account_service: Arc<AccountService>,
    #[allow(dead_code)]
    asset_service: Arc<AssetService>,
}

impl CancelTransactionUseCase {
    /// Creates a new CancelTransactionUseCase.
    pub fn new(account_service: Arc<AccountService>, asset_service: Arc<AssetService>) -> Self {
        Self {
            account_service,
            asset_service,
        }
    }

    /// Cancels a transaction and recalculates (or removes) the associated holding (TRX-034).
    pub async fn cancel_transaction(&self, account_id: &str, transaction_id: &str) -> Result<()> {
        self.account_service
            .cancel_transaction(account_id, transaction_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::account::{
        AccountService, SqliteAccountRepository, SqliteHoldingRepository,
        SqliteTransactionRepository, UpdateFrequency,
    };
    use crate::context::asset::{
        AssetClass, AssetService, CreateAssetDTO, SqliteAssetCategoryRepository,
        SqliteAssetPriceRepository, SqliteAssetRepository, SYSTEM_CATEGORY_ID,
    };
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_pool() -> sqlx::Pool<sqlx::Sqlite> {
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

    fn make_services(pool: &sqlx::Pool<sqlx::Sqlite>) -> (Arc<AccountService>, Arc<AssetService>) {
        let account_svc = Arc::new(AccountService::new(
            Box::new(SqliteAccountRepository::new(pool.clone())),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
            Box::new(SqliteTransactionRepository::new(pool.clone())),
        ));
        let asset_svc = Arc::new(AssetService::new(
            Box::new(SqliteAssetRepository::new(pool.clone())),
            Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
            Box::new(SqliteAssetPriceRepository::new(pool.clone())),
        ));
        (account_svc, asset_svc)
    }

    fn base_asset_dto() -> CreateAssetDTO {
        CreateAssetDTO {
            name: "Test Asset".to_string(),
            reference: "TST".to_string(),
            class: AssetClass::Stocks,
            currency: "USD".to_string(),
            risk_level: 1,
            category_id: SYSTEM_CATEGORY_ID.to_string(),
        }
    }

    fn micro(v: i64) -> i64 {
        v * 1_000_000
    }

    // TRX-056 — AssetNotFound when asset does not exist
    #[tokio::test]
    async fn open_holding_rejects_unknown_asset() {
        let pool = setup_pool().await;
        let (account_svc, asset_svc) = make_services(&pool);
        let account = account_svc
            .create(
                "Acc".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();

        let uc = OpenHoldingUseCase::new(account_svc, asset_svc);
        let err = uc
            .open_holding(
                &account.id,
                "nonexistent-asset".to_string(),
                "2024-01-01".to_string(),
                micro(1),
                micro(100),
            )
            .await
            .unwrap_err();

        assert!(
            err.downcast_ref::<OpeningBalanceDomainError>()
                .map(|e| matches!(e, OpeningBalanceDomainError::AssetNotFound))
                .unwrap_or(false),
            "expected AssetNotFound, got: {err}"
        );
    }

    // TRX-050 — ArchivedAsset when asset is archived
    #[tokio::test]
    async fn open_holding_rejects_archived_asset() {
        let pool = setup_pool().await;
        let (account_svc, asset_svc) = make_services(&pool);
        let asset = asset_svc.create_asset(base_asset_dto()).await.unwrap();
        asset_svc.archive_asset(&asset.id).await.unwrap();
        let account = account_svc
            .create(
                "Acc".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();

        let uc = OpenHoldingUseCase::new(account_svc, asset_svc);
        let err = uc
            .open_holding(
                &account.id,
                asset.id,
                "2024-01-01".to_string(),
                micro(1),
                micro(100),
            )
            .await
            .unwrap_err();

        assert!(
            err.downcast_ref::<OpeningBalanceDomainError>()
                .map(|e| matches!(e, OpeningBalanceDomainError::ArchivedAsset))
                .unwrap_or(false),
            "expected ArchivedAsset, got: {err}"
        );
    }

    // TRX-047 — happy path: transaction and holding created with correct fields
    #[tokio::test]
    async fn open_holding_happy_path() {
        use crate::context::account::TransactionType;

        let pool = setup_pool().await;
        let (account_svc, asset_svc) = make_services(&pool);
        let asset = asset_svc.create_asset(base_asset_dto()).await.unwrap();
        let account = account_svc
            .create(
                "Acc".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();

        let uc = OpenHoldingUseCase::new(Arc::clone(&account_svc), asset_svc);
        let tx = uc
            .open_holding(
                &account.id,
                asset.id.clone(),
                "2024-01-01".to_string(),
                micro(2),
                micro(200),
            )
            .await
            .unwrap();

        assert_eq!(tx.transaction_type, TransactionType::OpeningBalance);
        assert_eq!(tx.total_amount, micro(200));
        assert_eq!(tx.fees, 0);
        assert_eq!(tx.exchange_rate, 1_000_000);
        assert_eq!(tx.unit_price, micro(100));

        let holdings = account_svc
            .get_holdings_for_account(&account.id)
            .await
            .unwrap();
        assert_eq!(holdings.len(), 1);
        assert_eq!(holdings[0].quantity, micro(2));
        assert_eq!(holdings[0].average_price, micro(100));
    }
}
