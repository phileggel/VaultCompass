import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  Account,
  AccountDetailsResponse,
  AccountPerformanceResponse,
  Asset,
  HoldingDetail,
  PerformancePeriod,
} from "@/bindings";
import { useAppStore } from "@/lib/store";

// Mock the gateway so no real Tauri calls fire (docs/test_convention.md § Mocking gateway modules)
vi.mock("../gateway");

import * as gateway from "../gateway";
import { useGlobalPerformance } from "./useGlobalPerformance";

// ---- Fixtures ---------------------------------------------------------------

const makeYearRow = (overrides: Partial<PerformancePeriod> = {}): PerformancePeriod => ({
  year: 2025,
  month: null,
  end_value: 10_000_000_000,
  previous_value: 9_000_000_000,
  cash_flow: 500_000_000,
  asset_flow: 0,
  dividends: 120_000_000,
  pnl: 380_000_000,
  period_over_period: { gain: 500_000_000, pct: 5_000_000 },
  year_to_date: null,
  since_inception: { gain: 500_000_000, pct: 5_000_000 },
  annualized_yield: { gain: 500_000_000, pct: 5_000_000 },
  ...overrides,
});

const makeResponse = (
  overrides: Partial<AccountPerformanceResponse> = {},
): AccountPerformanceResponse => ({
  account_name: "",
  currency: "EUR",
  month_view_available: false,
  yearly: [makeYearRow()],
  monthly: [],
  ...overrides,
});

const makeAccount = (overrides: Partial<Account> = {}): Account => ({
  id: "account-1",
  name: "Broker One",
  bank_name: "",
  currency: "EUR",
  update_frequency: "ManualMonth",
  management_fees_enabled: false,
  ...overrides,
});

const makeCatalogAsset = (overrides: Partial<Asset> = {}): Asset => ({
  id: "asset-1",
  name: "Apple Inc",
  reference: "AAPL",
  isin: null,
  class: "Stocks",
  currency: "USD",
  risk_level: 4,
  category: { id: "cat-1", name: "US Stocks" },
  is_archived: false,
  price_refresh_blocked: false,
  interest_bearing: false,
  exchange: null,
  ...overrides,
});

const makeHolding = (overrides: Partial<HoldingDetail> = {}): HoldingDetail => ({
  asset_id: "asset-1",
  asset_name: "Apple Inc",
  asset_reference: "AAPL",
  quantity: 2_000_000,
  average_price: 100_000_000,
  cost_basis: 200_000_000,
  realized_pnl: 0,
  asset_currency: "EUR",
  current_price: null,
  current_price_date: null,
  current_price_source: null,
  unrealized_pnl: null,
  performance_pct: null,
  dividends_received: 0,
  total_return_pct: null,
  fx_rate_date: null,
  management_fees: 0,
  market_value: null,
  fee_rate_percent_micros: null,
  period_performance: {
    ytd: null,
    one_year: null,
    two_years: null,
    five_years: null,
    ten_years: null,
  },
  ...overrides,
});

const makeDetailsResponse = (
  overrides: Partial<AccountDetailsResponse> = {},
): AccountDetailsResponse => ({
  account_name: "Broker One",
  holdings: [makeHolding({ asset_id: "system-cash-EUR", asset_name: "Cash" }), makeHolding()],
  closed_holdings: [],
  total_holding_count: 2,
  total_cost_basis: 200_000_000,
  total_realized_pnl: 0,
  total_unrealized_pnl: null,
  total_global_value: 0,
  total_dividends_received: 0,
  total_management_fees: 0,
  total_net_cash_input: 0,
  ...overrides,
});

// ---- Tests ------------------------------------------------------------------

