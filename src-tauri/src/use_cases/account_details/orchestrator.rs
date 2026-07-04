use crate::context::account::{
    Account, AccountError, AccountService, Transaction, TransactionType,
};
use crate::context::asset::{AssetPriceSource, AssetService};
use crate::context::currency::CurrencyService;
use crate::core::cash::{is_cash_asset, system_cash_asset_id};
use crate::core::logger::BACKEND;
use chrono::{Local, NaiveDate};
use serde::Serialize;
use specta::Type;
use std::collections::{HashMap, HashSet};
use std::result::Result as StdResult;
use std::sync::Arc;

/// Enriched view of a single active holding (quantity > 0) with asset metadata (ACD-020).
#[derive(Debug, Serialize, Clone, Type)]
pub struct HoldingDetail {
    /// ID of the held asset.
    pub asset_id: String,
    /// Display name of the asset.
    pub asset_name: String,
    /// Ticker or user-defined reference.
    pub asset_reference: String,
    /// Current units held (i64 micro-units, ADR-001).
    pub quantity: i64,
    /// VWAP purchase price in account currency (i64 micro-units, ADR-001).
    pub average_price: i64,
    /// Total cost of position: quantity × average_price / MICRO (i64 micro-units, ACD-023).
    pub cost_basis: i64,
    /// Sum of realized P&L from all Sell transactions for this asset (i64 micro-units, SEL-042).
    pub realized_pnl: i64,
    /// ISO 4217 currency code of the asset's native currency (MKT-023).
    pub asset_currency: String,
    /// Most recently dated price for this asset in asset currency (i64 micros). None if no price recorded (MKT-031).
    pub current_price: Option<i64>,
    /// ISO date string of the price observation. None when current_price is None (MKT-031).
    pub current_price_date: Option<String>,
    /// Provenance of `current_price`. None when current_price is None (MKT-142).
    pub current_price_source: Option<AssetPriceSource>,
    /// Unrealized gain/loss in account currency (i64 micros). None when no price exists, or
    /// when a foreign holding has no usable rate (MKT-033/034, FXR-031/034).
    /// 0 (not None) when current price equals average price (MKT-033).
    pub unrealized_pnl: Option<i64>,
    /// Performance percentage as i64 micros (5.25% = 5_250_000). None when unrealized_pnl is None or cost_basis = 0 (MKT-035).
    /// 0 (not None) when unrealized_pnl is 0 (MKT-035).
    pub performance_pct: Option<i64>,
    /// Sum of dividend cash credited for this (account, asset), in account currency (i64 micros).
    /// 0 when no dividends recorded; always computable (DIV-070).
    pub dividends_received: i64,
    /// Dividend-inclusive total return: (unrealized_pnl + dividends_received) × 100 / cost_basis.
    /// None under the same conditions as performance_pct (DIV-071).
    pub total_return_pct: Option<i64>,
    /// ISO date of the FX rate used to value a foreign holding in the account
    /// currency (FXR-090). `None` for a same-currency holding (no conversion),
    /// a foreign holding with no usable rate, or cash — i.e. present only when a
    /// converted value backed by a real rate is shown.
    pub fx_rate_date: Option<String>,
    /// Cumulative value removed via management fee deductions for this (account, asset),
    /// in account-currency micros (FEE-051/052).
    /// Computed on read as Σ(qty_removed × price_as_of(date)), FXR-converted.
    /// 0 when no management fee transactions have been recorded.
    pub management_fees: i64,
    /// Market value of the holding in account-currency micros (ACD-052) — exactly
    /// its contribution to `total_global_value` (CSH-094): the balance for the Cash
    /// Holding, price × quantity × FX for a priced non-cash holding. None when no
    /// price is recorded or a foreign holding has no usable rate (FXR-034).
    pub market_value: Option<i64>,
}

/// Enriched view of a fully-closed position (quantity == 0, ACD-044).
#[derive(Debug, Serialize, Clone, Type)]
pub struct ClosedHoldingDetail {
    /// ID of the previously held asset.
    pub asset_id: String,
    /// Display name of the asset.
    pub asset_name: String,
    /// Ticker or user-defined reference.
    pub asset_reference: String,
    /// Total realized P&L for this position (micro-units, ACD-045).
    pub realized_pnl: i64,
    /// Total dividends received over the life of this position (micro-units, DIV-073).
    pub dividends_received: i64,
    /// ISO date of the most recent sell for this position ("YYYY-MM-DD", ACD-043).
    pub last_sold_date: String,
}

/// Top-level response for the get_account_details command (ACD spec).
#[derive(Debug, Serialize, Clone, Type)]
pub struct AccountDetailsResponse {
    /// Display name of the account (ACD-032).
    pub account_name: String,
    /// Active holdings (quantity > 0), sorted by asset_name asc (ACD-020, ACD-033).
    pub holdings: Vec<HoldingDetail>,
    /// Closed positions (quantity == 0), sorted by asset_name asc (ACD-044, ACD-046).
    pub closed_holdings: Vec<ClosedHoldingDetail>,
    /// Total holding count regardless of quantity (ACD-034).
    pub total_holding_count: i64,
    /// Sum of cost_basis across all active holdings, 0 if none (ACD-031).
    pub total_cost_basis: i64,
    /// Sum of total_realized_pnl across ALL holdings (active + closed), 0 if none (ACD-047).
    pub total_realized_pnl: i64,
    /// Sum of unrealized_pnl across priced active holdings (foreign holdings converted to
    /// account currency, FXR-040). None when none qualify (MKT-040).
    pub total_unrealized_pnl: Option<i64>,
    /// Total economic value of the account in account-currency micros (CSH-094):
    /// `cash_holding.quantity + Σ_h (h.quantity × latest_price(h))` over non-cash active holdings.
    /// Unpriced non-cash holdings contribute 0 (no fallback to `average_price`).
    /// Returns 0 when no Cash Holding and no priced non-cash holdings.
    pub total_global_value: i64,
    /// Sum of dividend cash credited across all of the account's dividend transactions, in account
    /// currency (i64 micros). 0 when none (DIV-073).
    pub total_dividends_received: i64,
    /// Sum of `management_fees` across all active holdings, in account-currency micros (FEE-072/073).
    /// 0 when no management fee transactions have been recorded.
    pub total_management_fees: i64,
    /// Net external cash input since inception: Σ Deposit − Σ Withdrawal amounts, in
    /// account-currency micros (ACD-053). Negative when withdrawals exceed deposits.
    /// The as-of view counts only transactions dated on or before the as-of date.
    pub total_net_cash_input: i64,
}

/// Orchestrates a cross-context read of account + asset data (ADR-003, ADR-004).
pub struct AccountDetailsUseCase {
    account_service: Arc<AccountService>,
    asset_service: Arc<AssetService>,
    currency_service: Arc<CurrencyService>,
}

impl AccountDetailsUseCase {
    /// Creates a new use case instance. The currency service is the valuation
    /// read port for foreign-currency holdings (FXR-030/035).
    pub fn new(
        account_service: Arc<AccountService>,
        asset_service: Arc<AssetService>,
        currency_service: Arc<CurrencyService>,
    ) -> Self {
        Self {
            account_service,
            asset_service,
            currency_service,
        }
    }

    /// Builds an AccountDetailsResponse for the given account (ACD-012 to ACD-050).
    ///
    /// `as_of_date` selects the valuation date: `None` is the live view (holdings
    /// as they stand today), `Some("YYYY-MM-DD")` reconstructs the account as it
    /// stood on a past date (read-only). The as-of date must be a valid ISO date
    /// not in the future (`InvalidDate` / `DateInFuture`).
    pub async fn get_account_details(
        &self,
        account_id: &str,
        as_of_date: Option<&str>,
    ) -> StdResult<AccountDetailsResponse, AccountError> {
        match as_of_date {
            None => self.get_account_details_live(account_id).await,
            Some(date) => self.get_account_details_as_of(account_id, date).await,
        }
    }

