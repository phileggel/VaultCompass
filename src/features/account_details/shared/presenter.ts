import type { AccountDetailsResponse, HoldingDetail } from "@/bindings";
import { microToDecimal } from "@/lib/microUnits";

export interface HoldingRowViewModel {
  assetId: string;
  assetName: string;
  assetReference: string;
  quantity: string;
  averagePrice: string;
  costBasis: string;
}

export interface AccountSummaryViewModel {
  accountName: string;
  totalCostBasis: string;
  holdingCount: number;
  isEmpty: boolean;
  isAllClosed: boolean;
}

export function toHoldingRow(detail: HoldingDetail): HoldingRowViewModel {
  return {
    assetId: detail.asset_id,
    assetName: detail.asset_name,
    assetReference: detail.asset_reference,
    quantity: microToDecimal(detail.quantity, 6),
    averagePrice: microToDecimal(detail.average_price, 2),
    costBasis: microToDecimal(detail.cost_basis, 2),
  };
}

export function toAccountSummary(response: AccountDetailsResponse): AccountSummaryViewModel {
  return {
    accountName: response.account_name,
    totalCostBasis: microToDecimal(response.total_cost_basis, 2),
    holdingCount: response.total_holding_count,
    isEmpty: response.total_holding_count === 0,
    isAllClosed: response.total_holding_count > 0 && response.holdings.length === 0,
  };
}
