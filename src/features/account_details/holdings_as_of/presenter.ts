import type { AccountError, HoldingAsOf, HoldingsAsOfResponse } from "@/bindings";
import {
  microToFormatted,
  microToFormattedPrice,
  microToFormattedQuantity,
} from "@/lib/microUnits";
import type { I18nMessage } from "@/ui/format/i18n";
import { isCashAsset } from "../shared/presenter";

const DASH = "—";

/**
 * F27 — Maps the `get_account_holdings_as_of` error surface (`AccountError`) to
 * an i18n key. Lists the codes this read-only command can raise (date validation
 * + account lookup + infra) and falls back to a generic key for any other.
 */
export function holdingsAsOfErrorToI18n(err: AccountError): I18nMessage {
  switch (err.code) {
    case "InvalidDate":
    case "DateInFuture":
    case "DateTooOld":
    case "AccountNotFound":
    case "DatabaseError":
      return { key: `error.${err.code}` };
    default:
      return { key: "error.Unknown" };
  }
}

/** Display-ready row for the holdings-as-of table. */
export interface HoldingAsOfRowViewModel {
  assetId: string;
  assetName: string;
  /** Formatted quantity (cash: 2 decimals; non-cash: quantity precision). */
  quantity: string;
  /** Formatted average cost in account currency, or "—" for the cash row. */
  averageCost: string;
  /** Formatted price in the asset's native currency, or "—" when none. */
  price: string;
  /** ISO date of the price observation, or null when no price. */
  priceDate: string | null;
  /** Formatted market value in account currency, or "—" when not computable. */
  marketValue: string;
  /** Formatted unrealized P&L, or "—" when not computable. */
  unrealizedPnl: string;
  /** Raw unrealized P&L in micro-units, or null — used for sign-based color styling. */
  unrealizedPnlRaw: number | null;
  /** True when this row is the system Cash Holding. */
  isCash: boolean;
}

/** Formatted account-currency totals for the holdings-as-of footer. */
export interface HoldingsAsOfTotals {
  totalCostBasis: string;
  totalMarketValue: string;
}

/** Maps the response totals (micro-units) to display strings. */
export function toHoldingsAsOfTotals(data: HoldingsAsOfResponse): HoldingsAsOfTotals {
  return {
    totalCostBasis: microToFormatted(data.total_cost_basis, 2),
    totalMarketValue: microToFormatted(data.total_market_value, 2),
  };
}

/** Maps a `HoldingAsOf` DTO row to display strings. */
export function toHoldingAsOfRow(detail: HoldingAsOf): HoldingAsOfRowViewModel {
  const isCash = isCashAsset(detail.asset_id);
  return {
    assetId: detail.asset_id,
    assetName: detail.asset_name,
    quantity: isCash
      ? microToFormatted(detail.quantity, 2)
      : microToFormattedQuantity(detail.quantity),
    averageCost: isCash ? DASH : microToFormattedPrice(detail.average_price),
    price: detail.price !== null ? microToFormattedPrice(detail.price) : DASH,
    priceDate: detail.price_date,
    marketValue: detail.market_value !== null ? microToFormatted(detail.market_value, 2) : DASH,
    unrealizedPnl:
      detail.unrealized_pnl !== null ? microToFormatted(detail.unrealized_pnl, 2) : DASH,
    unrealizedPnlRaw: detail.unrealized_pnl,
    isCash,
  };
}