    /// Live view: holdings as they stand today, read from the stored Holding
    /// aggregates (ACD-012 to ACD-050).
    async fn get_account_details_live(
        &self,
        account_id: &str,
    ) -> StdResult<AccountDetailsResponse, AccountError> {
        // ACD-032 — fetch account; bail with not-found if missing (ACD-012)
        let account = self
            .account_service
            .get_by_id(account_id)
            .await?
            .ok_or_else(|| AccountError::AccountNotFound {
                account_id: account_id.to_string(),
            })?;

        // ACD-034 — total count before quantity filter
        let all_holdings = self
            .account_service
            .get_holdings_for_account(account_id)
            .await?;
        let total_holding_count = all_holdings.len() as i64;

        // ACD-047 — total realized pnl from ALL holdings (active + closed)
        let total_realized_pnl: i64 = all_holdings.iter().map(|h| h.total_realized_pnl).sum();

        // ACD-020 — active holdings (quantity > 0); ACD-044 — closed (quantity == 0, last_sold_date set).
        // CSH-090 — the Cash Holding is always active, even at quantity 0 (never closed), so the
        // cash row is always shown. The predicate runs on raw holdings before asset-class
        // enrichment, so it tests the deterministic cash-asset id prefix, not the class.
        let (active_holdings, closed_holdings_raw): (Vec<_>, Vec<_>) = all_holdings
            .into_iter()
            .partition(|h| h.quantity > 0 || crate::core::cash::is_cash_asset(&h.asset_id));

        // ACD-022 — enrich each active holding with asset metadata; ACD-021 — archived assets included
        // CSH-094 — accumulate the Global Value as we go: cash quantity + Σ priced non-cash holdings.
        // CSH-093 — accumulate the total cost basis from non-cash holdings only (cash has no
        // cost basis by spec; its `cost_basis` field is set to 0 below so the row stays blank).
        // DIV-070/073 — sum Dividend cash per (account, asset) and across the whole account,
        // from a single transaction fetch. Dividends are stored in account currency.
        let all_txs = self
            .account_service
            .get_all_transactions_for_account(account_id)
            .await?;
        let mut dividends_by_asset: HashMap<String, i64> = HashMap::new();
        let mut total_dividends_received: i64 = 0;
        // ACD-053 — net external cash input since inception: deposits − withdrawals.
        let mut total_net_cash_input: i64 = 0;
        for t in &all_txs {
            match t.transaction_type {
                TransactionType::Dividend => {
                    let entry = dividends_by_asset.entry(t.asset_id.clone()).or_insert(0);
                    *entry = entry.saturating_add(t.total_amount);
                    total_dividends_received =
                        total_dividends_received.saturating_add(t.total_amount);
                }
                TransactionType::Deposit => {
                    total_net_cash_input = total_net_cash_input.saturating_add(t.total_amount);
                }
                TransactionType::Withdrawal => {
                    total_net_cash_input = total_net_cash_input.saturating_sub(t.total_amount);
                }
                _ => {}
            }
        }

        // FEE-052/053 — cumulative management fees per asset + account total (all fees to date).
        let (management_fees_by_asset, total_management_fees) = self
            .compute_management_fees(&all_txs, &account.currency, None)
            .await?;

        // FXR-035 — valuation date for resolving FX rates is "today"; the
        // write-guard (FXR-022) forbids future-dated rates, so the latest rate
        // on or before today is simply the latest recorded rate.
        let today = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();

        let mut details: Vec<HoldingDetail> = Vec::with_capacity(active_holdings.len());
        let mut total_global_value: i64 = 0;
        let mut total_cost_basis: i64 = 0;
        for holding in active_holdings {
            let asset = self
                .asset_service
                .get_asset_by_id(&holding.asset_id)
                .await
                .map_err(|e| {
                    tracing::error!(target: BACKEND, asset_id = %holding.asset_id, err = ?e, "get_account_details: get_asset_by_id failed");
                    AccountError::DatabaseError
                })?
                .ok_or_else(|| {
                    tracing::error!(target: BACKEND, asset_id = %holding.asset_id, "get_account_details: holding references missing asset");
                    AccountError::DatabaseError
                })?;

            let is_cash = asset.class == crate::context::asset::AssetClass::Cash;

            // CSH-094 / FXR-041 — Cash Holding contributes its raw quantity (already in
            // account currency). Non-cash holdings contribute their market value converted
            // to account currency (FXR-030); unpriced holdings, or foreign holdings with no
            // usable rate (FXR-034), contribute 0. The conversion happens further down once
            // the rate and price are known.
            if is_cash {
                total_global_value = total_global_value.saturating_add(holding.quantity);
            }

            // CSH-093 — cash holdings carry no cost basis. Non-cash uses the standard
            // (quantity × average_price) formula with i128 intermediates (ACD-023/024).
            let cost_basis = if is_cash {
                0
            } else {
                let computed =
                    (holding.quantity as i128 * holding.average_price as i128 / 1_000_000) as i64;
                total_cost_basis = total_cost_basis.saturating_add(computed);
                computed
            };

            // FXR-030/035 — resolve the rate to value this non-cash holding in the
            // account currency. An identity pair resolves to 1.0 without a lookup
            // (same-currency unchanged, MKT-033); a foreign pair with no usable rate
            // yields None and the holding falls back to the FXR-034 mismatch path.
            let resolved_rate = if is_cash {
                None
            } else {
                self.currency_service
                    .resolve_rate(&asset.currency, &account.currency, &today)
                    .await
                    .map_err(|e| {
                        tracing::error!(target: BACKEND, asset_id = %holding.asset_id, err = ?e, "get_account_details: resolve_rate failed");
                        AccountError::DatabaseError
                    })?
            };
            let conversion_rate: Option<i64> =
                resolved_rate.as_ref().map(|resolved| resolved.rate_micros);
            // FXR-090 — date of the FX rate used; None for identity (same-currency)
            // and no-rate holdings, so the staleness label shows only on a
            // converted foreign value.
            let fx_rate_date: Option<String> =
                resolved_rate.and_then(|resolved| resolved.rate_date);

            // MKT-031 — fetch latest price, degrade gracefully on failure
            let latest_price = self
                .asset_service
                .get_latest_price(&holding.asset_id)
                .await
                .ok()
                .flatten();

            let (
                current_price,
                current_price_date,
                current_price_source,
                unrealized_pnl,
                performance_pct,
            ) = if let Some(ref latest) = latest_price {
                let current_price = latest.price;
                let current_price_date = latest.date.clone();
                let current_price_source = latest.source.clone();
                // FXR-030/031/032 — value the holding in account currency using the
                // resolved rate. Same-currency holdings resolve to rate 1.0 so the
                // arithmetic is unchanged (MKT-033/034). When no usable rate exists
                // for a foreign pair, P&L stays None (FXR-034).
                let (unrealized_pnl, performance_pct) = match conversion_rate {
                    Some(rate) => {
                        // FXR-030 — current_price × rate, i128 intermediates (ACD-024)
                        let converted_price =
                            (current_price as i128 * rate as i128 / 1_000_000) as i64;
                        let unrealized_pnl = ((converted_price as i128
                            - holding.average_price as i128)
                            * holding.quantity as i128
                            / 1_000_000) as i64;
                        let performance_pct = if cost_basis != 0 {
                            Some((unrealized_pnl as i128 * 100_000_000 / cost_basis as i128) as i64)
                        } else {
                            None
                        };
                        (Some(unrealized_pnl), performance_pct)
                    }
                    None => (None, None),
                };
                (
                    Some(current_price),
                    Some(current_price_date),
                    Some(current_price_source),
                    unrealized_pnl,
                    performance_pct,
                )
            } else {
                (None, None, None, None, None)
            };

            // CSH-094 / FXR-041 / ACD-052 — the holding's market value in account
            // currency: the balance for the Cash Holding, price × quantity × FX for a
            // priced non-cash holding, None when no price or no usable rate (FXR-034).
            // A priced non-cash holding contributes it to the Global Value (the cash
            // quantity was already added above).
            let market_value: Option<i64> = if is_cash {
                Some(holding.quantity)
            } else if let (Some(cp), Some(rate)) = (current_price, conversion_rate) {
                let converted_price = (cp as i128 * rate as i128 / 1_000_000) as i64;
                Some((holding.quantity as i128 * converted_price as i128 / 1_000_000) as i64)
            } else {
                None
            };
            if !is_cash {
                if let Some(value) = market_value {
                    total_global_value = total_global_value.saturating_add(value);
                }
            }

            // DIV-070 — dividends_received: sum of Dividend total_amount for this (account, asset).
            let dividends_received: i64 = *dividends_by_asset.get(&holding.asset_id).unwrap_or(&0);
            // DIV-071 — total_return_pct: (unrealized_pnl + dividends_received) × 100 / cost_basis;
            // None under the same conditions as performance_pct (no price / no usable rate / zero cost basis).
            let total_return_pct: Option<i64> = match unrealized_pnl {
                Some(upnl) if cost_basis != 0 => Some(
                    ((upnl as i128 + dividends_received as i128) * 100_000_000 / cost_basis as i128)
                        as i64,
                ),
                _ => None,
            };
            let management_fees = *management_fees_by_asset
                .get(&holding.asset_id)
                .unwrap_or(&0);
            details.push(HoldingDetail {
                asset_id: holding.asset_id,
                asset_name: asset.name,
                asset_reference: asset.reference,
                quantity: holding.quantity,
                average_price: holding.average_price,
                cost_basis,
                realized_pnl: holding.total_realized_pnl,
                asset_currency: asset.currency,
                current_price,
                current_price_date,
                current_price_source,
                unrealized_pnl,
                performance_pct,
                dividends_received,
                total_return_pct,
                fx_rate_date,
                management_fees, // FEE-052
                market_value,    // ACD-052
            });
        }

        // ACD-033 — sort alphabetically by asset_name ascending
        details.sort_by(|a, b| a.asset_name.cmp(&b.asset_name));

        // ACD-031 / CSH-093 — total_cost_basis already accumulated above (non-cash only).

        // MKT-040 — sum unrealized_pnl across qualifying holdings; None when none qualify
        let qualifying_pnls: Vec<i64> = details.iter().filter_map(|d| d.unrealized_pnl).collect();
        let total_unrealized_pnl = if qualifying_pnls.is_empty() {
            None
        } else {
            Some(qualifying_pnls.iter().sum())
        };

        // ACD-044/ACD-045 — enrich closed positions with asset metadata
        // Only holdings with last_sold_date set are shown (they're genuinely closed)
        let mut closed_details: Vec<ClosedHoldingDetail> =
            Vec::with_capacity(closed_holdings_raw.len());
        for holding in closed_holdings_raw {
            let Some(last_sold_date) = holding.last_sold_date else {
                continue; // ACD-045: skip qty=0 holdings without a sell date
            };
            let asset = self
                .asset_service
                .get_asset_by_id(&holding.asset_id)
                .await
                .map_err(|e| {
                    tracing::error!(target: BACKEND, asset_id = %holding.asset_id, err = ?e, "get_account_details: get_asset_by_id failed (closed)");
                    AccountError::DatabaseError
                })?
                .ok_or_else(|| {
                    tracing::error!(target: BACKEND, asset_id = %holding.asset_id, "get_account_details: closed holding references missing asset");
                    AccountError::DatabaseError
                })?;
            // DIV-073 — carry forward dividends received while the position was open.
            let dividends_received = *dividends_by_asset.get(&holding.asset_id).unwrap_or(&0);
            closed_details.push(ClosedHoldingDetail {
                asset_id: holding.asset_id,
                asset_name: asset.name,
                asset_reference: asset.reference,
                realized_pnl: holding.total_realized_pnl,
                dividends_received,
                last_sold_date,
            });
        }

        // ACD-046 — sort closed holdings by asset_name ascending
        closed_details.sort_by(|a, b| a.asset_name.cmp(&b.asset_name));

        // DIV-073 — total_dividends_received computed above from the single transaction fetch.

        Ok(AccountDetailsResponse {
            account_name: account.name,
            holdings: details,
            closed_holdings: closed_details,
            total_holding_count,
            total_cost_basis,
            total_realized_pnl,
            total_unrealized_pnl,
            total_global_value,
            total_dividends_received,
            total_management_fees, // FEE-053/072
            total_net_cash_input,  // ACD-053
        })
    }

    /// FEE-051/052/053 — cumulative management-fee value per `(account, asset)` and the
    /// account total, in account currency. Each `ManagementFee` transaction is valued at the
    /// carry-forward recorded price as of its own date (PRF-022); it contributes 0 when no
    /// price is recorded on or before that date (FEE-054) or no usable FX rate exists
    /// (FEE-073). When `as_of` is `Some`, only fees dated on or before it are counted (FEE-072).
    async fn compute_management_fees(
        &self,
        all_txs: &[Transaction],
        account_currency: &str,
        as_of: Option<&str>,
    ) -> StdResult<(HashMap<String, i64>, i64), AccountError> {
        let mut by_asset: HashMap<String, i64> = HashMap::new();
        let mut total: i64 = 0;
        for t in all_txs {
            if t.transaction_type != TransactionType::ManagementFee {
                continue;
            }
            if let Some(as_of) = as_of {
                if t.date.as_str() > as_of {
                    continue;
                }
            }
            let Some(asset) = self.asset_service.get_asset_by_id(&t.asset_id).await.map_err(|e| {
                tracing::error!(target: BACKEND, asset_id = %t.asset_id, err = ?e, "compute_management_fees: get_asset_by_id failed");
                AccountError::DatabaseError
            })?
            else {
                continue;
            };
            let prices = self.asset_service.get_asset_prices(&t.asset_id).await.map_err(|e| {
                tracing::error!(target: BACKEND, asset_id = %t.asset_id, err = ?e, "compute_management_fees: get_asset_prices failed");
                AccountError::DatabaseError
            })?;
            // PRF-022 carry-forward: latest recorded price dated ≤ the fee's date (prices are date-desc).
            let Some(price) = prices.iter().find(|p| p.date.as_str() <= t.date.as_str()) else {
                continue; // FEE-054 — no recorded price → contributes 0
            };
            let value_asset_ccy = (t.quantity as i128 * price.price as i128 / 1_000_000) as i64;
            let fee_value = if asset.currency == account_currency {
                value_asset_ccy
            } else {
                match self
                    .currency_service
                    .resolve_rate(&asset.currency, account_currency, &t.date)
                    .await
                    .map_err(|e| {
                        tracing::error!(target: BACKEND, err = ?e, "compute_management_fees: resolve_rate failed");
                        AccountError::DatabaseError
                    })? {
                    // FEE-073 — convert the fee value to account currency as of the fee's date.
                    Some(resolved) => {
                        (value_asset_ccy as i128 * resolved.rate_micros as i128 / 1_000_000) as i64
                    }
                    // FEE-073/054 — no usable rate → contributes 0.
                    None => 0,
                }
            };
            let entry = by_asset.entry(t.asset_id.clone()).or_insert(0);
            *entry = entry.saturating_add(fee_value);
            total = total.saturating_add(fee_value);
        }
        Ok((by_asset, total))
    }

