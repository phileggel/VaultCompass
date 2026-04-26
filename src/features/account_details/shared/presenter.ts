import type { AccountDetailsResponse, ClosedHoldingDetail, HoldingDetail } from "@/bindings";
import { microToDecimal } from "@/lib/microUnits";

export interface HoldingRowViewModel {
  assetId: string;
  assetName: string;
  assetReference: string;
  quantity: string;
  /** Holding quantity in raw micro-units — used to pass to SellTransactionModal (SEL-010). */
  quantityMicro: number;
  averagePrice: string;
  costBasis: string;
  /** Formatted realized P&L string (2 decimal places, SEL-042). */
  realizedPnl: string;
  /** Raw realized P&L in micro-units — used for sign-based color styling (SEL-043). */
  realizedPnlRaw: number;
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
}

export function toHoldingRow(detail: HoldingDetail): HoldingRowViewModel {
  return {
    assetId: detail.asset_id,
    assetName: detail.asset_name,
    assetReference: detail.asset_reference,
    quantity: microToDecimal(detail.quantity, 6),
    quantityMicro: detail.quantity,
    averagePrice: microToDecimal(detail.average_price, 2),
    costBasis: microToDecimal(detail.cost_basis, 2),
    realizedPnl: microToDecimal(detail.realized_pnl, 2),
    realizedPnlRaw: detail.realized_pnl,
  };
}

export function toClosedHoldingRow(detail: ClosedHoldingDetail): ClosedHoldingRowViewModel {
  return {
    assetId: detail.asset_id,
    assetName: detail.asset_name,
    assetReference: detail.asset_reference,
    realizedPnl: microToDecimal(detail.realized_pnl, 2),
    realizedPnlRaw: detail.realized_pnl,
    lastSoldDate: detail.last_sold_date,
  };
}

export function toAccountSummary(response: AccountDetailsResponse): AccountSummaryViewModel {
  return {
    accountName: response.account_name,
    totalCostBasis: microToDecimal(response.total_cost_basis, 2),
    totalRealizedPnl: microToDecimal(response.total_realized_pnl, 2),
    totalRealizedPnlRaw: response.total_realized_pnl,
    holdingCount: response.total_holding_count,
    isEmpty: response.total_holding_count === 0,
    isAllClosed: response.total_holding_count > 0 && response.holdings.length === 0,
    hasClosedHoldings: response.closed_holdings.length > 0,
  };
}
