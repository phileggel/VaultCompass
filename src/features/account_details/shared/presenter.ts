import type {
  AccountDetailsResponse,
  AssetCrudError,
  AssetPriceError,
  AssetPriceSource,
  ClosedHoldingDetail,
  DividendError,
  HoldingDetail,
} from "@/bindings";
import {
  microToFormatted,
  microToFormattedPrice,
  microToFormattedQuantity,
} from "@/lib/microUnits";
import type { I18nMessage } from "@/ui/format/i18n";

/**
 * F27 — Maps any asset-price mutation error (record / update / delete /
 * get prices) to an i18n key + optional interpolation vars. Pure function:
 * no React, no useTranslation. Exhaustive switch on `code`; TypeScript
 * catches new variants at compile time.
 */
export function assetPriceMutationErrorToI18n(err: AssetPriceError): I18nMessage {
  switch (err.code) {
    case "InvalidDateFormat":
      return { key: "error.InvalidDateFormat", vars: { date: err.date } };
    case "NotFound":
    case "Archived":
    case "DatabaseError":
    case "PriceNotFound":
    case "NotPositive":
    case "NonFinite":
    case "DateInFuture":
      return { key: `error.${err.code}` };
    default: {
      const _exhaustive: never = err;
      return _exhaustive;
    }
  }
}

/**
 * F27 — Maps the AssetCrudError variants reachable by the price-refresh lock
 * toggle commands (MKT-156) to an i18n key. Narrowed exhaustive switch over
 * the three reachable codes per the asset contract: `NotFound`,
 * `CashAssetNotEditable`, `DatabaseError`. Other AssetCrudError variants
 * cannot be produced by `block_asset_price_refresh` / `unblock_asset_price_refresh`
 * so they map to a generic key without triggering the exhaustiveness check.
 */
export function priceRefreshLockErrorToI18n(err: AssetCrudError): I18nMessage {
  switch (err.code) {
    case "NotFound":
    case "CashAssetNotEditable":
    case "DatabaseError":
      return { key: `error.${err.code}` };
    default:
      return { key: "error.Unknown" };
  }
}

/**
 * F27 — Maps the `record_dividend` error surface to an i18n key. `DividendError`
 * is an untagged union of three tagged leaves (`AccountApplicationError` |
 * `DividendApplicationError` | `TransactionDomainError`) whose combined `code`
 * set is wider than what the command can actually raise, so this switch lists
 * the reachable codes (per the account contract) and falls back to a generic
 * key for any other — mirroring `priceRefreshLockErrorToI18n` rather than the
 * `never`-exhaustive style used for narrow single-type errors.
 */
export function dividendErrorToI18n(err: DividendError): I18nMessage {
  switch (err.code) {
    case "AccountNotFound":
    case "DatabaseError":
    case "AssetNotFound":
    case "AssetNotHeld":
    case "DividendOnCashAsset":
    case "AmountNotPositive":
    case "ExchangeRateNotPositive":
    case "DateInFuture":
    case "DateTooOld":
    case "InvalidDate":
      return { key: `error.${err.code}` };
    default:
      return { key: "error.Unknown" };
  }
}

const DASH = "—";
const CASH_ASSET_PREFIX = "system-cash-";

/** True when the asset_id is the deterministic system Cash Asset ID (CSH-014). */
export function isCashAsset(assetId: string): boolean {
  return assetId.startsWith(CASH_ASSET_PREFIX);
}