    /// As-of view: reconstructs the account exactly as it stood on `as_of_date`
    /// from the transaction history (read-only). Mirrors the live view's DTO and
    /// arithmetic (i128 intermediates, Option-on-missing-price/FX), substituting
    /// per-asset replay for the stored Holding aggregates and carry-forward
    /// price/FX lookups for the live "latest" ones.
    async fn get_account_details_as_of(
        &self,
        account_id: &str,
        as_of_date: &str,
    ) -> StdResult<AccountDetailsResponse, AccountError> {
        // Validate the date: ISO YYYY-MM-DD, not in the future.
        let as_of = NaiveDate::parse_from_str(as_of_date, "%Y-%m-%d")
            .map_err(|_| AccountError::InvalidDate)?;
        if as_of > Local::now().date_naive() {
            return Err(AccountError::DateInFuture);
        }

        let account = self
            .account_service
            .get_by_id(account_id)
            .await?
            .ok_or_else(|| AccountError::AccountNotFound {
                account_id: account_id.to_string(),
            })?;

        let all_txs = self
            .account_service
            .get_all_transactions_for_account(account_id)
            .await?;

        // DIV-070/073 — dividends credited on or before the as-of date, per asset
        // and in total. Dividends are stored in account currency.
        let mut dividends_by_asset: HashMap<String, i64> = HashMap::new();
        let mut total_dividends_received: i64 = 0;
        // ACD-053 — net cash input as of the date: deposits − withdrawals dated ≤ as_of.
        let mut total_net_cash_input: i64 = 0;
        for t in &all_txs {
            if t.date.as_str() > as_of_date {
                continue;
            }
            match t.transaction_type {
                TransactionType::Dividend => {
                    let entry = dividends_by_asset.entry(t.asset_id.clone()).or_insert(0);
                    *entry = entry.saturating_add(t.total_amount);
                    total_dividends_received =
                        total_dividends_received.saturating_add(t.total_amount);
                }
                TransactionType::Deposit => {
                    total_net_cash_input = total_net_cash_input.saturating_add(t.total_amount);
                }
                TransactionType::Withdrawal => {
                    total_net_cash_input = total_net_cash_input.saturating_sub(t.total_amount);
                }
                _ => {}
            }
        }

        // FEE-052/053/072 — cumulative management fees per asset + total, fees dated ≤ as_of.
        let (management_fees_by_asset, total_management_fees) = self
            .compute_management_fees(&all_txs, &account.currency, Some(as_of_date))
            .await?;

        let mut details: Vec<HoldingDetail> = Vec::new();
        let mut closed_details: Vec<ClosedHoldingDetail> = Vec::new();
        let mut total_global_value: i64 = 0;
        let mut total_cost_basis: i64 = 0;
        let mut total_realized_pnl: i64 = 0;

        // Distinct non-cash assets with at least one transaction on or before the
        // as-of date. Each reconstructs to an active (qty > 0), closed (qty 0 with
        // a sell), or absent holding on the date.
        let mut seen: HashSet<&str> = HashSet::new();
        for transaction in &all_txs {
            let asset_id = transaction.asset_id.as_str();
            if is_cash_asset(asset_id) || transaction.date.as_str() > as_of_date {
                continue;
            }
            if !seen.insert(asset_id) {
                continue;
            }

            let reconstruction = Account::reconstruct_holding_as_of(&all_txs, asset_id, as_of_date);
            let realized_pnl = reconstruction.total_realized_pnl;
            total_realized_pnl = total_realized_pnl.saturating_add(realized_pnl);

            let asset = self
                .asset_service
                .get_asset_by_id(asset_id)
                .await
                .map_err(|e| {
                    tracing::error!(target: BACKEND, asset_id = %asset_id, err = ?e, "get_account_details_as_of: get_asset_by_id failed");
                    AccountError::DatabaseError
                })?
                .ok_or_else(|| {
                    tracing::error!(target: BACKEND, asset_id = %asset_id, "get_account_details_as_of: transaction references missing asset");
                    AccountError::DatabaseError
                })?;

            // Closed-as-of: qty 0 on the date with a recorded sell → closed row.
            if reconstruction.quantity <= 0 {
                if let Some(last_sold_date) = reconstruction.last_sold_date {
                    let dividends_received = *dividends_by_asset.get(asset_id).unwrap_or(&0);
                    closed_details.push(ClosedHoldingDetail {
                        asset_id: asset_id.to_string(),
                        asset_name: asset.name,
                        asset_reference: asset.reference,
                        realized_pnl,
                        dividends_received,
                        last_sold_date,
                    });
                }
                continue;
            }

            // Active-as-of: enrich with the carry-forward price + FX as of the date.
            let cost_basis = (reconstruction.quantity as i128
                * reconstruction.average_price as i128
                / 1_000_000) as i64;
            total_cost_basis = total_cost_basis.saturating_add(cost_basis);

            // FXR-035 — resolve the rate to value this holding in the account
            // currency as of the date. Identity pair → 1.0 without a lookup.
            let resolved_rate = self
                .currency_service
                .resolve_rate(&asset.currency, &account.currency, as_of_date)
                .await
                .map_err(|e| {
                    tracing::error!(target: BACKEND, asset_id = %asset_id, err = ?e, "get_account_details_as_of: resolve_rate failed");
                    AccountError::DatabaseError
                })?;
            let conversion_rate: Option<i64> =
                resolved_rate.as_ref().map(|resolved| resolved.rate_micros);
            let fx_rate_date: Option<String> =
                resolved_rate.and_then(|resolved| resolved.rate_date);

            // Carry-forward price: latest recorded price dated on or before the
            // as-of date (asset native currency). get_asset_prices is date DESC,
            // so the first match is the most recent on or before the date.
            let prices = self
                .asset_service
                .get_asset_prices(asset_id)
                .await
                .map_err(|e| {
                    tracing::error!(target: BACKEND, asset_id = %asset_id, err = ?e, "get_account_details_as_of: get_asset_prices failed");
                    AccountError::DatabaseError
                })?;
            let price_as_of = prices.iter().find(|p| p.date.as_str() <= as_of_date);

            let (
                current_price,
                current_price_date,
                current_price_source,
                unrealized_pnl,
                performance_pct,
            ) = if let Some(price) = price_as_of {
                let current_price = price.price;
                // FXR-030/031/032 — value in account currency using the resolved
                // rate; same-currency resolves to 1.0. No usable rate → P&L None.
                let (unrealized_pnl, performance_pct) = match conversion_rate {
                    Some(rate) => {
                        let converted_price =
                            (current_price as i128 * rate as i128 / 1_000_000) as i64;
                        let unrealized_pnl = ((converted_price as i128
                            - reconstruction.average_price as i128)
                            * reconstruction.quantity as i128
                            / 1_000_000) as i64;
                        let performance_pct = if cost_basis != 0 {
                            Some((unrealized_pnl as i128 * 100_000_000 / cost_basis as i128) as i64)
                        } else {
                            None
                        };
                        (Some(unrealized_pnl), performance_pct)
                    }
                    None => (None, None),
                };
                (
                    Some(current_price),
                    Some(price.date.clone()),
                    Some(price.source.clone()),
                    unrealized_pnl,
                    performance_pct,
                )
            } else {
                (None, None, None, None, None)
            };

            // CSH-094 / FXR-041 / ACD-052 — market value in account currency as of
            // the date; a priced holding contributes it to the Global Value, a
            // holding with no price or no usable rate carries None.
            let market_value: Option<i64> = if let (Some(cp), Some(rate)) =
                (current_price, conversion_rate)
            {
                let converted_price = (cp as i128 * rate as i128 / 1_000_000) as i64;
                Some((reconstruction.quantity as i128 * converted_price as i128 / 1_000_000) as i64)
            } else {
                None
            };
            if let Some(value) = market_value {
                total_global_value = total_global_value.saturating_add(value);
            }

            let dividends_received = *dividends_by_asset.get(asset_id).unwrap_or(&0);
            let total_return_pct: Option<i64> = match unrealized_pnl {
                Some(upnl) if cost_basis != 0 => Some(
                    ((upnl as i128 + dividends_received as i128) * 100_000_000 / cost_basis as i128)
                        as i64,
                ),
                _ => None,
            };

            details.push(HoldingDetail {
                asset_id: asset_id.to_string(),
                asset_name: asset.name,
                asset_reference: asset.reference,
                quantity: reconstruction.quantity,
                average_price: reconstruction.average_price,
                cost_basis,
                realized_pnl,
                asset_currency: asset.currency,
                current_price,
                current_price_date,
                current_price_source,
                unrealized_pnl,
                performance_pct,
                dividends_received,
                total_return_pct,
                fx_rate_date,
                management_fees: *management_fees_by_asset.get(asset_id).unwrap_or(&0), // FEE-052
                market_value,                                                           // ACD-052
            });
        }

        // CSH-090/094 — include the system Cash Holding with its balance on the
        // date. Cash carries no cost basis and no price; its quantity counts as
        // account-currency value directly.
        let cash_balance = Account::cash_balance_as_of(&all_txs, as_of_date);
        if cash_balance > 0 {
            let cash_asset_id = system_cash_asset_id(&account.currency);
            let cash_asset = self
                .asset_service
                .get_asset_by_id(&cash_asset_id)
                .await
                .map_err(|e| {
                    tracing::error!(target: BACKEND, asset_id = %cash_asset_id, err = ?e, "get_account_details_as_of: get cash asset failed");
                    AccountError::DatabaseError
                })?
                .ok_or_else(|| {
                    tracing::error!(target: BACKEND, asset_id = %cash_asset_id, "get_account_details_as_of: cash asset missing");
                    AccountError::DatabaseError
                })?;
            total_global_value = total_global_value.saturating_add(cash_balance);
            details.push(HoldingDetail {
                asset_id: cash_asset_id,
                asset_name: cash_asset.name,
                asset_reference: cash_asset.reference,
                quantity: cash_balance,
                average_price: 1_000_000,
                cost_basis: 0,
                realized_pnl: 0,
                asset_currency: account.currency.clone(),
                current_price: None,
                current_price_date: None,
                current_price_source: None,
                unrealized_pnl: None,
                performance_pct: None,
                dividends_received: 0,
                total_return_pct: None,
                fx_rate_date: None,
                management_fees: 0, // cash holdings never have management fees
                market_value: Some(cash_balance), // ACD-052 — cash value is its balance
            });
        }

        // ACD-033/046 — sort each section alphabetically by asset_name ascending.
        details.sort_by(|a, b| a.asset_name.cmp(&b.asset_name));
        closed_details.sort_by(|a, b| a.asset_name.cmp(&b.asset_name));

        // MKT-040 — sum unrealized_pnl across qualifying holdings; None when none.
        let qualifying_pnls: Vec<i64> = details.iter().filter_map(|d| d.unrealized_pnl).collect();
        let total_unrealized_pnl = if qualifying_pnls.is_empty() {
            None
        } else {
            Some(qualifying_pnls.iter().sum())
        };

        let total_holding_count = (details.len() + closed_details.len()) as i64;

        Ok(AccountDetailsResponse {
            account_name: account.name,
            holdings: details,
            closed_holdings: closed_details,
            total_holding_count,
            total_cost_basis,
            total_realized_pnl,
            total_unrealized_pnl,
            total_global_value,
            total_dividends_received,
            total_management_fees, // FEE-053/072
            total_net_cash_input,  // ACD-053
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::account::{
        AccountService, Holding, HoldingRepository, SqliteAccountRepository,
        SqliteHoldingRepository, SqliteTransactionRepository, UpdateFrequency,
    };
    use crate::context::asset::AssetService;
    use crate::context::asset::{
        AssetClass, CreateAssetDTO, SqliteAssetCategoryRepository, SqliteAssetRepository,
        SYSTEM_CATEGORY_ID,
    };
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup(pool: &sqlx::Pool<sqlx::Sqlite>) -> (Arc<AccountService>, Arc<AssetService>) {
        let account_svc = Arc::new(AccountService::new(
            Box::new(SqliteAccountRepository::new(pool.clone())),
            Box::new(SqliteHoldingRepository::new(pool.clone())),
            Box::new(SqliteTransactionRepository::new(pool.clone())),
        ));
        let asset_svc = Arc::new(AssetService::new(
            Box::new(SqliteAssetRepository::new(pool.clone())),
            Box::new(SqliteAssetCategoryRepository::new(pool.clone())),
            Box::new(crate::context::asset::SqliteAssetPriceRepository::new(
                pool.clone(),
            )),
        ));
        (account_svc, asset_svc)
    }

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

    // ACD-012 — unknown account returns AccountError::AccountNotFound with id payload
    #[tokio::test]
    async fn unknown_account_returns_error() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let err = uc
            .get_account_details("nonexistent-id", None)
            .await
            .unwrap_err();
        assert!(
            matches!(
                &err,
                AccountError::AccountNotFound { account_id }
                    if account_id == "nonexistent-id"
            ),
            "got: {err:?}"
        );
    }

    // ACD-020 — holdings with quantity == 0 are excluded; ACD-034 — total_holding_count counts all
    #[tokio::test]
    async fn zero_quantity_holdings_excluded_from_active() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create(
                "Test".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "AAPL".to_string(),
                reference: "AAPL".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 3,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();

        // Insert a zero-quantity holding directly via repo
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(Holding::new(account.id.clone(), asset.id.clone(), 0, 0, 0, None).unwrap())
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();

        assert_eq!(resp.holdings.len(), 0, "active holdings should be empty");
        assert_eq!(
            resp.total_holding_count, 1,
            "total count should include zero-qty holding"
        );
        assert_eq!(resp.total_cost_basis, 0);
    }

    // ACD-023/024 — cost basis uses i128 intermediates; ACD-031 — total_cost_basis is sum
    #[tokio::test]
    async fn cost_basis_and_total_computed_correctly() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create(
                "Portfolio".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Bond".to_string(),
                reference: "BOND".to_string(),
                isin: None,
                class: AssetClass::Bonds,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();

        // 2.0 units at 100.00 → cost_basis = 200_000_000 micros = 200.00
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    2_000_000,   // 2.0 units
                    100_000_000, // 100.00
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();

        assert_eq!(resp.holdings.len(), 1);
        assert_eq!(resp.holdings[0].cost_basis, 200_000_000);
        assert_eq!(resp.total_cost_basis, 200_000_000);
    }

    // ACD-021 — holdings for archived assets are included (quantity > 0)
    #[tokio::test]
    async fn archived_asset_holding_included() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create(
                "Archived Test".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Archived Stock".to_string(),
                reference: "ARCH".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 2,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();

        // Archive the asset
        asset_svc.archive_asset(&asset.id).await.unwrap();

        // Insert a positive-quantity holding for the archived asset
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    1_000_000,
                    50_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();

        assert_eq!(
            resp.holdings.len(),
            1,
            "archived asset holding should be included"
        );
        assert_eq!(resp.holdings[0].asset_reference, "ARCH");
    }

    // ACD-032 — account_name is present in the response
    #[tokio::test]
    async fn account_name_present_in_response() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "My Account".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(resp.account_name, "My Account");
    }

    // ACD-043 — Holding entity exposes last_sold_date: Option<String> and total_realized_pnl: i64
    #[tokio::test]
    async fn holding_entity_carries_last_sold_date_and_total_realized_pnl() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "X".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "A".to_string(),
                reference: "A".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    0,
                    50_000_000,
                    15_000_000, // total_realized_pnl
                    Some("2026-01-15".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let holdings = holding_repo.get_by_account(&account.id).await.unwrap();
        assert_eq!(holdings.len(), 1);
        assert_eq!(holdings[0].total_realized_pnl, 15_000_000);
        assert_eq!(holdings[0].last_sold_date.as_deref(), Some("2026-01-15"));
    }

    // ACD-044 — closed_holdings contains holdings with quantity == 0; active holdings do not
    #[tokio::test]
    async fn closed_holdings_contains_zero_qty_holdings() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Acct".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Closed Co".to_string(),
                reference: "CC".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    0,
                    50_000_000,
                    5_000_000,
                    Some("2025-12-01".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(resp.holdings.len(), 0);
        assert_eq!(resp.closed_holdings.len(), 1);
        assert_eq!(resp.closed_holdings[0].asset_reference, "CC");
    }

    // ACD-044 — closed holdings are enriched with asset_name and asset_reference
    #[tokio::test]
    async fn closed_holdings_enriched_with_asset_metadata() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Acct".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Meta Inc".to_string(),
                reference: "META".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 3,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id,
                    0,
                    0,
                    1_000_000,
                    Some("2026-03-10".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(resp.closed_holdings[0].asset_name, "Meta Inc");
        assert_eq!(resp.closed_holdings[0].asset_reference, "META");
    }

    // ACD-045 — ClosedHoldingDetail.realized_pnl equals Holding.total_realized_pnl
    #[tokio::test]
    async fn closed_holding_detail_realized_pnl_matches_holding() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "P".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Q".to_string(),
                reference: "Q".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id,
                    0,
                    0,
                    42_000_000,
                    Some("2026-02-01".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(resp.closed_holdings[0].realized_pnl, 42_000_000);
    }

    // DIV-073 — ClosedHoldingDetail.dividends_received sums Dividend cash for the position
    #[tokio::test]
    async fn closed_holding_detail_carries_dividends_received() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "P".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Q".to_string(),
                reference: "Q".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        // Cash deposit establishes the cash holding the dividends credit into.
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2025-01-05".to_string(), 100_000_000, None)
            .await
            .unwrap();
        // Two dividends recorded over the life of the (now closed) position.
        account_svc
            .record_dividend(
                &account.id,
                asset.id.clone(),
                "2025-06-01".to_string(),
                3_000_000,
                1_000_000,
                None,
            )
            .await
            .unwrap();
        account_svc
            .record_dividend(
                &account.id,
                asset.id.clone(),
                "2025-09-01".to_string(),
                2_000_000,
                1_000_000,
                None,
            )
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id,
                    0,
                    0,
                    42_000_000,
                    Some("2026-02-01".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(resp.closed_holdings[0].dividends_received, 5_000_000);
    }

    // ACD-045 — last_sold_date on ClosedHoldingDetail is non-optional String from Holding
    #[tokio::test]
    async fn closed_holding_detail_last_sold_date_is_non_optional() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "D".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "E".to_string(),
                reference: "E".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id,
                    0,
                    0,
                    0,
                    Some("2025-11-30".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(resp.closed_holdings[0].last_sold_date, "2025-11-30");
    }

    // ACD-045 — holdings with last_sold_date == None are excluded from closed_holdings
    #[tokio::test]
    async fn holding_without_last_sold_date_excluded_from_closed_holdings() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "F".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "G".to_string(),
                reference: "G".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        // qty=0 but no last_sold_date — should not appear in closed_holdings
        holding_repo
            .upsert(Holding::new(account.id.clone(), asset.id, 0, 0, 0, None).unwrap())
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(resp.closed_holdings.len(), 0);
    }

