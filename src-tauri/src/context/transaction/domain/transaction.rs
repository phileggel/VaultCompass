use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

/// Type of financial transaction.
#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Type,
    PartialEq,
    Eq,
    Default,
    strum_macros::Display,
    strum_macros::EnumString,
)]
pub enum TransactionType {
    /// A purchase (acquisition) of an asset.
    #[default]
    Purchase,
}

/// A single financial event affecting an asset's quantity and cost basis within an account.
/// All financial fields are stored as i64 micro-units (ADR-001, TRX-024).
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
pub struct Transaction {
    /// Unique identifier.
    pub id: String,
    /// The account where the transaction occurred.
    pub account_id: String,
    /// The financial asset involved.
    pub asset_id: String,
    /// Type of transaction (Purchase; Sell deferred per TRX-040).
    pub transaction_type: TransactionType,
    /// Date when the transaction was executed (ISO 8601, "YYYY-MM-DD").
    pub date: String,
    /// Number of units acquired (micro-units: value × 10^6). Must be > 0.
    pub quantity: i64,
    /// Price per unit in asset's native currency (micro-units). Can be 0 (gifted assets).
    pub unit_price: i64,
    /// Conversion rate from asset currency to account currency (micro-units).
    pub exchange_rate: i64,
    /// Transaction fees in account currency (micro-units).
    pub fees: i64,
    /// Total cost in account currency including fees (micro-units). Must be > 0.
    pub total_amount: i64,
    /// Optional user comment.
    pub note: Option<String>,
}

impl Transaction {
    /// Creates a new Transaction with a generated ID.
    /// Validates TRX-020 and TRX-026.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        account_id: String,
        asset_id: String,
        transaction_type: TransactionType,
        date: String,
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        fees: i64,
        total_amount: i64,
        note: Option<String>,
    ) -> Result<Self> {
        Self::validate(
            &date,
            quantity,
            unit_price,
            exchange_rate,
            fees,
            total_amount,
        )?;
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            account_id,
            asset_id,
            transaction_type,
            date,
            quantity,
            unit_price,
            exchange_rate,
            fees,
            total_amount,
            note,
        })
    }

    /// Creates a Transaction with a provided ID (used for updates, TRX-033).
    /// Applies the same validation as new().
    #[allow(clippy::too_many_arguments)]
    pub fn with_id(
        id: String,
        account_id: String,
        asset_id: String,
        transaction_type: TransactionType,
        date: String,
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        fees: i64,
        total_amount: i64,
        note: Option<String>,
    ) -> Result<Self> {
        Self::validate(
            &date,
            quantity,
            unit_price,
            exchange_rate,
            fees,
            total_amount,
        )?;
        Ok(Self {
            id,
            account_id,
            asset_id,
            transaction_type,
            date,
            quantity,
            unit_price,
            exchange_rate,
            fees,
            total_amount,
            note,
        })
    }

    /// Reconstructs a Transaction from storage without validation.
    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: String,
        account_id: String,
        asset_id: String,
        transaction_type: TransactionType,
        date: String,
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        fees: i64,
        total_amount: i64,
        note: Option<String>,
    ) -> Self {
        Self {
            id,
            account_id,
            asset_id,
            transaction_type,
            date,
            quantity,
            unit_price,
            exchange_rate,
            fees,
            total_amount,
            note,
        }
    }

    /// Validates business rules (TRX-020).
    /// total_amount is computed by the orchestrator (TRX-026) before this is called —
    /// no formula check here.
    fn validate(
        date: &str,
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        _fees: i64,
        total_amount: i64,
    ) -> Result<()> {
        // TRX-020 — date must be parseable, not in the future, not before 1900-01-01
        let parsed_date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| anyhow::anyhow!("Invalid date format — expected YYYY-MM-DD"))?;
        let today = chrono::Local::now().date_naive();
        if parsed_date > today {
            anyhow::bail!("Transaction date cannot be in the future");
        }
        let min_date = NaiveDate::from_ymd_opt(1900, 1, 1).expect("1900-01-01 is a valid date");
        if parsed_date < min_date {
            anyhow::bail!("Transaction date cannot be before 1900-01-01");
        }

        // TRX-020 — quantity must be strictly positive
        if quantity <= 0 {
            anyhow::bail!("Quantity must be strictly positive");
        }

        // TRX-020 — unit_price must be >= 0
        if unit_price < 0 {
            anyhow::bail!("Unit price cannot be negative");
        }

        // TRX-020 — exchange_rate must be strictly positive
        if exchange_rate <= 0 {
            anyhow::bail!("Exchange rate must be strictly positive");
        }

        // TRX-020 — total_amount must be > 0
        if total_amount <= 0 {
            anyhow::bail!("Total amount must be strictly positive");
        }

        Ok(())
    }
}