describe("useGlobalPerformance", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      accounts: [
        makeAccount({ id: "account-2", name: "Zeta Bank" }),
        makeAccount({ id: "account-1", name: "Broker One" }),
      ],
      assets: [
        makeCatalogAsset({ id: "asset-2", name: "Microsoft Corp", reference: "MSFT" }),
        makeCatalogAsset(),
        makeCatalogAsset({ id: "asset-3", name: "Old Fund", is_archived: true }),
        makeCatalogAsset({ id: "system-cash-EUR", name: "Cash" }),
      ],
    });
    vi.mocked(gateway.globalPerformanceGateway.getGlobalPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse(),
    });
    vi.mocked(gateway.globalPerformanceGateway.getAccountHoldings).mockResolvedValue({
      status: "ok",
      data: makeDetailsResponse(),
    });
    vi.mocked(gateway.globalPerformanceGateway.subscribeToEvents).mockResolvedValue(() => {});
  });

  // GPF-010 — default scope is the whole portfolio: (null, null)
  it("defaults to all accounts and all assets (GPF-010)", async () => {
    const { result } = renderHook(() => useGlobalPerformance());

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.selectedAccountId).toBeNull();
    expect(result.current.selectedAssetId).toBeNull();
    expect(result.current.scopeLabel).toBeNull();
    expect(gateway.globalPerformanceGateway.getGlobalPerformance).toHaveBeenCalledWith(null, null);
  });

  // GPF-011 — the reporting currency of the response is exposed
  it("exposes the response currency (GPF-011)", async () => {
    const { result } = renderHook(() => useGlobalPerformance());

    await waitFor(() => expect(result.current.currency).toBe("EUR"));
  });

  // The account selector offers every catalog account, name asc
  it("exposes the accounts catalog as account options, name asc", async () => {
    const { result } = renderHook(() => useGlobalPerformance());

    await waitFor(() =>
      expect(result.current.accountOptions).toEqual([
        { accountId: "account-1", accountName: "Broker One" },
        { accountId: "account-2", accountName: "Zeta Bank" },
      ]),
    );
  });

  // All-accounts scope — the asset selector offers the non-archived non-cash catalog, name asc
  it("offers the non-archived non-cash catalog assets when unscoped", async () => {
    const { result } = renderHook(() => useGlobalPerformance());

    await waitFor(() =>
      expect(result.current.assetOptions).toEqual([
        { assetId: "asset-1", assetName: "Apple Inc" },
        { assetId: "asset-2", assetName: "Microsoft Corp" },
      ]),
    );
  });

  // GPF-010 — selecting an account re-fetches with the account scope
  it("re-fetches with the account id when an account scope is selected (GPF-010)", async () => {
    const { result } = renderHook(() => useGlobalPerformance());
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    act(() => result.current.setSelectedAccountId("account-1"));

    await waitFor(() =>
      expect(gateway.globalPerformanceGateway.getGlobalPerformance).toHaveBeenCalledWith(
        "account-1",
        null,
      ),
    );
    expect(result.current.selectedAccountId).toBe("account-1");
    await waitFor(() => expect(result.current.scopeLabel).toBe("Broker One"));
  });

  // Account scope — the asset selector switches to the account's non-cash holdings
  it("offers the scoped account's non-cash holdings as asset options", async () => {
    const { result } = renderHook(() => useGlobalPerformance());
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    act(() => result.current.setSelectedAccountId("account-1"));

    await waitFor(() =>
      expect(result.current.assetOptions).toEqual([{ assetId: "asset-1", assetName: "Apple Inc" }]),
    );
    expect(gateway.globalPerformanceGateway.getAccountHoldings).toHaveBeenCalledWith("account-1");
  });

  // GPF-010 — selecting an asset re-fetches with both scope ids
  it("re-fetches with account and asset ids when both scopes are selected (GPF-010)", async () => {
    const { result } = renderHook(() => useGlobalPerformance());
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    act(() => result.current.setSelectedAccountId("account-1"));
    await waitFor(() => expect(result.current.selectedAccountId).toBe("account-1"));

    act(() => result.current.setSelectedAssetId("asset-1"));

    await waitFor(() =>
      expect(gateway.globalPerformanceGateway.getGlobalPerformance).toHaveBeenCalledWith(
        "account-1",
        "asset-1",
      ),
    );
    await waitFor(() => expect(result.current.scopeLabel).toBe("Broker One — Apple Inc"));
  });

  // GPF-010 — an asset scope without an account scope reads the asset across accounts
  it("re-fetches with the asset id alone for the cross-account asset scope (GPF-010)", async () => {
    const { result } = renderHook(() => useGlobalPerformance());
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    act(() => result.current.setSelectedAssetId("asset-2"));

    await waitFor(() =>
      expect(gateway.globalPerformanceGateway.getGlobalPerformance).toHaveBeenCalledWith(
        null,
        "asset-2",
      ),
    );
    await waitFor(() => expect(result.current.scopeLabel).toBe("Microsoft Corp"));
  });

  // Changing the account scope resets the asset scope to All assets
  it("resets the asset scope when the account scope changes", async () => {
    const { result } = renderHook(() => useGlobalPerformance());
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    act(() => result.current.setSelectedAccountId("account-1"));
    await waitFor(() => expect(result.current.selectedAccountId).toBe("account-1"));
    act(() => result.current.setSelectedAssetId("asset-1"));
    await waitFor(() => expect(result.current.selectedAssetId).toBe("asset-1"));

    act(() => result.current.setSelectedAccountId("account-2"));

    expect(result.current.selectedAssetId).toBeNull();
    await waitFor(() =>
      expect(gateway.globalPerformanceGateway.getGlobalPerformance).toHaveBeenCalledWith(
        "account-2",
        null,
      ),
    );
    // The new account scope is never fetched with the previous asset scope.
    expect(gateway.globalPerformanceGateway.getGlobalPerformance).not.toHaveBeenCalledWith(
      "account-2",
      "asset-1",
    );
  });

  // Returning the account scope to All accounts re-fetches the whole portfolio
  it("re-fetches unscoped when the account scope returns to All accounts", async () => {
    const { result } = renderHook(() => useGlobalPerformance());
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    act(() => result.current.setSelectedAccountId("account-1"));
    await waitFor(() => expect(result.current.selectedAccountId).toBe("account-1"));

    act(() => result.current.setSelectedAccountId(null));

    await waitFor(() => expect(result.current.selectedAccountId).toBeNull());
    const calls = vi.mocked(gateway.globalPerformanceGateway.getGlobalPerformance).mock.calls;
    expect(calls[calls.length - 1]).toEqual([null, null]);
  });

  // F27 — a gateway error surfaces as an i18n message with retry
  it("exposes the presented error and retries the fetch", async () => {
    vi.mocked(gateway.globalPerformanceGateway.getGlobalPerformance).mockResolvedValue({
      status: "error",
      error: { code: "DatabaseError" },
    });

    const { result } = renderHook(() => useGlobalPerformance());

    await waitFor(() =>
      expect(result.current.error).toEqual({
        key: "account_performance.error.database_error",
      }),
    );

    vi.mocked(gateway.globalPerformanceGateway.getGlobalPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse(),
    });
    await act(async () => result.current.retry());

    await waitFor(() => expect(result.current.error).toBeNull());
    expect(result.current.rows).toHaveLength(1);
  });

  // GPF-014 — month view available opens in month view with the most recent year selected
  it("opens in month view with the most recent year when month view is available (GPF-014)", async () => {
    vi.mocked(gateway.globalPerformanceGateway.getGlobalPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse({
        month_view_available: true,
        monthly: [makeYearRow({ year: 2025, month: 5 }), makeYearRow({ year: 2024, month: 12 })],
      }),
    });

    const { result } = renderHook(() => useGlobalPerformance());

    await waitFor(() => expect(result.current.viewMode).toBe("month"));
    expect(result.current.selectedYear).toBe(2025);
    expect(result.current.availableYears).toEqual([2025, 2024]);
  });

  // GPF-015 — the empty portfolio read is exposed as isEmpty
  it("flags the empty portfolio (GPF-015)", async () => {
    vi.mocked(gateway.globalPerformanceGateway.getGlobalPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse({ yearly: [], monthly: [] }),
    });

    const { result } = renderHook(() => useGlobalPerformance());

    await waitFor(() => expect(result.current.isEmpty).toBe(true));
  });
});