    // ACD-046 — closed_holdings sorted by asset_name ascending
    #[tokio::test]
    async fn closed_holdings_sorted_by_asset_name_ascending() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "H".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        for (name, reference) in [("Zebra", "ZBR"), ("Alpha", "ALP"), ("Mango", "MNG")] {
            let asset = asset_svc
                .create_asset(CreateAssetDTO {
                    name: name.to_string(),
                    reference: reference.to_string(),
                    isin: None,
                    class: AssetClass::Stocks,
                    currency: "EUR".to_string(),
                    risk_level: 1,
                    category_id: SYSTEM_CATEGORY_ID.to_string(),
                    exchange: None,
                })
                .await
                .unwrap();
            holding_repo
                .upsert(
                    Holding::new(
                        account.id.clone(),
                        asset.id,
                        0,
                        0,
                        0,
                        Some("2026-01-01".to_string()),
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
        }

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        let names: Vec<&str> = resp
            .closed_holdings
            .iter()
            .map(|h| h.asset_name.as_str())
            .collect();
        assert_eq!(names, vec!["Alpha", "Mango", "Zebra"]);
    }

    // ACD-047 — total_realized_pnl is sum across ALL holdings (active + closed)
    #[tokio::test]
    async fn total_realized_pnl_sums_active_and_closed_holdings() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "I".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());

        // Active holding with partial sells (pnl = 10)
        let asset1 = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Active".to_string(),
                reference: "ACT".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset1.id,
                    1_000_000,
                    50_000_000,
                    10_000_000,
                    Some("2025-06-01".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        // Closed holding (pnl = 25)
        let asset2 = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Closed".to_string(),
                reference: "CLO".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset2.id,
                    0,
                    0,
                    25_000_000,
                    Some("2026-01-10".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(resp.total_realized_pnl, 35_000_000);
    }

    // ACD-047 — total_realized_pnl is 0 when no holdings have realized P&L
    #[tokio::test]
    async fn total_realized_pnl_is_zero_when_no_sells() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "J".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(resp.total_realized_pnl, 0);
    }

