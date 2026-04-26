import type { AccountDetailsResponse, ClosedHoldingDetail, HoldingDetail } from "@/bindings";
import { microToFormatted } from "@/lib/microUnits";

const DASH = "—";

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
  /** Formatted current market price (2 decimal places) or "—" when no price recorded (MKT-030). */
  currentPrice: string;
  /** ISO date of the price observation, or null when no price recorded (MKT-030). */
  currentPriceDate: string | null;
  /** Formatted unrealized P&L (2 decimal places) or "—" when not computable (MKT-032/034). */
  unrealizedPnl: string;
  /** Raw unrealized P&L in micro-units, or null when not computable (MKT-034). */
  unrealizedPnlRaw: number | null;
  /** Formatted performance % (e.g. "5.25%") or "—" when not computable (MKT-032/035). */
  performancePct: string;
}

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
}

export function toHoldingRow(detail: HoldingDetail): HoldingRowViewModel {
  return {
    assetId: detail.asset_id,
    assetName: detail.asset_name,
    assetReference: detail.asset_reference,
    assetCurrency: detail.asset_currency,
    quantity: microToFormatted(detail.quantity, 6),
    quantityMicro: detail.quantity,
    averagePrice: microToFormatted(detail.average_price, 2),
    costBasis: microToFormatted(detail.cost_basis, 2),
    realizedPnl: microToFormatted(detail.realized_pnl, 2),
    realizedPnlRaw: detail.realized_pnl,
    canEnterPrice: true,
    currentPrice: detail.current_price !== null ? microToFormatted(detail.current_price, 2) : DASH,
    currentPriceDate: detail.current_price_date,
    unrealizedPnl:
      detail.unrealized_pnl !== null ? microToFormatted(detail.unrealized_pnl, 2) : DASH,
    unrealizedPnlRaw: detail.unrealized_pnl,
    performancePct:
      detail.performance_pct !== null ? `${microToFormatted(detail.performance_pct, 2)}%` : DASH,
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
  return {
    accountName: response.account_name,
    totalCostBasis: microToFormatted(response.total_cost_basis, 2),
    totalRealizedPnl: microToFormatted(response.total_realized_pnl, 2),
    totalRealizedPnlRaw: response.total_realized_pnl,
    holdingCount: response.total_holding_count,
    isEmpty: response.total_holding_count === 0,
    isAllClosed: response.total_holding_count > 0 && response.holdings.length === 0,
    hasClosedHoldings: response.closed_holdings.length > 0,
    totalUnrealizedPnl:
      response.total_unrealized_pnl !== null
        ? microToFormatted(response.total_unrealized_pnl, 2)
        : DASH,
  };
}