/// Interface for transaction persistence.
#[async_trait]
pub trait TransactionRepository: Send + Sync {
    /// Fetches a transaction by ID.
    async fn get_by_id(&self, id: &str) -> Result<Option<Transaction>>;
    /// Fetches all transactions for a given account and asset, ordered chronologically (TRX-036).
    async fn get_by_account_asset(
        &self,
        account_id: &str,
        asset_id: &str,
    ) -> Result<Vec<Transaction>>;
    /// Persists a new transaction.
    async fn create(&self, tx: Transaction) -> Result<Transaction>;
    /// Updates an existing transaction.
    async fn update(&self, tx: Transaction) -> Result<Transaction>;
    /// Deletes a transaction by ID.
    async fn delete(&self, id: &str) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_transaction(
        quantity: i64,
        unit_price: i64,
        exchange_rate: i64,
        fees: i64,
        total_amount: i64,
    ) -> Result<Transaction> {
        Transaction::new(
            "account-1".to_string(),
            "asset-1".to_string(),
            TransactionType::Purchase,
            "2020-01-01".to_string(),
            quantity,
            unit_price,
            exchange_rate,
            fees,
            total_amount,
            None,
        )
    }

    // TRX-020 — quantity must be > 0
    #[test]
    fn rejects_zero_quantity() {
        let micro = 1_000_000i64;
        let result = make_transaction(0, micro, micro, 0, 0);
        assert!(result.is_err());
    }

    // TRX-020 — unit_price can be 0 (gifted assets, OQ-1)
    #[test]
    fn accepts_zero_unit_price() {
        // qty=1_000_000 (1.0), price=0, rate=1_000_000, fees=1_000_000 (1.0), total=1_000_000
        // expected = (1_000_000 * 0 / 1_000_000) * 1_000_000 / 1_000_000 + 1_000_000 = 1_000_000 ✓
        let micro = 1_000_000i64;
        let result = make_transaction(micro, 0, micro, micro, micro);
        assert!(result.is_ok(), "got: {:?}", result.err());
    }

    // TRX-020 — date before 1900-01-01 is rejected
    #[test]
    fn rejects_date_before_1900() {
        let micro = 1_000_000i64;
        let result = Transaction::new(
            "a".to_string(),
            "b".to_string(),
            TransactionType::Purchase,
            "1899-12-31".to_string(),
            micro,
            micro,
            micro,
            0,
            micro,
            None,
        );
        assert!(result.is_err());
    }

    // TRX-020 — future date is rejected
    #[test]
    fn rejects_future_date() {
        let micro = 1_000_000i64;
        let result = Transaction::new(
            "a".to_string(),
            "b".to_string(),
            TransactionType::Purchase,
            "2099-01-01".to_string(),
            micro,
            micro,
            micro,
            0,
            micro,
            None,
        );
        assert!(result.is_err());
    }

    // TRX-020 — exchange_rate must be strictly positive
    #[test]
    fn rejects_zero_exchange_rate() {
        let micro = 1_000_000i64;
        // total_amount=0 also fails (TRX-020) but exchange_rate=0 is caught first
        let result = make_transaction(micro, micro, 0, 0, 0);
        assert!(result.is_err());
    }
}
