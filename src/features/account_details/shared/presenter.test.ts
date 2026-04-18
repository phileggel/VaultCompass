import { describe, expect, it } from "vitest";
import type { AccountDetailsResponse, HoldingDetail } from "@/bindings";
import { toAccountSummary, toHoldingRow } from "./presenter";

const makeHolding = (overrides: Partial<HoldingDetail> = {}): HoldingDetail => ({
  asset_id: "asset-1",
  asset_name: "Apple Inc",
  asset_reference: "AAPL",
  quantity: 2_000_000,
  average_price: 100_000_000,
  cost_basis: 200_000_000,
  ...overrides,
});

const makeResponse = (overrides: Partial<AccountDetailsResponse> = {}): AccountDetailsResponse => ({
  account_name: "My Portfolio",
  holdings: [makeHolding()],
  total_holding_count: 1,
  total_cost_basis: 200_000_000,
  ...overrides,
});

describe("toHoldingRow", () => {
  it("formats quantity with 6 decimals", () => {
    const row = toHoldingRow(makeHolding({ quantity: 1_500_000 }));
    expect(row.quantity).toBe("1.500000");
  });

  it("formats averagePrice with 2 decimals", () => {
    const row = toHoldingRow(makeHolding({ average_price: 150_000_000 }));
    expect(row.averagePrice).toBe("150.00");
  });

  it("formats costBasis with 2 decimals", () => {
    const row = toHoldingRow(makeHolding({ cost_basis: 300_000_000 }));
    expect(row.costBasis).toBe("300.00");
  });

  it("maps asset metadata fields correctly", () => {
    const row = toHoldingRow(makeHolding());
    expect(row.assetId).toBe("asset-1");
    expect(row.assetName).toBe("Apple Inc");
    expect(row.assetReference).toBe("AAPL");
  });
});

describe("toAccountSummary", () => {
  it("formats totalCostBasis with 2 decimals", () => {
    const summary = toAccountSummary(makeResponse({ total_cost_basis: 250_000_000 }));
    expect(summary.totalCostBasis).toBe("250.00");
  });

  it("isEmpty true when total_holding_count is 0", () => {
    const summary = toAccountSummary(makeResponse({ total_holding_count: 0, holdings: [] }));
    expect(summary.isEmpty).toBe(true);
    expect(summary.isAllClosed).toBe(false);
  });

  it("isAllClosed true when holdings exist but active list is empty (ACD-034)", () => {
    const summary = toAccountSummary(makeResponse({ total_holding_count: 2, holdings: [] }));
    expect(summary.isEmpty).toBe(false);
    expect(summary.isAllClosed).toBe(true);
  });

  it("neither isEmpty nor isAllClosed when active holdings present", () => {
    const summary = toAccountSummary(makeResponse());
    expect(summary.isEmpty).toBe(false);
    expect(summary.isAllClosed).toBe(false);
  });
});
