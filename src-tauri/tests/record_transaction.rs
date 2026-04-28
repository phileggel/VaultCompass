use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use vault_compass_lib::context::account::{
    AccountOperationError, AccountService, SqliteAccountRepository, SqliteHoldingRepository,
    SqliteTransactionRepository, UpdateFrequency,
};
use vault_compass_lib::context::asset::{
    AssetClass, AssetService, CreateAssetDTO, SqliteAssetCategoryRepository,
    SqliteAssetPriceRepository, SqliteAssetRepository, SYSTEM_CATEGORY_ID,
};
use vault_compass_lib::use_cases::record_transaction::{
    CreateTransactionDTO, RecordTransactionError, RecordTransactionUseCase,
};

async fn make_pool() -> sqlx::Pool<sqlx::Sqlite> {
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

async fn setup_uc(pool: &sqlx::Pool<sqlx::Sqlite>) -> RecordTransactionUseCase {
    let account_service = Arc::new(AccountService::new(
        Box::new(SqliteAccountRepository::new(pool.clone())),
        Box::new(SqliteHoldingRepository::new(pool.clone())),
        Box::new(SqliteTransactionRepository::new(pool.clone())),
    ));
    let asset_service = Arc::new(AssetService::new(
        Box::new(SqliteAssetRepository::new(pool.clone())),
        Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
        Box::new(SqliteAssetPriceRepository::new(pool.clone())),
    ));
    RecordTransactionUseCase::new(account_service, asset_service)
}

async fn create_account(pool: &sqlx::Pool<sqlx::Sqlite>) -> String {
    AccountService::new(
        Box::new(SqliteAccountRepository::new(pool.clone())),
        Box::new(SqliteHoldingRepository::new(pool.clone())),
        Box::new(SqliteTransactionRepository::new(pool.clone())),
    )
    .create(
        "Test Account".to_string(),
        "EUR".to_string(),
        UpdateFrequency::ManualMonth,
    )
    .await
    .unwrap()
    .id
}

async fn create_asset(pool: &sqlx::Pool<sqlx::Sqlite>) -> String {
    AssetService::new(
        Box::new(SqliteAssetRepository::new(pool.clone())),
        Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
        Box::new(SqliteAssetPriceRepository::new(pool.clone())),
    )
    .create_asset(CreateAssetDTO {
        name: "AAPL".to_string(),
        reference: "AAPL".to_string(),
        class: AssetClass::Stocks,
        currency: "USD".to_string(),
        risk_level: 3,
        category_id: SYSTEM_CATEGORY_ID.to_string(),
    })
    .await
    .unwrap()
    .id
}

fn buy_dto(account_id: &str, asset_id: &str, qty: i64) -> CreateTransactionDTO {
    let micro = 1_000_000i64;
    CreateTransactionDTO {
        account_id: account_id.to_string(),
        asset_id: asset_id.to_string(),
        transaction_type: "Purchase".to_string(),
        date: "2024-01-01".to_string(),
        quantity: qty,
        unit_price: 100 * micro,
        exchange_rate: micro,
        fees: 0,
        note: None,
        record_price: false,
    }
}

fn sell_dto(account_id: &str, asset_id: &str, qty: i64) -> CreateTransactionDTO {
    let micro = 1_000_000i64;
    CreateTransactionDTO {
        account_id: account_id.to_string(),
        asset_id: asset_id.to_string(),
        transaction_type: "Sell".to_string(),
        date: "2024-06-01".to_string(),
        quantity: qty,
        unit_price: 120 * micro,
        exchange_rate: micro,
        fees: 0,
        note: None,
        record_price: false,
    }
}

// SEL-012 — selling when holding quantity is 0 is rejected (error now from AccountOperationError)
#[tokio::test]
async fn sell_rejected_when_no_holding() {
    let pool = make_pool().await;
    let uc = setup_uc(&pool).await;
    let account_id = create_account(&pool).await;
    let asset_id = create_asset(&pool).await;

    let err = uc
        .create_transaction(sell_dto(&account_id, &asset_id, 1_000_000))
        .await
        .unwrap_err();
    assert!(
        err.downcast_ref::<AccountOperationError>()
            .map(|e| matches!(e, AccountOperationError::ClosedPosition))
            .unwrap_or(false),
        "got: {err}"
    );
}

// SEL-026 — when full position is sold, holding is retained at quantity=0 with last VWAP preserved
#[tokio::test]
async fn full_sell_retains_holding_at_zero_with_last_vwap() {
    let pool = make_pool().await;
    let uc = setup_uc(&pool).await;
    let account_id = create_account(&pool).await;
    let asset_id = create_asset(&pool).await;
    let micro = 1_000_000i64;

    uc.create_transaction(buy_dto(&account_id, &asset_id, 2 * micro))
        .await
        .unwrap();
    uc.create_transaction(sell_dto(&account_id, &asset_id, 2 * micro))
        .await
        .unwrap();

    let svc = AccountService::new(
        Box::new(SqliteAccountRepository::new(pool.clone())),
        Box::new(SqliteHoldingRepository::new(pool.clone())),
        Box::new(SqliteTransactionRepository::new(pool.clone())),
    );
    let holdings = svc.get_holdings_for_account(&account_id).await.unwrap();
    let h = holdings.iter().find(|h| h.asset_id == asset_id).unwrap();
    assert_eq!(h.quantity, 0, "holding should be retained at qty=0");
    assert_eq!(h.average_price, 100 * micro, "VWAP should be preserved");
}

// SEL-032 — editing a purchase so it creates an oversell on a subsequent sell is rejected
#[tokio::test]
async fn edit_purchase_rejected_when_causes_oversell() {
    let pool = make_pool().await;
    let uc = setup_uc(&pool).await;
    let account_id = create_account(&pool).await;
    let asset_id = create_asset(&pool).await;
    let micro = 1_000_000i64;

    let buy = uc
        .create_transaction(buy_dto(&account_id, &asset_id, 3 * micro))
        .await
        .unwrap();
    uc.create_transaction(sell_dto(&account_id, &asset_id, 2 * micro))
        .await
        .unwrap();

    let mut reduced_buy = buy_dto(&account_id, &asset_id, micro);
    reduced_buy.transaction_type = "Purchase".to_string();
    let err = uc
        .update_transaction(buy.id, reduced_buy)
        .await
        .unwrap_err();
    assert!(
        err.downcast_ref::<AccountOperationError>()
            .map(|e| matches!(e, AccountOperationError::CascadingOversell))
            .unwrap_or(false),
        "got: {err}"
    );
}

// SEL-037 — creating a sell on an archived asset is rejected
#[tokio::test]
async fn create_sell_rejected_when_asset_archived() {
    let pool = make_pool().await;
    let uc = setup_uc(&pool).await;
    let account_id = create_account(&pool).await;
    let asset_id = create_asset(&pool).await;
    let micro = 1_000_000i64;

    uc.create_transaction(buy_dto(&account_id, &asset_id, 2 * micro))
        .await
        .unwrap();

    sqlx::query("UPDATE assets SET is_archived = TRUE WHERE id = ?")
        .bind(&asset_id)
        .execute(&pool)
        .await
        .unwrap();

    let err = uc
        .create_transaction(sell_dto(&account_id, &asset_id, micro))
        .await
        .unwrap_err();
    assert!(
        matches!(
            err.downcast_ref::<RecordTransactionError>(),
            Some(RecordTransactionError::ArchivedAssetSell)
        ),
        "got: {err}"
    );
}

// SEL-037 / TRX-033 — editing a sell on an archived asset is rejected
#[tokio::test]
async fn update_sell_rejected_when_asset_archived() {
    let pool = make_pool().await;
    let uc = setup_uc(&pool).await;
    let account_id = create_account(&pool).await;
    let asset_id = create_asset(&pool).await;
    let micro = 1_000_000i64;

    uc.create_transaction(buy_dto(&account_id, &asset_id, 2 * micro))
        .await
        .unwrap();
    let sell = uc
        .create_transaction(sell_dto(&account_id, &asset_id, micro))
        .await
        .unwrap();

    sqlx::query("UPDATE assets SET is_archived = TRUE WHERE id = ?")
        .bind(&asset_id)
        .execute(&pool)
        .await
        .unwrap();

    let err = uc
        .update_transaction(sell.id, sell_dto(&account_id, &asset_id, micro))
        .await
        .unwrap_err();
    assert!(
        matches!(
            err.downcast_ref::<RecordTransactionError>(),
            Some(RecordTransactionError::ArchivedAssetSell)
        ),
        "got: {err}"
    );
}

// SEL-035 — changing transaction_type on update is rejected
#[tokio::test]
async fn update_rejects_transaction_type_change() {
    let pool = make_pool().await;
    let uc = setup_uc(&pool).await;
    let account_id = create_account(&pool).await;
    let asset_id = create_asset(&pool).await;
    let micro = 1_000_000i64;

    let buy = uc
        .create_transaction(buy_dto(&account_id, &asset_id, micro))
        .await
        .unwrap();

    let mut sell_edit = sell_dto(&account_id, &asset_id, micro);
    sell_edit.transaction_type = "Sell".to_string();
    let err = uc.update_transaction(buy.id, sell_edit).await.unwrap_err();
    assert!(
        matches!(
            err.downcast_ref::<RecordTransactionError>(),
            Some(RecordTransactionError::TypeImmutable)
        ),
        "got: {err}"
    );
}

// MKT-055 — create_purchase with record_price=true writes an AssetPrice row
#[tokio::test]
async fn create_purchase_with_record_price_writes_asset_price() {
    let pool = make_pool().await;
    let uc = setup_uc(&pool).await;
    let account_id = create_account(&pool).await;
    let asset_id = create_asset(&pool).await;
    let micro = 1_000_000i64;

    let dto = CreateTransactionDTO {
        record_price: true,
        ..buy_dto(&account_id, &asset_id, micro)
    };
    let expected_price = dto.unit_price;
    let expected_date = dto.date.clone();

    uc.create_transaction(dto).await.unwrap();

    let rows = sqlx::query!(
        "SELECT asset_id, date, price FROM asset_prices WHERE asset_id = ?",
        asset_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 1, "expected exactly one AssetPrice row");
    assert_eq!(rows[0].asset_id, asset_id);
    assert_eq!(rows[0].date, expected_date);
    assert_eq!(rows[0].price, expected_price);
}

// MKT-055 — create_sell with record_price=true writes an AssetPrice row at the sell date/price
#[tokio::test]
async fn create_sell_with_record_price_writes_asset_price() {
    let pool = make_pool().await;
    let uc = setup_uc(&pool).await;
    let account_id = create_account(&pool).await;
    let asset_id = create_asset(&pool).await;
    let micro = 1_000_000i64;

    uc.create_transaction(buy_dto(&account_id, &asset_id, 2 * micro))
        .await
        .unwrap();

    let sell = CreateTransactionDTO {
        record_price: true,
        ..sell_dto(&account_id, &asset_id, micro)
    };
    let expected_price = sell.unit_price;
    let expected_date = sell.date.clone();

    uc.create_transaction(sell).await.unwrap();

    let rows = sqlx::query!(
        "SELECT asset_id, date, price FROM asset_prices WHERE asset_id = ?",
        asset_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(
        rows.len(),
        1,
        "expected exactly one AssetPrice row from the sell"
    );
    assert_eq!(rows[0].asset_id, asset_id);
    assert_eq!(rows[0].date, expected_date);
    assert_eq!(rows[0].price, expected_price);
}

// MKT-055 — update_transaction with record_price=true writes an AssetPrice row
#[tokio::test]
async fn update_transaction_with_record_price_writes_asset_price() {
    let pool = make_pool().await;
    let uc = setup_uc(&pool).await;
    let account_id = create_account(&pool).await;
    let asset_id = create_asset(&pool).await;
    let micro = 1_000_000i64;

    let tx = uc
        .create_transaction(buy_dto(&account_id, &asset_id, micro))
        .await
        .unwrap();

    let new_unit_price = 150 * micro;
    let update_dto = CreateTransactionDTO {
        record_price: true,
        unit_price: new_unit_price,
        ..buy_dto(&account_id, &asset_id, micro)
    };
    let expected_date = update_dto.date.clone();

    uc.update_transaction(tx.id, update_dto).await.unwrap();

    let rows = sqlx::query!(
        "SELECT asset_id, date, price FROM asset_prices WHERE asset_id = ?",
        asset_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(
        rows.len(),
        1,
        "expected exactly one AssetPrice row after update"
    );
    assert_eq!(rows[0].asset_id, asset_id);
    assert_eq!(rows[0].date, expected_date);
    assert_eq!(rows[0].price, new_unit_price);
}

// MKT-054 — record_price=false does NOT write any AssetPrice row
#[tokio::test]
async fn record_price_false_does_not_write_asset_price() {
    let pool = make_pool().await;
    let uc = setup_uc(&pool).await;
    let account_id = create_account(&pool).await;
    let asset_id = create_asset(&pool).await;
    let micro = 1_000_000i64;

    uc.create_transaction(buy_dto(&account_id, &asset_id, micro))
        .await
        .unwrap();

    let rows = sqlx::query!(
        "SELECT asset_id FROM asset_prices WHERE asset_id = ?",
        asset_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert!(
        rows.is_empty(),
        "expected no AssetPrice rows when record_price is false"
    );
}

// MKT-058 — same-date collision is silently overwritten; exactly one row remains
#[tokio::test]
async fn same_date_collision_overwrites_silently() {
    let pool = make_pool().await;
    let uc = setup_uc(&pool).await;
    let account_id = create_account(&pool).await;
    let asset_id = create_asset(&pool).await;
    let micro = 1_000_000i64;

    let old_price = 50 * micro;
    sqlx::query!(
        "INSERT INTO asset_prices (asset_id, date, price) VALUES (?, ?, ?)",
        asset_id,
        "2024-01-01",
        old_price
    )
    .execute(&pool)
    .await
    .unwrap();

    let new_unit_price = 100 * micro;
    let dto = CreateTransactionDTO {
        record_price: true,
        unit_price: new_unit_price,
        ..buy_dto(&account_id, &asset_id, micro)
    };
    uc.create_transaction(dto).await.unwrap();

    let rows = sqlx::query!(
        "SELECT price FROM asset_prices WHERE asset_id = ? AND date = ?",
        asset_id,
        "2024-01-01"
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(
        rows.len(),
        1,
        "expected exactly one row after collision (no duplicate)"
    );
    assert_eq!(
        rows[0].price, new_unit_price,
        "old price should be silently overwritten"
    );
}

// MKT-061 — zero unit_price skips the AssetPrice write; transaction still succeeds
#[tokio::test]
async fn zero_unit_price_skips_auto_record() {
    let pool = make_pool().await;
    let uc = setup_uc(&pool).await;
    let account_id = create_account(&pool).await;
    let asset_id = create_asset(&pool).await;
    let micro = 1_000_000i64;

    let dto = CreateTransactionDTO {
        record_price: true,
        unit_price: 0,
        fees: micro,
        ..buy_dto(&account_id, &asset_id, micro)
    };

    let result = uc.create_transaction(dto).await;
    assert!(
        result.is_ok(),
        "transaction with unit_price=0 should succeed: {:?}",
        result
    );

    let rows = sqlx::query!(
        "SELECT asset_id FROM asset_prices WHERE asset_id = ?",
        asset_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert!(
        rows.is_empty(),
        "expected no AssetPrice row when unit_price is 0 (MKT-061 skip)"
    );
}

// MKT-059 — editing to a new date leaves the old price row intact and creates a new one
#[tokio::test]
async fn edit_to_new_date_preserves_old_price_and_creates_new() {
    let pool = make_pool().await;
    let uc = setup_uc(&pool).await;
    let account_id = create_account(&pool).await;
    let asset_id = create_asset(&pool).await;
    let micro = 1_000_000i64;

    let original_unit_price = 100 * micro;
    let tx = uc
        .create_transaction(CreateTransactionDTO {
            record_price: true,
            unit_price: original_unit_price,
            ..buy_dto(&account_id, &asset_id, micro)
        })
        .await
        .unwrap();

    let new_unit_price = 200 * micro;
    let update_dto = CreateTransactionDTO {
        record_price: true,
        date: "2024-06-01".to_string(),
        unit_price: new_unit_price,
        ..buy_dto(&account_id, &asset_id, micro)
    };

    uc.update_transaction(tx.id, update_dto).await.unwrap();

    let old_rows = sqlx::query!(
        "SELECT price FROM asset_prices WHERE asset_id = ? AND date = ?",
        asset_id,
        "2024-01-01"
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(
        old_rows.len(),
        1,
        "original AssetPrice row at 2024-01-01 should be untouched"
    );
    assert_eq!(
        old_rows[0].price, original_unit_price,
        "price at original date should be unchanged"
    );

    let new_rows = sqlx::query!(
        "SELECT price FROM asset_prices WHERE asset_id = ? AND date = ?",
        asset_id,
        "2024-06-01"
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(
        new_rows.len(),
        1,
        "new AssetPrice row at 2024-06-01 should have been created"
    );
    assert_eq!(
        new_rows[0].price, new_unit_price,
        "price at new date should equal the updated unit_price"
    );
}

// MKT-060 — deleting a transaction does NOT remove the AssetPrice row written by it
#[tokio::test]
async fn delete_does_not_cascade_to_asset_price() {
    let pool = make_pool().await;
    let uc = setup_uc(&pool).await;
    let account_id = create_account(&pool).await;
    let asset_id = create_asset(&pool).await;
    let micro = 1_000_000i64;

    let dto = CreateTransactionDTO {
        record_price: true,
        ..buy_dto(&account_id, &asset_id, micro)
    };
    let expected_date = dto.date.clone();
    let expected_price = dto.unit_price;

    let tx = uc.create_transaction(dto).await.unwrap();

    let before = sqlx::query!(
        "SELECT price FROM asset_prices WHERE asset_id = ? AND date = ?",
        asset_id,
        expected_date
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(before.len(), 1, "price row should exist before delete");

    uc.delete_transaction(&tx.id).await.unwrap();

    let after = sqlx::query!(
        "SELECT price FROM asset_prices WHERE asset_id = ? AND date = ?",
        asset_id,
        expected_date
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(
        after.len(),
        1,
        "AssetPrice row must survive transaction deletion (MKT-060)"
    );
    assert_eq!(
        after[0].price, expected_price,
        "price value must be unchanged after deletion"
    );
}