export interface HoldingRowViewModel {
  assetId: string;
  assetName: string;
  assetReference: string;
  /** ISO 4217 currency code of the asset (MKT-023). */
  assetCurrency: string;
  quantity: string;
  /** Holding quantity in raw micro-units — used to pass to SellTransactionModal (SEL-010). */
  quantityMicro: number;
  averagePrice: string;
  costBasis: string;
  /** Formatted realized P&L string (2 decimal places, SEL-042). */
  realizedPnl: string;
  /** Raw realized P&L in micro-units — used for sign-based color styling (SEL-043). */
  realizedPnlRaw: number;
  /** Always true — active holding rows can trigger the price entry modal (MKT-010). */
  canEnterPrice: boolean;
  /** Current price state — formatted price or typed diagnostic (MKT-030, MKT-032). */
  currentPrice: CurrentPriceState;
  /** ISO date of the price observation, or null when no price recorded (MKT-030). */
  currentPriceDate: string | null;
  /** Formatted unrealized P&L (2 decimal places) or "—" when not computable (MKT-032/034). */
  unrealizedPnl: string;
  /** Raw unrealized P&L in micro-units, or null when not computable (MKT-034). */
  unrealizedPnlRaw: number | null;
  /** Formatted performance % (e.g. "5.25%") or "—" when not computable (MKT-032/035). */
  performancePct: string;
  /** Formatted cumulative dividends received for this holding, account currency (DIV-072). Always shown ("0,00" when none). */
  dividendsReceived: string;
  /** Formatted total return % (price + dividends) or "—" when not computable (DIV-071/072). */
  totalReturnPct: string;
  /** Raw total return % in micro-units, or null when not computable — used for sign-based color styling (DIV-072). */
  totalReturnPctRaw: number | null;
  /** True when this row is the system Cash Holding (CSH-090). Drives the cash variant in HoldingRow. */
  isCash: boolean;
  /** Staleness label for the current price (MKT-140); null when no price is recorded. */
  staleness: StalenessLabel | null;
  /** i18n key for the price source badge (MKT-142), or null when no price is recorded. */
  sourceLabel: string | null;
  /** FX-rate staleness label (FXR-090); null/absent unless a converted value is shown. */
  fxStaleness?: StalenessLabel | null;
}

/** i18n key + optional interpolation params for the price staleness label (MKT-140). */
export type StalenessLabel = { key: string; params?: { days: number } };

/**
 * MKT-032 — Discriminated state for the Current Price cell. When `current_price`
 * is `None`, the cell renders a typed diagnostic so the user can see _why_ a
 * price is unavailable (instead of an unannotated "—").
 *
 * - `present`: a recorded price; `formatted` holds the displayable string.
 * - `missing_ticker`: `asset_reference` is empty — adding a ticker would unlock fetch.
 * - `no_price_available`: reference is non-empty but no price recorded (provider
 *   returned N/D, or no fetch has run yet — intentionally merged per spec).
 *
 * Cash-row variant uses `{ kind: "present", formatted: "" }`; the HoldingRow
 * cash branch never reads this field (renders empty cells).
 */
export type CurrentPriceState =
  | { kind: "present"; formatted: string }
  | { kind: "missing_ticker" }
  | { kind: "no_price_available" };

export interface ClosedHoldingRowViewModel {
  assetId: string;
  assetName: string;
  assetReference: string;
  /** Formatted realized P&L string (2 decimal places, ACD-049). */
  realizedPnl: string;
  /** Raw realized P&L in micro-units — used for sign-based color styling (ACD-049). */
  realizedPnlRaw: number;
  /** ISO date of last sell "YYYY-MM-DD" (ACD-049). */
  lastSoldDate: string;
}

export interface AccountSummaryViewModel {
  accountName: string;
  totalCostBasis: string;
  /** Formatted total realized P&L string (2 decimal places, SEL-042). */
  totalRealizedPnl: string;
  /** Raw total realized P&L in micro-units — used for sign-based color styling (SEL-043). */
  totalRealizedPnlRaw: number;
  holdingCount: number;
  isEmpty: boolean;
  isAllClosed: boolean;
  /** True when there is at least one closed holding to display (ACD-048). */
  hasClosedHoldings: boolean;
  /** Formatted total unrealized P&L (2 decimals) or "—" when no qualifying holdings (MKT-041). */
  totalUnrealizedPnl: string;
  /** Formatted total Global Value (cash + priced holdings, 2 decimals, CSH-094). */
  totalGlobalValue: string;
  /** Raw total Global Value in micro-units (CSH-094). */
  totalGlobalValueRaw: number;
  /** Formatted cumulative dividends received across the account, account currency (DIV-073). */
  totalDividendsReceived: string;
  /** Raw cumulative dividends received in micro-units — used to gate header display (DIV-073). */
  totalDividendsReceivedRaw: number;
  /** True when the account currently holds a non-zero cash balance (CSH-019/095). */
  hasCashHolding: boolean;
}