    // ACD-050 — closed_holdings is empty list when no closed positions exist
    #[tokio::test]
    async fn closed_holdings_empty_when_no_closed_positions() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "K".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert!(resp.closed_holdings.is_empty());
    }

    // MKT-031 — unrealized_pnl is None on a HoldingDetail when no price has been recorded
    #[tokio::test]
    async fn holding_detail_unrealized_pnl_is_none_when_no_price() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "X".to_string(),
                reference: "X".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id,
                    1_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert!(resp.holdings[0].unrealized_pnl.is_none());
        assert!(resp.holdings[0].current_price.is_none());
    }

    // MKT-033 — unrealized_pnl is computed when asset currency matches account currency
    #[tokio::test]
    async fn holding_detail_unrealized_pnl_computed_same_currency() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "X".to_string(),
                reference: "X".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(), // same as account
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        // 2 units at avg_price 100.00 → cost_basis = 200.00
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    2_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        // current_price = 110.00 → unrealized_pnl = (110 - 100) * 2 = 20.00
        asset_svc
            .record_asset_price(&asset.id, "2026-01-01", 110.0)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(resp.holdings[0].unrealized_pnl, Some(20_000_000));
    }

    // FXR-034 — unrealized_pnl is None when a foreign holding has no usable rate
    // (amends MKT-034: mismatch alone no longer forces None — only rate absence does)
    #[tokio::test]
    async fn holding_detail_unrealized_pnl_is_none_on_currency_mismatch() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "X".to_string(),
                reference: "X".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "USD".to_string(), // differs from account EUR
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    1_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        asset_svc
            .record_asset_price(&asset.id, "2026-01-01", 110.0)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        // current_price present (raw asset-currency price), but P&L is None — no usable rate
        assert!(resp.holdings[0].current_price.is_some());
        assert!(resp.holdings[0].unrealized_pnl.is_none());
        assert!(resp.holdings[0].performance_pct.is_none());
    }

    // ACD-052 — market_value is the priced holding's value in account currency and
    // matches its contribution to total_global_value
    #[tokio::test]
    async fn market_value_present_for_priced_same_currency_holding() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "X".to_string(),
                reference: "X".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    2_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        asset_svc
            .record_asset_price(&asset.id, "2026-01-01", 110.0)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        // 2 units × 110.00 = 220.00 in account currency
        assert_eq!(resp.holdings[0].market_value, Some(220_000_000));
        assert_eq!(resp.total_global_value, 220_000_000);
    }

    // ACD-052 — market_value is None for an unpriced holding and for a foreign
    // holding with no usable FX rate (FXR-034)
    #[tokio::test]
    async fn market_value_none_when_unpriced_or_no_usable_rate() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let unpriced = asset_svc
            .create_asset(CreateAssetDTO {
                name: "NoPrice".to_string(),
                reference: "NOP".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        let foreign = asset_svc
            .create_asset(CreateAssetDTO {
                name: "NoRate".to_string(),
                reference: "NOR".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        for asset_id in [unpriced.id.clone(), foreign.id.clone()] {
            holding_repo
                .upsert(
                    Holding::new(
                        account.id.clone(),
                        asset_id,
                        1_000_000,
                        100_000_000,
                        0,
                        None,
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
        }
        asset_svc
            .record_asset_price(&foreign.id, "2026-01-01", 110.0)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        for holding in &resp.holdings {
            assert_eq!(holding.market_value, None, "asset {}", holding.asset_name);
        }
        assert_eq!(resp.total_global_value, 0);
    }

    // ACD-052 — the Cash Holding's market_value is its balance (already account currency)
    #[tokio::test]
    async fn market_value_of_cash_holding_is_its_balance() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2026-01-05".to_string(), 100_000_000, None)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        let cash = resp
            .holdings
            .iter()
            .find(|h| crate::core::cash::is_cash_asset(&h.asset_id))
            .expect("cash holding present");
        assert_eq!(cash.market_value, Some(100_000_000));
        assert_eq!(resp.total_global_value, 100_000_000);
    }

    // ACD-053 — total_net_cash_input sums deposits minus withdrawals since inception
    #[tokio::test]
    async fn net_cash_input_sums_deposits_minus_withdrawals() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2026-01-05".to_string(), 100_000_000, None)
            .await
            .unwrap();
        account_svc
            .record_deposit(&account.id, "2026-02-10".to_string(), 50_000_000, None)
            .await
            .unwrap();
        account_svc
            .record_withdrawal(&account.id, "2026-03-01".to_string(), 30_000_000, None)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(resp.total_net_cash_input, 120_000_000);
    }

    // ACD-053 — total_net_cash_input is 0 for an account with no cash transactions
    #[tokio::test]
    async fn net_cash_input_is_zero_without_cash_transactions() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(resp.total_net_cash_input, 0);
    }

    // ACD-053 — the as-of view counts only cash transactions dated on or before the date
    #[tokio::test]
    async fn net_cash_input_as_of_excludes_later_transactions() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2026-01-05".to_string(), 100_000_000, None)
            .await
            .unwrap();
        account_svc
            .record_deposit(&account.id, "2026-03-01".to_string(), 50_000_000, None)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc
            .get_account_details(&account.id, Some("2026-02-01"))
            .await
            .unwrap();
        assert_eq!(resp.total_net_cash_input, 100_000_000);
    }

    // MKT-035 — performance_pct is None when cost_basis is zero
    #[tokio::test]
    async fn holding_detail_performance_pct_is_none_when_cost_basis_zero() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "X".to_string(),
                reference: "X".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        // average_price = 0 → cost_basis = 0
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(account.id.clone(), asset.id.clone(), 1_000_000, 0, 0, None).unwrap(),
            )
            .await
            .unwrap();
        asset_svc
            .record_asset_price(&asset.id, "2026-01-01", 50.0)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert!(resp.holdings[0].performance_pct.is_none());
    }

    // MKT-035 — performance_pct is computed correctly (5.25% = 5_250_000 micros)
    #[tokio::test]
    async fn holding_detail_performance_pct_computed_correctly() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "X".to_string(),
                reference: "X".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        // 2 units at avg 100.00 → cost_basis = 200_000_000 micros
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    2_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        // current_price = 110.00 → unrealized = 20.00 → perf = 20/200 = 10% = 10_000_000 micros
        asset_svc
            .record_asset_price(&asset.id, "2026-01-01", 110.0)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(resp.holdings[0].performance_pct, Some(10_000_000));
    }

    // MKT-033 — unrealized_pnl is Some(0) (not None) when current_price equals average_price
    #[tokio::test]
    async fn holding_detail_unrealized_pnl_is_zero_when_price_equals_average() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "X".to_string(),
                reference: "X".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        // avg_price = 100.00, qty = 2
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    2_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        // current_price == average_price → unrealized_pnl = 0
        asset_svc
            .record_asset_price(&asset.id, "2026-01-01", 100.0)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(resp.holdings[0].unrealized_pnl, Some(0));
    }

    // MKT-035 — performance_pct is Some(0) (not None) when unrealized_pnl is 0 and cost_basis nonzero
    #[tokio::test]
    async fn holding_detail_performance_pct_is_zero_when_unrealized_pnl_is_zero() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "X".to_string(),
                reference: "X".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        // avg_price = 100.00, cost_basis = 100_000_000 micros
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    1_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        // current_price == average_price → unrealized_pnl = 0 → perf = 0%
        asset_svc
            .record_asset_price(&asset.id, "2026-01-01", 100.0)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(resp.holdings[0].unrealized_pnl, Some(0));
        assert_eq!(resp.holdings[0].performance_pct, Some(0));
    }

    // MKT-040 — total_unrealized_pnl is None when no holding has a computable value
    #[tokio::test]
    async fn total_unrealized_pnl_is_none_when_no_qualifying_holdings() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        // No holdings at all → None
        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert!(resp.total_unrealized_pnl.is_none());
    }

    // MKT-040 — total_unrealized_pnl sums unrealized_pnl across same-currency priced active holdings
    #[tokio::test]
    async fn total_unrealized_pnl_sums_qualifying_holdings() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();

        // Holding 1: EUR asset, gain 20.00
        let asset1 = asset_svc
            .create_asset(CreateAssetDTO {
                name: "A1".to_string(),
                reference: "A1".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset1.id.clone(),
                    2_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        asset_svc
            .record_asset_price(&asset1.id, "2026-01-01", 110.0)
            .await
            .unwrap();

        // Holding 2: USD asset (mismatch) — should NOT contribute
        let asset2 = asset_svc
            .create_asset(CreateAssetDTO {
                name: "A2".to_string(),
                reference: "A2".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset2.id.clone(),
                    1_000_000,
                    50_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        asset_svc
            .record_asset_price(&asset2.id, "2026-01-01", 100.0)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        // Only holding 1 qualifies: unrealized_pnl = 20_000_000
        assert_eq!(resp.total_unrealized_pnl, Some(20_000_000));
    }

    // ACD-033 — holdings sorted by asset_name ascending
    #[tokio::test]
    async fn holdings_sorted_by_asset_name_ascending() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create(
                "Alpha".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();

        for (name, reference) in [
            ("Zebra Fund", "ZBR"),
            ("Apple Inc", "AAPL"),
            ("Microsoft", "MSFT"),
        ] {
            let asset = asset_svc
                .create_asset(CreateAssetDTO {
                    name: name.to_string(),
                    reference: reference.to_string(),
                    isin: None,
                    class: AssetClass::Stocks,
                    currency: "USD".to_string(),
                    risk_level: 2,
                    category_id: SYSTEM_CATEGORY_ID.to_string(),
                    exchange: None,
                })
                .await
                .unwrap();
            let holding_repo = SqliteHoldingRepository::new(pool.clone());
            holding_repo
                .upsert(
                    Holding::new(account.id.clone(), asset.id, 1_000_000, 50_000_000, 0, None)
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();

        let names: Vec<&str> = resp
            .holdings
            .iter()
            .map(|h| h.asset_name.as_str())
            .collect();
        assert_eq!(names, vec!["Apple Inc", "Microsoft", "Zebra Fund"]);
    }

    // CSH-093 — total_cost_basis sums non-cash holdings only.
    // CSH-094 — total_global_value = cash quantity + Σ priced same-currency non-cash holdings.
    // CSH-091 — cash row's cost_basis is 0 in the response (rendered blank in UI).
    #[tokio::test]
    async fn cash_holding_excluded_from_cost_basis_and_added_to_global_value() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create(
                "Cash + Stocks".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        // Seed the cash asset for the account currency, then a deposit so the cash
        // holding exists with a known balance (250.00 EUR in micros).
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2020-01-01".to_string(), 250_000_000, None)
            .await
            .unwrap();

        // Add a non-cash holding worth 200 EUR cost basis. No price recorded yet,
        // so it contributes 0 to the global value (CSH-094 no-fallback semantic).
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Bond".to_string(),
                reference: "BOND".to_string(),
                isin: None,
                class: AssetClass::Bonds,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    2_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();

        // Two active holdings: cash + bond.
        assert_eq!(resp.holdings.len(), 2);

        // CSH-091: cash row carries cost_basis = 0; bond row keeps its 200 EUR cost basis.
        let cash_row = resp
            .holdings
            .iter()
            .find(|h| h.asset_id.starts_with("system-cash-"))
            .expect("cash holding present");
        let bond_row = resp
            .holdings
            .iter()
            .find(|h| h.asset_id == asset.id)
            .expect("bond holding present");
        assert_eq!(cash_row.cost_basis, 0, "cash holding has no cost basis");
        assert_eq!(bond_row.cost_basis, 200_000_000, "bond cost basis = 200");

        // CSH-093: total_cost_basis sums non-cash only.
        assert_eq!(
            resp.total_cost_basis, 200_000_000,
            "total_cost_basis must exclude the cash holding (CSH-093)"
        );

        // CSH-094: with no recorded price for the bond, global value is cash only.
        assert_eq!(
            resp.total_global_value, 250_000_000,
            "global value = cash (250) + bond (0, unpriced) = 250"
        );
    }

    // CSH-090 / CSH-097 — Cash row always in the active holdings even at quantity 0
    // (exempt from ACD-020's quantity > 0 filter; never placed in closed_holdings).
    // Setup: deposit then withdraw the full balance so the cash holding sits at 0.
    #[tokio::test]
    async fn cash_holding_shown_in_active_when_quantity_is_zero() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create(
                "Cash drained".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2020-01-01".to_string(), 100_000_000, None)
            .await
            .unwrap();
        account_svc
            .record_withdrawal(&account.id, "2020-02-01".to_string(), 100_000_000, None)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();

        let cash = resp
            .holdings
            .iter()
            .find(|h| h.asset_id.starts_with("system-cash-"))
            .expect("cash row must be present in active holdings even at quantity 0 (CSH-090)");
        assert_eq!(cash.quantity, 0);
        assert!(
            !resp
                .closed_holdings
                .iter()
                .any(|h| h.asset_id.starts_with("system-cash-")),
            "cash row must never be placed in closed_holdings (ACD-044)"
        );
    }

    // -------------------------------------------------------------------------
    // Asset-side failure translation (gold rule coverage)
    // -------------------------------------------------------------------------
    //
    // Each test seeds a real holding via the Sqlite-backed account_svc, then
    // injects a failing asset_svc (mocked asset_repo) so the orchestrator hits
    // the corresponding asset-lookup branch. Active-loop and closed-loop sites
    // mirror each other but have separate coverage so both branches are exercised.

    use crate::context::asset::{
        MockAssetCategoryRepository, MockAssetPriceRepository, MockAssetRepository,
    };

    fn failing_asset_svc(ar_setup: impl FnOnce(&mut MockAssetRepository)) -> Arc<AssetService> {
        let mut ar = MockAssetRepository::new();
        ar_setup(&mut ar);
        Arc::new(AssetService::new(
            Box::new(ar),
            Box::new(MockAssetCategoryRepository::new()),
            Box::new(MockAssetPriceRepository::new()),
        ))
    }

    async fn seed_account_with_active_holding(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        account_svc: &Arc<AccountService>,
        real_asset_svc: &Arc<AssetService>,
    ) -> (String, String) {
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = real_asset_svc
            .create_asset(CreateAssetDTO {
                name: "X".to_string(),
                reference: "X".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    1_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        (account.id, asset.id)
    }

    async fn seed_account_with_closed_holding(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        account_svc: &Arc<AccountService>,
        real_asset_svc: &Arc<AssetService>,
    ) -> (String, String) {
        let account = account_svc
            .create(
                "A".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = real_asset_svc
            .create_asset(CreateAssetDTO {
                name: "X".to_string(),
                reference: "X".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    0,
                    50_000_000,
                    0,
                    Some("2026-01-01".to_string()),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        (account.id, asset.id)
    }

    // Active loop: asset-repo Err → DatabaseError
    #[tokio::test]
    async fn get_account_details_translates_active_loop_asset_repo_failure() {
        let pool = make_pool().await;
        let (account_svc, real_asset_svc) = setup(&pool).await;
        let (account_id, _) =
            seed_account_with_active_holding(&pool, &account_svc, &real_asset_svc).await;

        let asset_svc = failing_asset_svc(|ar| {
            ar.expect_get_by_id()
                .returning(|_| Err(anyhow::anyhow!("simulated asset repo failure")));
        });
        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );

        let err = uc.get_account_details(&account_id, None).await.unwrap_err();
        assert!(matches!(err, AccountError::DatabaseError), "got: {err:?}");
    }

    // Active loop: asset-repo Ok(None) (FK integrity violation) → DatabaseError
    #[tokio::test]
    async fn get_account_details_translates_active_loop_missing_asset() {
        let pool = make_pool().await;
        let (account_svc, real_asset_svc) = setup(&pool).await;
        let (account_id, _) =
            seed_account_with_active_holding(&pool, &account_svc, &real_asset_svc).await;

        let asset_svc = failing_asset_svc(|ar| {
            ar.expect_get_by_id().returning(|_| Ok(None));
        });
        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );

        let err = uc.get_account_details(&account_id, None).await.unwrap_err();
        assert!(matches!(err, AccountError::DatabaseError), "got: {err:?}");
    }

    // Closed loop: asset-repo Err → DatabaseError
    #[tokio::test]
    async fn get_account_details_translates_closed_loop_asset_repo_failure() {
        let pool = make_pool().await;
        let (account_svc, real_asset_svc) = setup(&pool).await;
        let (account_id, _) =
            seed_account_with_closed_holding(&pool, &account_svc, &real_asset_svc).await;

        let asset_svc = failing_asset_svc(|ar| {
            ar.expect_get_by_id()
                .returning(|_| Err(anyhow::anyhow!("simulated asset repo failure")));
        });
        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );

        let err = uc.get_account_details(&account_id, None).await.unwrap_err();
        assert!(matches!(err, AccountError::DatabaseError), "got: {err:?}");
    }

    // Closed loop: asset-repo Ok(None) (FK integrity violation) → DatabaseError
    #[tokio::test]
    async fn get_account_details_translates_closed_loop_missing_asset() {
        let pool = make_pool().await;
        let (account_svc, real_asset_svc) = setup(&pool).await;
        let (account_id, _) =
            seed_account_with_closed_holding(&pool, &account_svc, &real_asset_svc).await;

        let asset_svc = failing_asset_svc(|ar| {
            ar.expect_get_by_id().returning(|_| Ok(None));
        });
        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );

        let err = uc.get_account_details(&account_id, None).await.unwrap_err();
        assert!(matches!(err, AccountError::DatabaseError), "got: {err:?}");
    }

    // -------------------------------------------------------------------------
    // FXR multi-currency valuation lift (FXR-030–035/040/041)
    // -------------------------------------------------------------------------
    //
    // Setup:
    //   account currency = EUR
    //   asset currency   = USD
    //   quantity         = 2_000_000  (2.0 units)
    //   average_price    = 100_000_000 (100.00 EUR — cost basis in account currency)
    //   current_price    = 110_000_000 (110.00 USD — asset's market price in USD)
    //   rate (USD→EUR)   = 1_080_000   (1.08 EUR per USD)
    //
    // Conversion formula (FXR-030):
    //   converted_current_price = (110_000_000 as i128 * 1_080_000 as i128 / 1_000_000) as i64
    //                           = 118_800_000 (118.80 EUR)
    //
    // unrealized_pnl (FXR-031):
    //   (converted_current_price - average_price) × quantity / MICRO
    //   = (118_800_000 - 100_000_000) × 2_000_000 / 1_000_000
    //   = 18_800_000 × 2 = 37_600_000 (37.60 EUR)
    //
    // cost_basis (ACD-023):
    //   quantity × average_price / MICRO = 2_000_000 × 100_000_000 / 1_000_000 = 200_000_000
    //
    // performance_pct (FXR-032):
    //   unrealized_pnl × 100_000_000 / cost_basis
    //   = 37_600_000 × 100_000_000 / 200_000_000 = 18_800_000  (18.80%)
    //
    // total_return_pct (FXR-033, dividends_received=0):
    //   (37_600_000 + 0) × 100_000_000 / 200_000_000 = 18_800_000  (18.80%)
    //
    // total_global_value contribution (FXR-041):
    //   quantity × converted_current_price / MICRO
    //   = 2_000_000 × 118_800_000 / 1_000_000 = 237_600_000

    use crate::context::currency::{
        application::service::CurrencyService,
        domain::{MockCurrencyPairRepository, MockCurrencyRateRepository},
    };

    fn make_currency_service_with_fixed_rate(rate_micros: i64) -> Arc<CurrencyService> {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo
            .expect_latest_rate_on_or_before()
            .returning(move |_, _, _| {
                Ok(Some(
                    crate::context::currency::domain::CurrencyRate::from_storage(
                        "USD".to_string(),
                        "EUR".to_string(),
                        "2026-01-01".to_string(),
                        rate_micros,
                        crate::context::currency::domain::CurrencyRateSource::Manual,
                    ),
                ))
            });
        Arc::new(CurrencyService::new(
            Box::new(pair_repo),
            Box::new(rate_repo),
        ))
    }

    fn make_currency_service_with_no_rate() -> Arc<CurrencyService> {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo
            .expect_latest_rate_on_or_before()
            .times(0..)
            .returning(|_, _, _| Ok(None));
        Arc::new(CurrencyService::new(
            Box::new(pair_repo),
            Box::new(rate_repo),
        ))
    }

    // FXR-030/031/032/033/041 — FOREIGN holding WITH a resolvable rate: unrealized_pnl,
    // performance_pct, total_return_pct are Some and use the converted price; the
    // converted market value is included in total_global_value and total_unrealized_pnl.
    #[tokio::test]
    async fn foreign_holding_with_rate_computes_converted_pnl_and_global_value() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        // EUR account
        let account = account_svc
            .create(
                "FX Test".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();

        // USD-denominated asset
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "US Stock".to_string(),
                reference: "USX".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 3,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();

        // 2 units, avg_price = 100.00 EUR (cost basis already in account currency)
        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    2_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();

        // current_price = 110.00 USD
        asset_svc
            .record_asset_price(&asset.id, "2026-01-01", 110.0)
            .await
            .unwrap();

        let currency_svc = make_currency_service_with_fixed_rate(1_080_000);
        let uc = AccountDetailsUseCase::new(account_svc, asset_svc, currency_svc);
        let resp = uc.get_account_details(&account.id, None).await.unwrap();

        assert_eq!(resp.holdings.len(), 1);
        let holding = &resp.holdings[0];

        // FXR-031 — unrealized_pnl = Some(37_600_000)
        assert_eq!(
            holding.unrealized_pnl,
            Some(37_600_000),
            "unrealized_pnl mismatch; got {:?}",
            holding.unrealized_pnl
        );

        // FXR-032 — performance_pct = Some(18_800_000)
        assert_eq!(
            holding.performance_pct,
            Some(18_800_000),
            "performance_pct mismatch; got {:?}",
            holding.performance_pct
        );

        // FXR-033 — total_return_pct = Some(18_800_000) (dividends_received = 0)
        assert_eq!(
            holding.total_return_pct,
            Some(18_800_000),
            "total_return_pct mismatch; got {:?}",
            holding.total_return_pct
        );

        // FXR-041 — total_global_value includes converted market value
        assert_eq!(
            resp.total_global_value, 237_600_000,
            "total_global_value mismatch; got {}",
            resp.total_global_value
        );

        // FXR-040 — total_unrealized_pnl includes converted holding
        assert_eq!(
            resp.total_unrealized_pnl,
            Some(37_600_000),
            "total_unrealized_pnl mismatch; got {:?}",
            resp.total_unrealized_pnl
        );

        // FXR-090 — fx_rate_date carries the date of the rate used for conversion.
        assert_eq!(
            holding.fx_rate_date.as_deref(),
            Some("2026-01-01"),
            "fx_rate_date mismatch; got {:?}",
            holding.fx_rate_date
        );
    }

    // FXR-034 — FOREIGN holding with NO resolvable rate: unrealized_pnl/performance_pct/
    // total_return_pct stay None; market value NOT added to total_global_value.
    #[tokio::test]
    async fn foreign_holding_without_rate_preserves_none_pnl_and_excludes_from_global_value() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create(
                "No FX Rate".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();

        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "US Stock".to_string(),
                reference: "USX".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 3,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();

        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    2_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();

        asset_svc
            .record_asset_price(&asset.id, "2026-01-01", 110.0)
            .await
            .unwrap();

        let currency_svc = make_currency_service_with_no_rate();
        let uc = AccountDetailsUseCase::new(account_svc, asset_svc, currency_svc);
        let resp = uc.get_account_details(&account.id, None).await.unwrap();

        let holding = &resp.holdings[0];
        assert!(
            holding.unrealized_pnl.is_none(),
            "unrealized_pnl must be None when no rate; got {:?}",
            holding.unrealized_pnl
        );
        assert!(
            holding.performance_pct.is_none(),
            "performance_pct must be None when no rate; got {:?}",
            holding.performance_pct
        );
        assert!(
            holding.total_return_pct.is_none(),
            "total_return_pct must be None when no rate; got {:?}",
            holding.total_return_pct
        );
        assert_eq!(
            resp.total_global_value, 0,
            "total_global_value must be 0 when no rate for foreign holding"
        );
        assert!(
            resp.total_unrealized_pnl.is_none(),
            "total_unrealized_pnl must be None when no qualifying holding; got {:?}",
            resp.total_unrealized_pnl
        );
        // FXR-090 — no usable rate → no FX date, so no staleness label is shown.
        assert!(
            holding.fx_rate_date.is_none(),
            "fx_rate_date must be None when no rate; got {:?}",
            holding.fx_rate_date
        );
    }

    // Regression guard — SAME-currency holding behaviour is unchanged after the FXR lift.
    // EUR account, EUR asset, price 110.00, avg_price 100.00, qty 2 → same as pre-FXR.
    #[tokio::test]
    async fn same_currency_holding_behaviour_unchanged_after_fxr_lift() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;

        let account = account_svc
            .create(
                "Same CCY".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();

        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "EUR Bond".to_string(),
                reference: "EURBND".to_string(),
                isin: None,
                class: AssetClass::Bonds,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();

        SqliteHoldingRepository::new(pool.clone())
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    2_000_000,
                    100_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();

        // current_price = 110.00 EUR → unrealized_pnl = (110 - 100) × 2 = 20.00 EUR
        asset_svc
            .record_asset_price(&asset.id, "2026-01-01", 110.0)
            .await
            .unwrap();

        // The currency service mock expects 0 calls for same-currency holdings.
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo
            .expect_latest_rate_on_or_before()
            .times(0)
            .returning(|_, _, _| Ok(None));
        let currency_svc = Arc::new(CurrencyService::new(
            Box::new(pair_repo),
            Box::new(rate_repo),
        ));

        let uc = AccountDetailsUseCase::new(account_svc, asset_svc, currency_svc);
        let resp = uc.get_account_details(&account.id, None).await.unwrap();

        let holding = &resp.holdings[0];
        assert_eq!(holding.unrealized_pnl, Some(20_000_000));
        assert_eq!(holding.performance_pct, Some(10_000_000));
        assert_eq!(holding.total_return_pct, Some(10_000_000));
        assert_eq!(resp.total_global_value, 220_000_000);
        assert_eq!(resp.total_unrealized_pnl, Some(20_000_000));
        // FXR-090 — a same-currency holding has no FX conversion, so no FX date.
        assert!(
            holding.fx_rate_date.is_none(),
            "same-currency must have no fx_rate_date"
        );
    }

    // -------------------------------------------------------------------------
    // As-of view (Some(date)) — read-only reconstruction on a past date
    // -------------------------------------------------------------------------

    async fn make_stock(asset_svc: &AssetService, name: &str, currency: &str) -> String {
        asset_svc
            .create_asset(CreateAssetDTO {
                name: name.to_string(),
                reference: name.to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: currency.to_string(),
                risk_level: 3,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap()
            .id
    }

    // A malformed as-of date is rejected with InvalidDate before any lookup.
    #[tokio::test]
    async fn as_of_malformed_date_returns_invalid_date() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let err = uc
            .get_account_details("any-id", Some("not-a-date"))
            .await
            .unwrap_err();
        assert!(matches!(err, AccountError::InvalidDate), "got: {err:?}");
    }

    // A future as-of date is rejected with DateInFuture.
    #[tokio::test]
    async fn as_of_future_date_returns_date_in_future() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let err = uc
            .get_account_details("any-id", Some("2999-12-31"))
            .await
            .unwrap_err();
        assert!(matches!(err, AccountError::DateInFuture), "got: {err:?}");
    }

    // A holding opened AFTER the as-of date is excluded; one opened before is
    // reconstructed with the as-of quantity + VWAP and priced (carry-forward) as
    // of the date, alongside the cash row at its as-of balance.
    #[tokio::test]
    async fn as_of_reconstructs_holdings_excluding_later_openings() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Acc".to_string(),
                "EUR".to_string(),
                UpdateFrequency::Automatic,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2024-01-01".to_string(), 1_000_000_000, None)
            .await
            .unwrap();

        let early = make_stock(&asset_svc, "Early", "EUR").await;
        let late = make_stock(&asset_svc, "Late", "EUR").await;
        // Early: buy 2 @ 100 on 2024-02-01 (cost 200).
        account_svc
            .buy_holding(
                &account.id,
                early.clone(),
                "2024-02-01".to_string(),
                2_000_000,
                100_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();
        // Late: buy 5 @ 50 on 2024-08-01 — after the as-of date.
        account_svc
            .buy_holding(
                &account.id,
                late.clone(),
                "2024-08-01".to_string(),
                5_000_000,
                50_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();
        // Price for Early: 120 on 2024-03-01 (carry-forward to the as-of date).
        asset_svc
            .record_asset_price(&early, "2024-03-01", 120.0)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc
            .get_account_details(&account.id, Some("2024-06-01"))
            .await
            .unwrap();

        // "Late" excluded; "Early" and cash present.
        assert!(
            resp.holdings.iter().all(|h| h.asset_name != "Late"),
            "Late opened after the date must be excluded"
        );
        let early_row = resp
            .holdings
            .iter()
            .find(|h| h.asset_name == "Early")
            .expect("Early present");
        assert_eq!(early_row.quantity, 2_000_000);
        assert_eq!(early_row.average_price, 100_000_000);
        assert_eq!(early_row.cost_basis, 200_000_000);
        assert_eq!(early_row.current_price, Some(120_000_000));
        assert_eq!(early_row.current_price_date.as_deref(), Some("2024-03-01"));
        // Same-currency: unrealized = (120-100) × 2 = 40; market value (in global) = 240.
        assert_eq!(early_row.unrealized_pnl, Some(40_000_000));

        let cash = resp
            .holdings
            .iter()
            .find(|h| is_cash_asset(&h.asset_id))
            .expect("cash row present");
        // Cash on the date = 1000 deposit − 200 buy = 800.
        assert_eq!(cash.quantity, 800_000_000);

        assert_eq!(resp.total_cost_basis, 200_000_000);
        // Global value = 240 (Early) + 800 (cash) = 1040.
        assert_eq!(resp.total_global_value, 1_040_000_000);
    }

    // A partial sell BEFORE the as-of date lowers quantity but preserves VWAP,
    // and the realized P&L from that sell is reflected as of the date.
    #[tokio::test]
    async fn as_of_partial_sell_before_date_preserves_vwap_and_realizes_pnl() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Acc".to_string(),
                "EUR".to_string(),
                UpdateFrequency::Automatic,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2024-01-01".to_string(), 1_000_000_000, None)
            .await
            .unwrap();
        let stock = make_stock(&asset_svc, "Stock", "EUR").await;
        account_svc
            .buy_holding(
                &account.id,
                stock.clone(),
                "2024-02-01".to_string(),
                4_000_000,
                100_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();
        // Sell 1 @ 150 on 2024-03-01 — before the as-of date. Realized = 150 − 100 = 50.
        account_svc
            .sell_holding(
                &account.id,
                stock.clone(),
                "2024-03-01".to_string(),
                1_000_000,
                150_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc
            .get_account_details(&account.id, Some("2024-06-01"))
            .await
            .unwrap();
        let row = resp
            .holdings
            .iter()
            .find(|h| h.asset_name == "Stock")
            .expect("Stock present");
        assert_eq!(row.quantity, 3_000_000, "4 bought − 1 sold = 3");
        assert_eq!(
            row.average_price, 100_000_000,
            "VWAP preserved across the sell"
        );
        assert_eq!(row.realized_pnl, 50_000_000, "realized = (150 − 100) × 1");
        assert_eq!(resp.total_realized_pnl, 50_000_000);
    }

    // A foreign holding is valued using the FX rate as of the date.
    #[tokio::test]
    async fn as_of_foreign_holding_valued_with_fx_rate() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Acc".to_string(),
                "EUR".to_string(),
                UpdateFrequency::Automatic,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2024-01-01".to_string(), 1_000_000_000, None)
            .await
            .unwrap();
        let stock = make_stock(&asset_svc, "US Co", "USD").await;
        // 2 units, avg 100.00 EUR (cost basis already in account currency).
        account_svc
            .buy_holding(
                &account.id,
                stock.clone(),
                "2024-02-01".to_string(),
                2_000_000,
                100_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();
        asset_svc
            .record_asset_price(&stock, "2024-03-01", 110.0)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_fixed_rate(1_080_000),
        );
        let resp = uc
            .get_account_details(&account.id, Some("2024-06-01"))
            .await
            .unwrap();
        let row = resp
            .holdings
            .iter()
            .find(|h| h.asset_name == "US Co")
            .expect("US Co present");
        // converted_price = 110 × 1.08 = 118.8; unrealized = (118.8 − 100) × 2 = 37.6.
        assert_eq!(row.current_price, Some(110_000_000));
        assert_eq!(row.unrealized_pnl, Some(37_600_000));
        assert_eq!(row.fx_rate_date.as_deref(), Some("2026-01-01"));
        // Global value = converted market value (2 × 118.8 = 237.6) + cash 800.
        assert_eq!(resp.total_global_value, 237_600_000 + 800_000_000);
    }

    // An asset fully sold BEFORE the as-of date appears as a closed holding (qty 0,
    // last_sold_date set), carrying its realized P&L as of the date.
    #[tokio::test]
    async fn as_of_fully_sold_before_date_is_closed_holding() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Acc".to_string(),
                "EUR".to_string(),
                UpdateFrequency::Automatic,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2024-01-01".to_string(), 1_000_000_000, None)
            .await
            .unwrap();
        let stock = make_stock(&asset_svc, "Gone Co", "EUR").await;
        account_svc
            .buy_holding(
                &account.id,
                stock.clone(),
                "2024-02-01".to_string(),
                2_000_000,
                100_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();
        // Sell all 2 @ 130 on 2024-03-01 → realized = (130 − 100) × 2 = 60.
        account_svc
            .sell_holding(
                &account.id,
                stock.clone(),
                "2024-03-01".to_string(),
                2_000_000,
                130_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc
            .get_account_details(&account.id, Some("2024-06-01"))
            .await
            .unwrap();
        assert!(
            resp.holdings.iter().all(|h| h.asset_name != "Gone Co"),
            "fully-sold asset must not be an active holding"
        );
        let closed = resp
            .closed_holdings
            .iter()
            .find(|h| h.asset_name == "Gone Co")
            .expect("Gone Co present in closed holdings");
        assert_eq!(closed.realized_pnl, 60_000_000);
        assert_eq!(closed.last_sold_date, "2024-03-01");
        assert_eq!(resp.total_realized_pnl, 60_000_000);
    }

    // A sell AFTER the as-of date does not appear: the asset reconstructs to its
    // pre-sell quantity and is an active holding with no realized P&L on the date.
    #[tokio::test]
    async fn as_of_excludes_sells_after_the_date() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Acc".to_string(),
                "EUR".to_string(),
                UpdateFrequency::Automatic,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2024-01-01".to_string(), 1_000_000_000, None)
            .await
            .unwrap();
        let stock = make_stock(&asset_svc, "Hold Co", "EUR").await;
        account_svc
            .buy_holding(
                &account.id,
                stock.clone(),
                "2024-02-01".to_string(),
                3_000_000,
                100_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();
        // Sell on 2024-09-01 — after the as-of date, so excluded.
        account_svc
            .sell_holding(
                &account.id,
                stock.clone(),
                "2024-09-01".to_string(),
                1_000_000,
                150_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc
            .get_account_details(&account.id, Some("2024-06-01"))
            .await
            .unwrap();
        let row = resp
            .holdings
            .iter()
            .find(|h| h.asset_name == "Hold Co")
            .expect("Hold Co present");
        assert_eq!(row.quantity, 3_000_000, "pre-sell quantity on the date");
        assert_eq!(
            row.realized_pnl, 0,
            "later sell not realized as of the date"
        );
        assert_eq!(resp.total_realized_pnl, 0);
    }

    // DIV-070/073 — a dividend dated AFTER the as-of date is excluded from both
    // the holding's dividends_received and the account-level total.
    #[tokio::test]
    async fn as_of_excludes_dividends_after_the_date() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "Acc".to_string(),
                "EUR".to_string(),
                UpdateFrequency::Automatic,
            )
            .await
            .unwrap();
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2024-01-01".to_string(), 1_000_000_000, None)
            .await
            .unwrap();
        let stock = make_stock(&asset_svc, "Payer Co", "EUR").await;
        account_svc
            .buy_holding(
                &account.id,
                stock.clone(),
                "2024-02-01".to_string(),
                2_000_000,
                100_000_000,
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();
        // Dividend on 2024-09-01 — after the as-of date, so excluded.
        account_svc
            .record_dividend(
                &account.id,
                stock.clone(),
                "2024-09-01".to_string(),
                3_000_000,
                1_000_000,
                None,
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc
            .get_account_details(&account.id, Some("2024-06-01"))
            .await
            .unwrap();
        let row = resp
            .holdings
            .iter()
            .find(|h| h.asset_name == "Payer Co")
            .expect("Payer Co present");
        assert_eq!(
            row.dividends_received, 0,
            "later dividend not credited as of the date"
        );
        assert_eq!(resp.total_dividends_received, 0);
    }

    // -------------------------------------------------------------------------
    // FEE-052/053/054/072/073 — management_fees aggregation in HoldingDetail
    // and AccountDetailsResponse
    // -------------------------------------------------------------------------

    // FEE-052/053 — HoldingDetail.management_fees is 0 when no management fee
    // transactions have been recorded for the (account, asset) pair.
    #[tokio::test]
    async fn fee_052_holding_detail_management_fees_is_zero_when_no_fees() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "FEE Zero Acct".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "No Fee Stock".to_string(),
                reference: "NFS".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();

        let holding_repo = SqliteHoldingRepository::new(pool.clone());
        holding_repo
            .upsert(
                Holding::new(
                    account.id.clone(),
                    asset.id.clone(),
                    1_000_000,
                    50_000_000,
                    0,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        let row = resp
            .holdings
            .iter()
            .find(|h| h.asset_reference == "NFS")
            .expect("holding present");
        assert_eq!(
            row.management_fees, 0,
            "management_fees must be 0 when no fee transactions recorded"
        );
    }

    // FEE-072/073 — AccountDetailsResponse.total_management_fees is 0 when no
    // management fee transactions have been recorded for the account.
    #[tokio::test]
    async fn fee_072_total_management_fees_is_zero_when_no_fees() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "FEE Total Zero".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        assert_eq!(
            resp.total_management_fees, 0,
            "total_management_fees must be 0 when no fees recorded"
        );
    }

    // FEE-053/054 — HoldingDetail.management_fees equals Σ(qty_removed × price_as_of(date))
    // for each ManagementFee transaction recorded for the (account, asset) pair.
    #[tokio::test]
    async fn fee_053_holding_detail_management_fees_sums_fee_value() {
        // This test will fail until the FEE-051 aggregation logic is implemented.
        // The stub returns management_fees = 0 for all holdings; the assertion
        // below expects a non-zero value — establishing the red baseline.
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "FEE Aggregate".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "Fee Stock".to_string(),
                reference: "FEES".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();

        // Seed: buy 100 units @ 10, then a management fee that removed 1 unit.
        // The fee value = 1 × 10 = 10_000_000 micros.
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2024-01-01".to_string(), 10_000_000_000, None)
            .await
            .unwrap();
        account_svc
            .buy_holding(
                &account.id,
                asset.id.clone(),
                "2024-01-01".to_string(),
                100_000_000, // 100 units
                10_000_000,  // 10.00
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();

        // Record a management fee that removes 1 unit (1_000_000 micro-units).
        account_svc
            .record_management_fee(
                &account.id,
                asset.id.clone(),
                "2024-06-30".to_string(),
                1_000_000, // 1%
                None,
            )
            .await
            .unwrap();

        // FEE-051/054 — value the fee at a recorded carry-forward price (10.00) on its date.
        asset_svc
            .record_asset_price(&asset.id, "2024-06-30", 10.0)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        let row = resp
            .holdings
            .iter()
            .find(|h| h.asset_reference == "FEES")
            .expect("fee stock holding present");

        // FEE-052/053 — management_fees = qty_removed × carry-forward price as of the fee date.
        // qty_removed = floor(100 × 1%) = 1 unit = 1_000_000 micro-units; price = 10.00 = 10_000_000.
        // Expected fee value = 1_000_000 × 10_000_000 / 1_000_000 = 10_000_000 micros.
        assert_eq!(
            row.management_fees, 10_000_000,
            "management_fees must equal qty_removed × price_as_of (FEE-053), got: {}",
            row.management_fees
        );
    }

    // FEE-073 — for a charged asset whose currency differs from the account currency, the
    // Management Fees figure converts each deduction's fee value to the account currency via
    // the FXR rate as-of the deduction date (FXR-042).
    #[tokio::test]
    async fn fee_073_management_fees_converts_foreign_currency_via_fxr() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        // EUR account.
        let account = account_svc
            .create(
                "FEE FX Acct".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        // USD-denominated asset (currency differs from the account currency).
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "US Fee Stock".to_string(),
                reference: "USFEE".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "USD".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();

        // Buy 100 units, then a 1% management fee removing 1 unit.
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2024-01-01".to_string(), 10_000_000_000, None)
            .await
            .unwrap();
        account_svc
            .buy_holding(
                &account.id,
                asset.id.clone(),
                "2024-01-01".to_string(),
                100_000_000, // 100 units
                10_000_000,  // 10.00
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();
        account_svc
            .record_management_fee(
                &account.id,
                asset.id.clone(),
                "2024-06-30".to_string(),
                1_000_000, // 1% → removes 1 unit (1_000_000 micro-units)
                None,
            )
            .await
            .unwrap();
        // FEE-051 — value the fee at the recorded carry-forward price (10.00 USD) on its date.
        asset_svc
            .record_asset_price(&asset.id, "2024-06-30", 10.0)
            .await
            .unwrap();

        // USD→EUR = 1.08 as of the deduction date.
        let currency_svc = make_currency_service_with_fixed_rate(1_080_000);
        let uc = AccountDetailsUseCase::new(account_svc, asset_svc, currency_svc);
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        let row = resp
            .holdings
            .iter()
            .find(|h| h.asset_reference == "USFEE")
            .expect("foreign fee stock holding present");

        // FEE-073 — value_asset_ccy = 1 unit × 10.00 USD = 10_000_000; converted = ×1.08 = 10_800_000 EUR.
        assert_eq!(
            row.management_fees, 10_800_000,
            "management_fees must be converted to account currency via FXR (FEE-073), got: {}",
            row.management_fees
        );
        assert_eq!(
            resp.total_management_fees, 10_800_000,
            "total_management_fees must equal the converted per-holding figure (FEE-073)"
        );
    }

    // FEE-054 — if no market price is recorded on or before a deduction's date,
    // that deduction contributes 0 to the Management Fees figure (valuation
    // degrades to 0), while the quantity removal itself still happens.
    #[tokio::test]
    async fn fee_054_management_fees_is_zero_when_no_price_but_quantity_still_reduced() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "FEE-054".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "No Price Stock".to_string(),
                reference: "NOPX".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();

        // Buy 100 units @ 10, then a 1% management fee removing 1 unit.
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2024-01-01".to_string(), 10_000_000_000, None)
            .await
            .unwrap();
        account_svc
            .buy_holding(
                &account.id,
                asset.id.clone(),
                "2024-01-01".to_string(),
                100_000_000, // 100 units
                10_000_000,  // 10.00
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();
        account_svc
            .record_management_fee(
                &account.id,
                asset.id.clone(),
                "2024-06-30".to_string(),
                1_000_000, // 1% → removes 1 unit (1_000_000 micro-units)
                None,
            )
            .await
            .unwrap();
        // Deliberately record NO asset price → no qualifying price on/before the fee date.

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );
        let resp = uc.get_account_details(&account.id, None).await.unwrap();
        let row = resp
            .holdings
            .iter()
            .find(|h| h.asset_reference == "NOPX")
            .expect("holding present");

        // FEE-054 — no price ⇒ the deduction contributes 0 to Management Fees.
        assert_eq!(
            row.management_fees, 0,
            "management_fees must be 0 when no price is recorded on/before the fee date (FEE-054)"
        );
        assert_eq!(
            resp.total_management_fees, 0,
            "total_management_fees must be 0 when no qualifying price exists (FEE-054)"
        );
        // FEE-054 — the quantity removal itself is unaffected: 100 − 1 = 99 units.
        assert_eq!(
            row.quantity, 99_000_000,
            "the share removal still happens even with no price (FEE-054)"
        );
    }

    // FEE-072 — the as-of Management Fees figure only includes deductions dated on
    // or before the as-of date; a later deduction is excluded.
    #[tokio::test]
    async fn fee_072_management_fees_excludes_deductions_after_as_of() {
        let pool = make_pool().await;
        let (account_svc, asset_svc) = setup(&pool).await;
        let account = account_svc
            .create(
                "FEE-072".to_string(),
                "EUR".to_string(),
                UpdateFrequency::ManualMonth,
            )
            .await
            .unwrap();
        let asset = asset_svc
            .create_asset(CreateAssetDTO {
                name: "As-Of Fee Stock".to_string(),
                reference: "AOFEE".to_string(),
                isin: None,
                class: AssetClass::Stocks,
                currency: "EUR".to_string(),
                risk_level: 1,
                category_id: SYSTEM_CATEGORY_ID.to_string(),
                exchange: None,
            })
            .await
            .unwrap();

        // Buy 100 units @ 10.
        asset_svc.seed_cash_asset("EUR").await.unwrap();
        account_svc
            .record_deposit(&account.id, "2024-01-01".to_string(), 10_000_000_000, None)
            .await
            .unwrap();
        account_svc
            .buy_holding(
                &account.id,
                asset.id.clone(),
                "2024-01-01".to_string(),
                100_000_000, // 100 units
                10_000_000,  // 10.00
                1_000_000,
                0,
                None,
            )
            .await
            .unwrap();

        // First fee on 2024-06-30 — removes floor(100 × 1%) = 1 unit.
        account_svc
            .record_management_fee(
                &account.id,
                asset.id.clone(),
                "2024-06-30".to_string(),
                1_000_000,
                None,
            )
            .await
            .unwrap();
        // Second fee on 2024-09-30 — removes floor(99 × 1%) = 0.99 unit = 990_000 micro-units.
        account_svc
            .record_management_fee(
                &account.id,
                asset.id.clone(),
                "2024-09-30".to_string(),
                1_000_000,
                None,
            )
            .await
            .unwrap();
        // Value both deductions at the recorded carry-forward price (10.00) on their dates.
        asset_svc
            .record_asset_price(&asset.id, "2024-06-30", 10.0)
            .await
            .unwrap();
        asset_svc
            .record_asset_price(&asset.id, "2024-09-30", 10.0)
            .await
            .unwrap();

        let uc = AccountDetailsUseCase::new(
            account_svc,
            asset_svc,
            make_currency_service_with_no_rate(),
        );

        // As-of 2024-07-01: only the 2024-06-30 deduction counts.
        // Its fee value = 1 unit × 10.00 = 10_000_000 micros.
        let as_of_resp = uc
            .get_account_details(&account.id, Some("2024-07-01"))
            .await
            .unwrap();
        let as_of_row = as_of_resp
            .holdings
            .iter()
            .find(|h| h.asset_reference == "AOFEE")
            .expect("holding present in as-of view");
        assert_eq!(
            as_of_row.management_fees, 10_000_000,
            "as-of Management Fees must include only deductions dated ≤ the as-of date (FEE-072)"
        );

        // Live view (no cutoff): BOTH deductions count.
        // 1 unit × 10.00 + 0.99 unit × 10.00 = 10_000_000 + 9_900_000 = 19_900_000.
        let live_resp = uc.get_account_details(&account.id, None).await.unwrap();
        let live_row = live_resp
            .holdings
            .iter()
            .find(|h| h.asset_reference == "AOFEE")
            .expect("holding present in live view");
        assert_eq!(
            live_row.management_fees, 19_900_000,
            "without a cutoff both deductions count — confirming the as-of filter excluded the later one (FEE-072)"
        );
    }
}
