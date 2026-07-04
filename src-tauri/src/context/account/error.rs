/// Single flat error enum for the `account` bounded context (gold error model).
///
/// Every failure the BC can raise — aggregate-invariant rejections from domain
/// methods, transaction-factory validation, holding validation, and service-layer
/// lookup / uniqueness / infrastructure translation — lives in this one type.
/// `#[serde(tag = "code")]` makes each variant serialize as
/// `{ "code": "VariantName", ...payload }` on the wire.
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type, Clone)]
#[serde(tag = "code")]
pub enum AccountError {
    // --- Account aggregate construction (own-input validation) ---
    /// Account name is empty or whitespace-only.
    #[error("Account name cannot be empty")]
    NameEmpty,
    /// The currency string is not a valid ISO 4217 code.
    #[error("Invalid currency code: {currency}")]
    InvalidCurrency {
        /// The offending currency string the caller passed.
        currency: String,
    },

    // --- Holding validation ---
    /// Holding quantity is negative.
    #[error("Holding quantity cannot be negative")]
    NegativeQuantity,
    /// Holding average_price is negative.
    #[error("Holding average_price cannot be negative")]
    NegativeAveragePrice,

    // --- Opening balance ---
    /// total_cost was negative (TRX-045). A zero total cost is valid (e.g. a
    /// mined / gifted / airdropped position).
    #[error("Total cost must not be negative")]
    InvalidTotalCost,

    // --- Account aggregate operations (buy/sell/correct/cancel/cash) ---
    /// Attempt to sell an asset with no open position (quantity = 0).
    #[error("No units available to sell (closed position)")]
    ClosedPosition,
    /// Sell quantity exceeds the currently held units.
    #[error("Oversell: requested {requested} exceeds available {available}")]
    Oversell {
        /// Units currently held before the sale.
        available: i64,
        /// Units the operation attempts to sell.
        requested: i64,
    },
    /// Correcting a transaction would leave a later sell with insufficient units.
    #[error("Editing this transaction would create a cascading oversell")]
    CascadingOversell,
    /// No transaction with the given ID exists within this account.
    #[error("Transaction not found")]
    TransactionNotFound,
    /// Attempted cash debit (or chronological replay step) would drive the cash
    /// holding strictly negative (CSH-080).
    #[error("Insufficient cash: current balance {current_balance_micros} {currency}")]
    InsufficientCash {
        /// Cash holding's running balance at the point of rejection (micro-units, account currency).
        current_balance_micros: i64,
        /// ISO 4217 currency code of the offending account's cash holding.
        currency: String,
    },

    // --- Transaction-factory validation (TRX-020) ---
    /// Date string could not be parsed as YYYY-MM-DD.
    #[error("Invalid date format — expected YYYY-MM-DD")]
    InvalidDate,
    /// Transaction date is in the future.
    #[error("Transaction date cannot be in the future")]
    DateInFuture,
    /// Transaction date is before 1900-01-01.
    #[error("Transaction date cannot be before 1900-01-01")]
    DateTooOld,
    /// Quantity is zero or negative.
    #[error("Quantity must be strictly positive")]
    QuantityNotPositive,
    /// Cash deposit/withdrawal amount was zero or negative (CSH-021/CSH-031).
    /// Cash-specific framing of the same TRX-020 constraint that surfaces as
    /// `QuantityNotPositive` for non-cash transactions; raised by the cash
    /// factories before the generic check so the FE sees the cash-specific code.
    #[error("Amount must be greater than 0")]
    AmountNotPositive,
    /// Unit price is negative.
    #[error("Unit price cannot be negative")]
    UnitPriceNegative,
    /// Fees amount is negative.
    #[error("Fees cannot be negative")]
    FeesNegative,
    /// Exchange rate is zero or negative.
    #[error("Exchange rate must be strictly positive")]
    ExchangeRateNotPositive,
    /// Total amount is zero or negative.
    #[error("Total amount must be strictly positive")]
    TotalAmountNotPositive,

    // --- ManagementFee factory validation (FEE-021) ---
    /// The management fee percentage is zero or negative (FEE-021).
    #[error("Percentage must be strictly positive")]
    PercentageNotPositive,
    /// The management fee percentage exceeds 100% in micro-percent (FEE-021).
    #[error("Percentage cannot exceed 100%")]
    PercentageAboveHundred,

    // --- FeeSchedule validation (FEE-032) ---
    /// The annual rate is zero or negative (FEE-032).
    #[error("Annual rate must be strictly positive")]
    RateNotPositive,
    /// The annual rate exceeds 100% in micro-percent (FEE-032).
    #[error("Annual rate cannot exceed 100%")]
    RateAboveHundred,
    /// The schedule end_date is not strictly after start_date (FEE-032).
    #[error("End date must be after start date")]
    EndBeforeStart,
    /// A fee schedule for this (account, asset) pair already exists (FEE-031).
    #[error("A fee schedule for this account and asset already exists")]
    ScheduleAlreadyExists,
    /// No fee schedule found for the given (account, asset) pair (FEE-060).
    #[error("Fee schedule not found")]
    ScheduleNotFound,
    /// The % management-fee mechanism is disabled on this account (FEE-077).
    #[error("Management fees are disabled on this account")]
    ManagementFeesDisabled,