/** Whole-day delta between an ISO date and `today`; null for a null/unparseable date. */
function computeDayDelta(isoDate: string | null, today: Date): number | null {
  if (isoDate === null) return null;
  const observed = new Date(`${isoDate}T00:00:00`);
  if (Number.isNaN(observed.getTime())) return null;
  const startOfToday = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  const millisPerDay = 24 * 60 * 60 * 1000;
  return Math.floor((startOfToday.getTime() - observed.getTime()) / millisPerDay);
}

/**
 * MKT-140 — Returns an i18n descriptor for the staleness of the current price.
 * `null` when no date is recorded; `{ key: "mkt.staleness_today" }` when the price is from today;
 * `{ key: "mkt.staleness_days_ago", params: { days } }` otherwise.
 *
 * The caller renders via `t(label.key, label.params)`.
 */
export function formatStaleness(
  currentPriceDate: string | null,
  today: Date,
): StalenessLabel | null {
  const dayDelta = computeDayDelta(currentPriceDate, today);
  if (dayDelta === null) return null;
  if (dayDelta <= 0) return { key: "mkt.staleness_today" };
  return { key: "mkt.staleness_days_ago", params: { days: dayDelta } };
}

/**
 * FXR-090 — Staleness label for the FX rate used to value a foreign holding in
 * the account currency. `null` when no FX rate date is present (same-currency,
 * no usable rate, or cash). Emits the `currency.rate_staleness_*` i18n keys
 * shared with the currency feature (key strings only — no cross-feature import).
 */
export function formatFxStaleness(fxRateDate: string | null, today: Date): StalenessLabel | null {
  const dayDelta = computeDayDelta(fxRateDate, today);
  if (dayDelta === null) return null;
  if (dayDelta <= 0) return { key: "currency.rate_staleness_today" };
  return { key: "currency.rate_staleness_days_old", params: { days: dayDelta } };
}

/**
 * MKT-032 — Derives the Current Price cell state from a holding's data.
 * Pure function: no i18n; price formatting delegates to `microToFormattedPrice`.
 */
export function derivePriceState(detail: HoldingDetail): CurrentPriceState {
  if (detail.current_price !== null) {
    return { kind: "present", formatted: microToFormattedPrice(detail.current_price) };
  }
  if (detail.asset_reference.trim() === "") {
    return { kind: "missing_ticker" };
  }
  return { kind: "no_price_available" };
}

/**
 * MKT-141 / MKT-142 — Maps an AssetPriceSource to its i18n label key, or null when source is null.
 */
export function formatSource(source: AssetPriceSource | null): string | null {
  if (source === null) return null;
  switch (source) {
    case "Manual":
      return "mkt.source_manual";
    case "Stooq":
      return "mkt.source_stooq";
  }
}

