import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { HoldingsAsOfResponse } from "@/bindings";
import { useHoldingsAsOf } from "./useHoldingsAsOf";

const { mockGetAccountHoldingsAsOf } = vi.hoisted(() => ({
  mockGetAccountHoldingsAsOf: vi.fn(),
}));

vi.mock("../gateway", () => ({
  accountDetailsGateway: {
    getAccountHoldingsAsOf: mockGetAccountHoldingsAsOf,
  },
}));

const TODAY = new Date().toISOString().slice(0, 10);

const RESPONSE: HoldingsAsOfResponse = {
  account_name: "Acme",
  as_of_date: TODAY,
  account_currency: "EUR",
  holdings: [
    {
      asset_id: "asset-1",
      asset_name: "Apple",
      asset_currency: "EUR",
      quantity: 2_000_000,
      average_price: 100_000_000,
      cost_basis: 200_000_000,
      market_value: 240_000_000,
      price: 120_000_000,
      price_date: "2024-03-01",
      unrealized_pnl: 40_000_000,
    },
  ],
  total_cost_basis: 200_000_000,
  total_market_value: 240_000_000,
};

describe("useHoldingsAsOf", () => {
  beforeEach(() => {
    mockGetAccountHoldingsAsOf.mockReset();
  });

  it("starts loading then resolves to formatted rows + totals", async () => {
    mockGetAccountHoldingsAsOf.mockResolvedValue({ status: "ok", data: RESPONSE });
    const { result } = renderHook(() => useHoldingsAsOf("account-1"));

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.error).toBeNull();
    expect(result.current.rows).toHaveLength(1);
    expect(result.current.rows[0]?.assetName).toBe("Apple");
    expect(result.current.accountCurrency).toBe("EUR");
    expect(result.current.totalCostBasis).not.toBe("");
    expect(result.current.totalMarketValue).not.toBe("");
    expect(mockGetAccountHoldingsAsOf).toHaveBeenCalledWith("account-1", TODAY);
  });

  it("re-fetches when the date changes", async () => {
    mockGetAccountHoldingsAsOf.mockResolvedValue({ status: "ok", data: RESPONSE });
    const { result } = renderHook(() => useHoldingsAsOf("account-1"));
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    act(() => result.current.setDate("2024-01-15"));

    await waitFor(() =>
      expect(mockGetAccountHoldingsAsOf).toHaveBeenLastCalledWith("account-1", "2024-01-15"),
    );
  });

  it("maps a backend error to an i18n message and clears rows", async () => {
    mockGetAccountHoldingsAsOf.mockResolvedValue({
      status: "error",
      error: { code: "DateInFuture" },
    });
    const { result } = renderHook(() => useHoldingsAsOf("account-1"));

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.error).toEqual({ key: "error.DateInFuture" });
    expect(result.current.rows).toHaveLength(0);
  });
});