    // --- Service-layer lookup / uniqueness / infrastructure ---
    /// No account exists with the requested ID.
    #[error("Account not found: {account_id}")]
    AccountNotFound {
        /// The ID the caller asked for.
        account_id: String,
    },
    /// Account name (case-insensitive) collides with an existing one.
    #[error("Account name already exists")]
    NameAlreadyExists,
    /// Application-layer translation of any infrastructure failure from an
    /// account-side repository call. No `hint` payload on the wire; the full
    /// diagnostic chain is preserved server-side via `tracing::error!` at the
    /// translation site. FE shows the i18n key `error.DatabaseError`.
    #[error("An unexpected database error occurred")]
    DatabaseError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, to_value};

    /// Verifies the `#[serde(tag = "code")]` contract: every variant emits a
    /// flat `{ "code": "VariantName", ...payload }` object on the wire. A missing
    /// or mis-tagged variant collapses to `null` under an untagged composite —
    /// this test catches that regression for `AccountError` itself.
    #[test]
    fn each_variant_emits_a_code() {
        assert_eq!(
            to_value(AccountError::NameEmpty).unwrap(),
            json!({ "code": "NameEmpty" })
        );
        assert_eq!(
            to_value(AccountError::InvalidCurrency {
                currency: "XX".into()
            })
            .unwrap(),
            json!({ "code": "InvalidCurrency", "currency": "XX" })
        );
        assert_eq!(
            to_value(AccountError::NegativeQuantity).unwrap(),
            json!({ "code": "NegativeQuantity" })
        );
        assert_eq!(
            to_value(AccountError::NegativeAveragePrice).unwrap(),
            json!({ "code": "NegativeAveragePrice" })
        );
        assert_eq!(
            to_value(AccountError::InvalidTotalCost).unwrap(),
            json!({ "code": "InvalidTotalCost" })
        );
        assert_eq!(
            to_value(AccountError::ClosedPosition).unwrap(),
            json!({ "code": "ClosedPosition" })
        );
        assert_eq!(
            to_value(AccountError::Oversell {
                available: 1,
                requested: 2
            })
            .unwrap(),
            json!({ "code": "Oversell", "available": 1, "requested": 2 })
        );
        assert_eq!(
            to_value(AccountError::CascadingOversell).unwrap(),
            json!({ "code": "CascadingOversell" })
        );
        assert_eq!(
            to_value(AccountError::TransactionNotFound).unwrap(),
            json!({ "code": "TransactionNotFound" })
        );
        assert_eq!(
            to_value(AccountError::InsufficientCash {
                current_balance_micros: 500,
                currency: "EUR".into()
            })
            .unwrap(),
            json!({ "code": "InsufficientCash", "current_balance_micros": 500, "currency": "EUR" })
        );
        assert_eq!(
            to_value(AccountError::InvalidDate).unwrap(),
            json!({ "code": "InvalidDate" })
        );
        assert_eq!(
            to_value(AccountError::DateInFuture).unwrap(),
            json!({ "code": "DateInFuture" })
        );
        assert_eq!(
            to_value(AccountError::DateTooOld).unwrap(),
            json!({ "code": "DateTooOld" })
        );
        assert_eq!(
            to_value(AccountError::QuantityNotPositive).unwrap(),
            json!({ "code": "QuantityNotPositive" })
        );
        assert_eq!(
            to_value(AccountError::AmountNotPositive).unwrap(),
            json!({ "code": "AmountNotPositive" })
        );
        assert_eq!(
            to_value(AccountError::UnitPriceNegative).unwrap(),
            json!({ "code": "UnitPriceNegative" })
        );
        assert_eq!(
            to_value(AccountError::FeesNegative).unwrap(),
            json!({ "code": "FeesNegative" })
        );
        assert_eq!(
            to_value(AccountError::ExchangeRateNotPositive).unwrap(),
            json!({ "code": "ExchangeRateNotPositive" })
        );
        assert_eq!(
            to_value(AccountError::TotalAmountNotPositive).unwrap(),
            json!({ "code": "TotalAmountNotPositive" })
        );
        assert_eq!(
            to_value(AccountError::AccountNotFound {
                account_id: "acc-1".into()
            })
            .unwrap(),
            json!({ "code": "AccountNotFound", "account_id": "acc-1" })
        );
        assert_eq!(
            to_value(AccountError::NameAlreadyExists).unwrap(),
            json!({ "code": "NameAlreadyExists" })
        );
        assert_eq!(
            to_value(AccountError::DatabaseError).unwrap(),
            json!({ "code": "DatabaseError" })
        );
        // FEE variants
        assert_eq!(
            to_value(AccountError::PercentageNotPositive).unwrap(),
            json!({ "code": "PercentageNotPositive" })
        );
        assert_eq!(
            to_value(AccountError::PercentageAboveHundred).unwrap(),
            json!({ "code": "PercentageAboveHundred" })
        );
        assert_eq!(
            to_value(AccountError::RateNotPositive).unwrap(),
            json!({ "code": "RateNotPositive" })
        );
        assert_eq!(
            to_value(AccountError::RateAboveHundred).unwrap(),
            json!({ "code": "RateAboveHundred" })
        );
        assert_eq!(
            to_value(AccountError::EndBeforeStart).unwrap(),
            json!({ "code": "EndBeforeStart" })
        );
        assert_eq!(
            to_value(AccountError::ScheduleAlreadyExists).unwrap(),
            json!({ "code": "ScheduleAlreadyExists" })
        );
        assert_eq!(
            to_value(AccountError::ScheduleNotFound).unwrap(),
            json!({ "code": "ScheduleNotFound" })
        );
    }
}