export function toHoldingRow(detail: HoldingDetail): HoldingRowViewModel {
  const isCash = isCashAsset(detail.asset_id);
  if (isCash) {
    // Cash row variant (CSH-090/091): no cost basis, average price, realized PnL or
    // market-price columns — those cells render blank in the table. Quantity is the
    // running cash balance, formatted to 2 decimals like an amount.
    return {
      assetId: detail.asset_id,
      assetName: detail.asset_name,
      assetReference: detail.asset_reference,
      assetCurrency: detail.asset_currency,
      quantity: microToFormatted(detail.quantity, 2),
      quantityMicro: detail.quantity,
      averagePrice: "",
      costBasis: "",
      realizedPnl: "",
      realizedPnlRaw: 0,
      canEnterPrice: false,
      currentPrice: { kind: "present", formatted: "" },
      currentPriceDate: null,
      unrealizedPnl: "",
      unrealizedPnlRaw: null,
      performancePct: "",
      dividendsReceived: "",
      totalReturnPct: "",
      totalReturnPctRaw: null,
      isCash: true,
      staleness: null,
      sourceLabel: null,
      fxStaleness: null,
    };
  }
  return {
    assetId: detail.asset_id,
    assetName: detail.asset_name,
    assetReference: detail.asset_reference,
    assetCurrency: detail.asset_currency,
    quantity: microToFormattedQuantity(detail.quantity),
    quantityMicro: detail.quantity,
    averagePrice: microToFormattedPrice(detail.average_price),
    costBasis: microToFormatted(detail.cost_basis, 2),
    realizedPnl: microToFormatted(detail.realized_pnl, 2),
    realizedPnlRaw: detail.realized_pnl,
    canEnterPrice: true,
    currentPrice: derivePriceState(detail),
    currentPriceDate: detail.current_price_date,
    unrealizedPnl:
      detail.unrealized_pnl !== null ? microToFormatted(detail.unrealized_pnl, 2) : DASH,
    unrealizedPnlRaw: detail.unrealized_pnl,
    performancePct:
      detail.performance_pct !== null ? `${microToFormatted(detail.performance_pct, 2)}%` : DASH,
    dividendsReceived: microToFormatted(detail.dividends_received, 2),
    totalReturnPct:
      detail.total_return_pct !== null ? `${microToFormatted(detail.total_return_pct, 2)}%` : DASH,
    totalReturnPctRaw: detail.total_return_pct,
    isCash: false,
    staleness: formatStaleness(detail.current_price_date, new Date()),
    sourceLabel: formatSource(detail.current_price_source),
    // FXR-090 — staleness of the FX rate used to value this holding; null for
    // same-currency / no-rate holdings (fx_rate_date is None).
    fxStaleness: formatFxStaleness(detail.fx_rate_date, new Date()),
  };
}

export function toClosedHoldingRow(detail: ClosedHoldingDetail): ClosedHoldingRowViewModel {
  return {
    assetId: detail.asset_id,
    assetName: detail.asset_name,
    assetReference: detail.asset_reference,
    realizedPnl: microToFormatted(detail.realized_pnl, 2),
    realizedPnlRaw: detail.realized_pnl,
    lastSoldDate: detail.last_sold_date,
  };
}

export function toAccountSummary(response: AccountDetailsResponse): AccountSummaryViewModel {
  // CSH-098 — exclude the cash holding from the active count used for empty/all-closed
  // gating, otherwise an account with only a cash holding would never trigger the
  // "no positions yet" empty state nor the closed-only message.
  const nonCashActive = response.holdings.filter((h) => !isCashAsset(h.asset_id));
  const hasCashHolding = response.holdings.some((h) => isCashAsset(h.asset_id) && h.quantity > 0);
  return {
    accountName: response.account_name,
    totalCostBasis: microToFormatted(response.total_cost_basis, 2),
    totalRealizedPnl: microToFormatted(response.total_realized_pnl, 2),
    totalRealizedPnlRaw: response.total_realized_pnl,
    holdingCount: response.total_holding_count,
    isEmpty:
      response.total_holding_count === 0 ||
      (nonCashActive.length === 0 && response.closed_holdings.length === 0),
    isAllClosed: response.total_holding_count > 0 && nonCashActive.length === 0,
    hasClosedHoldings: response.closed_holdings.length > 0,
    totalUnrealizedPnl:
      response.total_unrealized_pnl !== null
        ? microToFormatted(response.total_unrealized_pnl, 2)
        : DASH,
    totalGlobalValue: microToFormatted(response.total_global_value, 2),
    totalGlobalValueRaw: response.total_global_value,
    totalDividendsReceived: microToFormatted(response.total_dividends_received, 2),
    totalDividendsReceivedRaw: response.total_dividends_received,
    hasCashHolding,
  };
}
