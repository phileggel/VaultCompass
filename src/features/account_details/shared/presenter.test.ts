import { describe, expect, it } from "vitest";
import type { AccountDetailsResponse, ClosedHoldingDetail, HoldingDetail } from "@/bindings";
import { toAccountSummary, toClosedHoldingRow, toHoldingRow } from "./presenter";

const makeHolding = (overrides: Partial<HoldingDetail> = {}): HoldingDetail => ({
  asset_id: "asset-1",
  asset_name: "Apple Inc",
  asset_reference: "AAPL",
  quantity: 2_000_000,
  average_price: 100_000_000,
  cost_basis: 200_000_000,
  realized_pnl: 0,
  ...overrides,
});

const makeClosedHolding = (overrides: Partial<ClosedHoldingDetail> = {}): ClosedHoldingDetail => ({
  asset_id: "asset-2",
  asset_name: "Closed Corp",
  asset_reference: "CLSD",
  realized_pnl: 0,
  last_sold_date: "2024-12-31",
  ...overrides,
});

const makeResponse = (overrides: Partial<AccountDetailsResponse> = {}): AccountDetailsResponse => ({
  account_name: "My Portfolio",
  holdings: [makeHolding()],
  closed_holdings: [],
  total_holding_count: 1,
  total_cost_basis: 200_000_000,
  total_realized_pnl: 0,
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

  it("formats realizedPnl with 2 decimals (SEL-042)", () => {
    const row = toHoldingRow(makeHolding({ realized_pnl: 5_000_000 }));
    expect(row.realizedPnl).toBe("5.00");
    expect(row.realizedPnlRaw).toBe(5_000_000);
  });

  it("passes quantityMicro as raw value for sell modal (SEL-010)", () => {
    const row = toHoldingRow(makeHolding({ quantity: 3_500_000 }));
    expect(row.quantityMicro).toBe(3_500_000);
  });
});

describe("toAccountSummary", () => {
  it("formats totalCostBasis with 2 decimals", () => {
    const summary = toAccountSummary(makeResponse({ total_cost_basis: 250_000_000 }));
    expect(summary.totalCostBasis).toBe("250.00");
  });

  it("formats totalRealizedPnl with 2 decimals (SEL-042)", () => {
    const summary = toAccountSummary(makeResponse({ total_realized_pnl: 12_500_000 }));
    expect(summary.totalRealizedPnl).toBe("12.50");
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

describe("toClosedHoldingRow", () => {
  // ACD-044 — closed holding detail maps to a view model row
  it("maps closed holding metadata fields (ACD-044)", () => {
    const row = toClosedHoldingRow(makeClosedHolding());
    expect(row.assetId).toBe("asset-2");
    expect(row.assetName).toBe("Closed Corp");
    expect(row.assetReference).toBe("CLSD");
  });

  // ACD-049 — realized P&L formatted to 2 decimal places
  it("formats realizedPnl with 2 decimals (ACD-049)", () => {
    const row = toClosedHoldingRow(makeClosedHolding({ realized_pnl: 15_000_000 }));
    expect(row.realizedPnl).toBe("15.00");
  });

  // ACD-049 — raw realized P&L exposed for sign-based colour styling
  it("exposes realizedPnlRaw as micro-unit number (ACD-049)", () => {
    const row = toClosedHoldingRow(makeClosedHolding({ realized_pnl: -5_000_000 }));
    expect(row.realizedPnlRaw).toBe(-5_000_000);
  });

  // ACD-049 — last_sold_date passed through verbatim
  it("passes lastSoldDate through verbatim (ACD-049)", () => {
    const row = toClosedHoldingRow(makeClosedHolding({ last_sold_date: "2025-06-15" }));
    expect(row.lastSoldDate).toBe("2025-06-15");
  });

  // ACD-047 — toAccountSummary totalRealizedPnl covers active + closed (backend sums, presenter passes through)
  it("toAccountSummary totalRealizedPnl includes closed positions pnl (ACD-047)", () => {
    const summary = toAccountSummary(
      makeResponse({
        total_realized_pnl: 35_000_000,
        closed_holdings: [makeClosedHolding({ realized_pnl: 25_000_000 })],
      }),
    );
    expect(summary.totalRealizedPnl).toBe("35.00");
  });

  // ACD-050 — empty closed_holdings list → hasClosedHoldings false
  it("toAccountSummary hasClosedHoldings is false when closed_holdings is empty (ACD-050)", () => {
    const summary = toAccountSummary(makeResponse({ closed_holdings: [] }));
    expect(summary.hasClosedHoldings).toBe(false);
  });

  // ACD-044/ACD-048 — hasClosedHoldings is true when closed_holdings is non-empty
  it("toAccountSummary hasClosedHoldings is true when closed_holdings is non-empty (ACD-044)", () => {
    const summary = toAccountSummary(makeResponse({ closed_holdings: [makeClosedHolding()] }));
    expect(summary.hasClosedHoldings).toBe(true);
  });
});
