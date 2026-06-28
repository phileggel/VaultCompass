import type {
  AccountDetailsResponse,
  AccountError,
  AssetError,
  AssetPriceSource,
  ClosedHoldingDetail,
  DividendError,
  FreeSharesError,
  HoldingDetail,
} from "@/bindings";
import {
  microToFormatted,
  microToFormattedPrice,
  microToFormattedQuantity,
} from "@/lib/microUnits";
import type { I18nMessage } from "@/ui/format/i18n";
import { formatStalenessLabel, type StalenessLabel } from "@/ui/format/staleness";
import type { PriceableAsset } from "./types";

/**
 * F27 — Maps any asset-price mutation error (record / update / delete /
 * get prices) to an i18n key + optional interpolation vars. Pure function:
 * no React, no useTranslation. `err` is the BC-wide `AssetError` union, so the
 * switch lists the codes these price commands can raise and falls back to a
 * generic key for any other.
 */
export function assetPriceMutationErrorToI18n(err: AssetError): I18nMessage {
  switch (err.code) {
    case "InvalidDateFormat":
      return { key: "error.InvalidDateFormat", vars: { date: err.date } };
    case "AssetNotFound":
    case "Archived":
    case "DatabaseError":
    case "PriceNotFound":
    case "NotPositive":
    case "NonFinite":
    case "DateInFuture":
      return { key: `error.${err.code}` };
    default:
      return { key: "error.Unknown" };
  }
}

/**
 * F27 — Maps the AssetError variants reachable by the price-refresh lock
 * toggle commands (MKT-156) to an i18n key. The reachable codes per the asset
 * contract are `AssetNotFound`, `CashAssetNotEditable`, `DatabaseError`; other
 * `AssetError` variants cannot be produced by `block_asset_price_refresh` /
 * `unblock_asset_price_refresh` so they map to a generic key.
 */
export function priceRefreshLockErrorToI18n(err: AssetError): I18nMessage {
  switch (err.code) {
    case "AssetNotFound":
    case "CashAssetNotEditable":
    case "DatabaseError":
      return { key: `error.${err.code}` };
    default:
      return { key: "error.Unknown" };
  }
}

/**
 * F27 — Maps the `record_dividend` error surface to an i18n key. `DividendError`
 * is an untagged union of two tagged leaves (`AccountError` |
 * `DividendTask`) whose combined `code` set is wider than what the
 * command can actually raise, so this switch lists the reachable codes (per the
 * account contract) and falls back to a generic key for any other — mirroring
 * `priceRefreshLockErrorToI18n` rather than the `never`-exhaustive style used
 * for narrow single-type errors.
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

/**
 * F27 — Maps the free-shares error surfaces to an i18n key (FSD-021/011/040).
 * Covers both the create path (`FreeSharesError`) and the edit path
 * (`AccountError`, via `correct_transaction`). Every reachable code resolves to
 * `error.{code}`; the two holding-internal codes that never reach the wire
 * (`NegativeQuantity`, `NegativeAveragePrice`) have no i18n key, so they fall
 * back to `error.Unknown`.
 */
export function freeSharesErrorToI18n(err: FreeSharesError | AccountError): I18nMessage {
  if (err.code === "NegativeQuantity" || err.code === "NegativeAveragePrice") {
    return { key: "error.Unknown" };
  }
  return { key: `error.${err.code}` };
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
  /** Market value = current_price × quantity, asset currency; "—" when no price (MKT-143). */
  currentValue: string;
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
  /** Formatted dividends received over the life of the position (2 decimals, DIV-073). */
  dividendsReceived: string;
  /** Raw dividends received in micro-units (DIV-073). */
  dividendsReceivedRaw: number;
  /** Formatted total revenues = realized P&L + dividends (2 decimals). */
  totalRevenues: string;
  /** Raw total revenues in micro-units — used for sign-based color styling. */
  totalRevenuesRaw: number;
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

/**
 * MKT-140 — i18n descriptor for the staleness of the current price, via the
 * shared `formatStalenessLabel` with the price-specific `mkt.staleness_*` keys.
 */
export function formatStaleness(
  currentPriceDate: string | null,
  today: Date,
): StalenessLabel | null {
  return formatStalenessLabel(currentPriceDate, today, {
    today: "mkt.staleness_today",
    daysAgo: "mkt.staleness_days_ago",
  });
}

/**
 * FXR-090 — Staleness label for the FX rate used to value a foreign holding in
 * the account currency. `null` when no FX rate date is present (same-currency,
 * no usable rate, or cash). Emits the `currency.rate_staleness_*` i18n keys
 * shared with the currency feature (key strings only — no cross-feature import).
 */
export function formatFxStaleness(fxRateDate: string | null, today: Date): StalenessLabel | null {
  return formatStalenessLabel(fxRateDate, today, {
    today: "currency.rate_staleness_today",
    daysAgo: "currency.rate_staleness_days_old",
  });
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
    case "YahooFinance":
      return "mkt.source_yahoo";
  }
}

/**
 * MKT-011 — the active non-cash holdings selectable when recording a price, in
 * the shape the price modal's asset combobox consumes.
 */
export function toPriceableAssets(holdings: HoldingRowViewModel[]): PriceableAsset[] {
  return holdings
    .filter((h) => h.canEnterPrice)
    .map((h) => ({
      assetId: h.assetId,
      assetName: h.assetName,
      assetCurrency: h.assetCurrency,
    }));
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
      currentValue: "",
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
    // MKT-143 — market value = current_price × quantity in the asset's native
    // currency; "—" when no price has been recorded. Dividing the price out of
    // micros before multiplying keeps the intermediate below MAX_SAFE_INTEGER
    // (vs price_micros × qty_micros); the sub-micro float drift is absorbed by
    // microToFormatted's 2-decimal rounding.
    currentValue:
      detail.current_price !== null
        ? microToFormatted((detail.current_price / 1_000_000) * detail.quantity, 2)
        : DASH,
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
    dividendsReceived: microToFormatted(detail.dividends_received, 2),
    dividendsReceivedRaw: detail.dividends_received,
    totalRevenues: microToFormatted(detail.realized_pnl + detail.dividends_received, 2),
    totalRevenuesRaw: detail.realized_pnl + detail.dividends_received,
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
    // CSH-098 — cash is excluded from both counts. With eager cash, every account
    // has a Cash Holding (total_holding_count >= 1), so these key off non-cash active
    // holdings + closed holdings only, never the raw count.
    isEmpty: nonCashActive.length === 0 && response.closed_holdings.length === 0,
    isAllClosed: nonCashActive.length === 0 && response.closed_holdings.length > 0,
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
