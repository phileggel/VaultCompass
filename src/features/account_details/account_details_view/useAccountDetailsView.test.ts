import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getPerfPeriod, setPerfPeriod } from "@/lib/perfPeriodStorage";
import { useAppStore } from "@/lib/store";
import { useAccountDetailsView } from "./useAccountDetailsView";

const mockBlock = vi.fn();
const mockUnblock = vi.fn();
const mockShowSnackbar = vi.fn();
const mockNavigate = vi.fn();
const mockFetchAssets = vi.fn().mockResolvedValue(undefined);
// Defaults to an error response (most tests don't need holdings); individual
// tests override per-call via `mockResolvedValueOnce` to supply holdings.
const mockGetAccountDetails = vi.fn((..._args: unknown[]) =>
  Promise.resolve({ status: "error", error: { code: "DatabaseError" } }),
);

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => mockNavigate,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en-US" } }),
}));

vi.mock("@/lib/logger", () => ({
  logger: { error: vi.fn(), info: vi.fn() },
}));

vi.mock("@/ui/components/snackbar/snackbarStore", () => ({
  useSnackbar: () => mockShowSnackbar,
}));

vi.mock("../gateway", () => ({
  accountDetailsGateway: {
    blockAssetPriceRefresh: (...args: unknown[]) => mockBlock(...args),
    unblockAssetPriceRefresh: (...args: unknown[]) => mockUnblock(...args),
    getAccountDetails: (...args: unknown[]) => mockGetAccountDetails(...args),
    subscribeToEvents: vi.fn(() => Promise.resolve(() => {})),
  },
  // useAccountDetails reads the asset catalog via this selector; back it with the
  // real store so the setState-driven tests still drive it.
  useCachedAssets: () => useAppStore((state) => state.assets),
}));

describe("useAccountDetailsView — price-refresh lock toggle (MKT-156/157)", () => {
  beforeEach(() => {
    mockBlock.mockReset();
    mockUnblock.mockReset();
    mockShowSnackbar.mockReset();
    mockFetchAssets.mockClear();
    useAppStore.setState({
      assets: [],
      accounts: [{ id: "acc-1", name: "Main", currency: "USD" }] as never,
      fetchAssets: mockFetchAssets,
    } as never);
  });

  it("calls blockAssetPriceRefresh, refetches assets, and surfaces a success snackbar when toggling an unlocked asset", async () => {
    mockBlock.mockResolvedValue({ status: "ok", data: null });
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));

    await act(async () => {
      await result.current.handleTogglePriceRefreshLock("asset-1", false);
    });

    expect(mockBlock).toHaveBeenCalledWith("asset-1");
    expect(mockUnblock).not.toHaveBeenCalled();
    expect(mockFetchAssets).toHaveBeenCalledTimes(1);
    expect(mockShowSnackbar).toHaveBeenCalledWith("mkt.lock.success_blocked", "success");
  });

  it("calls unblockAssetPriceRefresh when the asset is currently locked", async () => {
    mockUnblock.mockResolvedValue({ status: "ok", data: null });
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));

    await act(async () => {
      await result.current.handleTogglePriceRefreshLock("asset-1", true);
    });

    expect(mockUnblock).toHaveBeenCalledWith("asset-1");
    expect(mockBlock).not.toHaveBeenCalled();
    expect(mockShowSnackbar).toHaveBeenCalledWith("mkt.lock.success_unblocked", "success");
  });

  it("surfaces a typed error snackbar when the backend rejects", async () => {
    mockBlock.mockResolvedValue({ status: "error", error: { code: "CashAssetNotEditable" } });
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));

    await act(async () => {
      await result.current.handleTogglePriceRefreshLock("cash-id", false);
    });

    expect(mockShowSnackbar).toHaveBeenCalledWith("error.CashAssetNotEditable", "error");
    expect(mockFetchAssets).not.toHaveBeenCalled();
  });

  it("surfaces a generic error snackbar when the gateway throws", async () => {
    mockBlock.mockRejectedValue(new Error("ipc broken"));
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));

    await act(async () => {
      await result.current.handleTogglePriceRefreshLock("asset-1", false);
    });

    expect(mockShowSnackbar).toHaveBeenCalledWith("error.Unknown", "error");
  });
});

// ---------------------------------------------------------------------------
// DIV-012 — Header "Record" menu: dividend modal state in useAccountDetailsView.
// The AccountDetailsView component replaces three standalone header buttons
// with a consolidated "Record" dropdown; the dividend modal open/close/success
// state is managed here. The view-level menu composition (button ids, item
// routing) is a render concern covered in AccountDetailsView.test.tsx.
// ---------------------------------------------------------------------------

describe("useAccountDetailsView — dividend modal state (DIV-012)", () => {
  beforeEach(() => {
    mockBlock.mockReset();
    mockUnblock.mockReset();
    mockShowSnackbar.mockReset();
    useAppStore.setState({
      assets: [],
      accounts: [{ id: "acc-1", name: "Main", currency: "EUR" }] as never,
      fetchAssets: mockFetchAssets,
    } as never);
  });

  // DIV-012 — dividendOpen is initially false
  it("dividendOpen starts as false", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    expect(result.current.dividendOpen).toBe(false);
  });

  // DIV-012 — handleDividendOpen sets dividendOpen to true
  it("handleDividendOpen sets dividendOpen to true (DIV-012)", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    act(() => result.current.handleDividendOpen());
    expect(result.current.dividendOpen).toBe(true);
  });

  // DIV-012 — handleDividendClose resets dividendOpen to false
  it("handleDividendClose resets dividendOpen to false (DIV-012)", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    act(() => result.current.handleDividendOpen());
    act(() => result.current.handleDividendClose());
    expect(result.current.dividendOpen).toBe(false);
  });

  // DIV-012 — handleDividendSuccess closes the modal and triggers a data re-fetch
  it("handleDividendSuccess closes modal and calls retry (DIV-012)", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    act(() => result.current.handleDividendOpen());

    act(() => result.current.handleDividendSuccess());

    expect(result.current.dividendOpen).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// FSD-012 — Header "Record" menu: free-shares modal state in useAccountDetailsView.
// Mirrors the DIV-012 dividend pattern: freeSharesOpen starts false, flips
// true on handleFreeSharesOpen, resets to false on close/success.
// ---------------------------------------------------------------------------

describe("useAccountDetailsView — free-shares modal state (FSD-012)", () => {
  beforeEach(() => {
    mockBlock.mockReset();
    mockUnblock.mockReset();
    mockShowSnackbar.mockReset();
    useAppStore.setState({
      assets: [],
      accounts: [{ id: "acc-1", name: "Main", currency: "EUR" }] as never,
      fetchAssets: mockFetchAssets,
    } as never);
  });

  // FSD-012 — freeSharesOpen starts as false
  it("freeSharesOpen starts as false (FSD-012)", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    expect(result.current.freeSharesOpen).toBe(false);
  });

  // FSD-012 — handleFreeSharesOpen sets freeSharesOpen to true
  it("handleFreeSharesOpen sets freeSharesOpen to true (FSD-012)", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    act(() => result.current.handleFreeSharesOpen());
    expect(result.current.freeSharesOpen).toBe(true);
  });

  // FSD-012 — handleFreeSharesClose resets freeSharesOpen to false
  it("handleFreeSharesClose resets freeSharesOpen to false (FSD-012)", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    act(() => result.current.handleFreeSharesOpen());
    act(() => result.current.handleFreeSharesClose());
    expect(result.current.freeSharesOpen).toBe(false);
  });

  // FSD-012 — handleFreeSharesSuccess closes the modal
  it("handleFreeSharesSuccess closes the modal (FSD-012)", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    act(() => result.current.handleFreeSharesOpen());
    act(() => result.current.handleFreeSharesSuccess());
    expect(result.current.freeSharesOpen).toBe(false);
  });
});

describe("useAccountDetailsView — management-fee modal state (FEE-010)", () => {
  beforeEach(() => {
    useAppStore.setState({
      assets: [],
      accounts: [{ id: "acc-1", name: "Main", currency: "EUR" }] as never,
      fetchAssets: mockFetchAssets,
    } as never);
  });

  it("managementFeeOpen starts false and flips on open", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    expect(result.current.managementFeeOpen).toBe(false);
    act(() => result.current.handleManagementFeeOpen());
    expect(result.current.managementFeeOpen).toBe(true);
  });

  it("handleManagementFeeClose and handleManagementFeeSuccess close the modal", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    act(() => result.current.handleManagementFeeOpen());
    act(() => result.current.handleManagementFeeClose());
    expect(result.current.managementFeeOpen).toBe(false);
    act(() => result.current.handleManagementFeeOpen());
    act(() => result.current.handleManagementFeeSuccess());
    expect(result.current.managementFeeOpen).toBe(false);
  });
});

describe("useAccountDetailsView — interest modal state (INT-010)", () => {
  beforeEach(() => {
    useAppStore.setState({
      assets: [],
      accounts: [{ id: "acc-1", name: "Main", currency: "EUR" }] as never,
      fetchAssets: mockFetchAssets,
    } as never);
  });

  it("interestOpen starts false and flips on open", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    expect(result.current.interestOpen).toBe(false);
    act(() => result.current.handleInterestOpen());
    expect(result.current.interestOpen).toBe(true);
  });

  it("handleInterestClose and handleInterestSuccess close the modal", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    act(() => result.current.handleInterestOpen());
    act(() => result.current.handleInterestClose());
    expect(result.current.interestOpen).toBe(false);
    act(() => result.current.handleInterestOpen());
    act(() => result.current.handleInterestSuccess());
    expect(result.current.interestOpen).toBe(false);
  });
});

describe("useAccountDetailsView — fee-schedule modal target (FEE-011)", () => {
  beforeEach(() => {
    useAppStore.setState({
      assets: [],
      accounts: [{ id: "acc-1", name: "Main", currency: "EUR" }] as never,
      fetchAssets: mockFetchAssets,
    } as never);
  });

  it("feeScheduleTarget starts null and captures asset id + name on open", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    expect(result.current.feeScheduleTarget).toBeNull();
    act(() => result.current.handleFeeScheduleOpen("asset-9", "Vanguard ETF"));
    expect(result.current.feeScheduleTarget).toEqual({
      assetId: "asset-9",
      assetName: "Vanguard ETF",
    });
  });

  it("handleFeeScheduleClose and handleFeeScheduleSuccess clear the target", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    act(() => result.current.handleFeeScheduleOpen("asset-9", "Vanguard ETF"));
    act(() => result.current.handleFeeScheduleClose());
    expect(result.current.feeScheduleTarget).toBeNull();
    act(() => result.current.handleFeeScheduleOpen("asset-9", "Vanguard ETF"));
    act(() => result.current.handleFeeScheduleSuccess());
    expect(result.current.feeScheduleTarget).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// DIV-011/020 — activeNonCashHoldings exposes only active, non-cash holdings
// (quantity > 0) as candidates for the dividend modal's asset selector.
// ---------------------------------------------------------------------------
const makeHoldingDetail = (overrides: Record<string, unknown> = {}) => ({
  asset_id: "asset-1",
  asset_name: "Asset One",
  asset_reference: "A1",
  quantity: 1_000_000,
  average_price: 1_000_000,
  cost_basis: 1_000_000,
  realized_pnl: 0,
  asset_currency: "EUR",
  current_price: null,
  current_price_date: null,
  current_price_source: null,
  unrealized_pnl: null,
  performance_pct: null,
  dividends_received: 0,
  total_return_pct: null,
  period_performance: {
    ytd: null,
    one_year: null,
    two_years: null,
    five_years: null,
    ten_years: null,
  },
  ...overrides,
});

describe("useAccountDetailsView — activeNonCashHoldings filter (DIV-011/020)", () => {
  beforeEach(() => {
    useAppStore.setState({
      assets: [],
      accounts: [{ id: "acc-1", name: "Main", currency: "EUR" }] as never,
      fetchAssets: mockFetchAssets,
    } as never);
  });

  it("includes only active non-cash holdings (excludes cash + zero-quantity)", async () => {
    mockGetAccountDetails.mockResolvedValueOnce({
      status: "ok",
      data: {
        account_name: "Main",
        holdings: [
          makeHoldingDetail({
            asset_id: "system-cash-eur",
            asset_name: "Cash EUR",
            quantity: 500_000_000,
          }),
          makeHoldingDetail({ asset_id: "asset-zero", asset_name: "Zero Co", quantity: 0 }),
          makeHoldingDetail({
            asset_id: "asset-active",
            asset_name: "Active Co",
            asset_currency: "USD",
            quantity: 2_000_000,
          }),
        ],
        closed_holdings: [],
        total_holding_count: 3,
        total_cost_basis: 0,
        total_realized_pnl: 0,
        total_unrealized_pnl: null,
        total_global_value: 0,
        total_dividends_received: 0,
      },
    } as never);

    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    await act(async () => {});

    expect(result.current.activeNonCashHoldings).toEqual([
      { assetId: "asset-active", assetName: "Active Co", assetCurrency: "USD" },
    ]);
  });

  // INT-020/023 — the interest candidates are the cash line plus the active
  // non-cash holdings whose asset is flagged interest_bearing (AST-024);
  // zero-quantity non-cash assets stay excluded even when flagged.
  it("interestEligibleHoldings includes the cash line and only flagged active non-cash holdings", async () => {
    useAppStore.setState({
      assets: [
        { id: "asset-zero", interest_bearing: true },
        { id: "asset-active", interest_bearing: true },
      ] as never,
    });
    mockGetAccountDetails.mockResolvedValueOnce({
      status: "ok",
      data: {
        account_name: "Main",
        holdings: [
          makeHoldingDetail({
            asset_id: "system-cash-eur",
            asset_name: "Cash EUR",
            quantity: 0,
          }),
          makeHoldingDetail({ asset_id: "asset-zero", asset_name: "Zero Co", quantity: 0 }),
          makeHoldingDetail({
            asset_id: "asset-active",
            asset_name: "Active Co",
            asset_currency: "USD",
            quantity: 2_000_000,
          }),
        ],
        closed_holdings: [],
        total_holding_count: 3,
        total_cost_basis: 0,
        total_realized_pnl: 0,
        total_unrealized_pnl: null,
        total_global_value: 0,
        total_dividends_received: 0,
      },
    } as never);

    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    await act(async () => {});

    expect(result.current.interestEligibleHoldings).toEqual([
      { assetId: "system-cash-eur", assetName: "Cash EUR", assetCurrency: "EUR" },
      { assetId: "asset-active", assetName: "Active Co", assetCurrency: "USD" },
    ]);
  });

  // INT-020 / AST-024 — a non-flagged non-cash holding is excluded from the
  // interest candidates even with quantity > 0; the cash line stays eligible.
  it("interestEligibleHoldings excludes unflagged non-cash holdings with quantity > 0", async () => {
    useAppStore.setState({
      assets: [
        { id: "asset-flagged", interest_bearing: true },
        { id: "asset-unflagged", interest_bearing: false },
      ] as never,
    });
    mockGetAccountDetails.mockResolvedValueOnce({
      status: "ok",
      data: {
        account_name: "Main",
        holdings: [
          makeHoldingDetail({
            asset_id: "system-cash-eur",
            asset_name: "Cash EUR",
            quantity: 500_000_000,
          }),
          makeHoldingDetail({
            asset_id: "asset-flagged",
            asset_name: "Flagged Co",
            quantity: 2_000_000,
          }),
          makeHoldingDetail({
            asset_id: "asset-unflagged",
            asset_name: "Unflagged Co",
            quantity: 3_000_000,
          }),
        ],
        closed_holdings: [],
        total_holding_count: 3,
        total_cost_basis: 0,
        total_realized_pnl: 0,
        total_unrealized_pnl: null,
        total_global_value: 0,
        total_dividends_received: 0,
      },
    } as never);

    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    await act(async () => {});

    expect(result.current.interestEligibleHoldings).toEqual([
      { assetId: "system-cash-eur", assetName: "Cash EUR", assetCurrency: "EUR" },
      { assetId: "asset-flagged", assetName: "Flagged Co", assetCurrency: "EUR" },
    ]);
  });
});

// ---------------------------------------------------------------------------
// As-of read-only mode: selecting a past date sets isAsOf and no-ops every
// mutation open-handler; clearing the date returns to the live, mutable view.
// ---------------------------------------------------------------------------

describe("useAccountDetailsView — as-of read-only mode", () => {
  beforeEach(() => {
    mockNavigate.mockReset();
    useAppStore.setState({
      assets: [],
      accounts: [{ id: "acc-1", name: "Main", currency: "EUR" }] as never,
      fetchAssets: mockFetchAssets,
    } as never);
  });

  it("isAsOf is false by default (live view)", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    expect(result.current.isAsOf).toBe(false);
  });

  it("selecting a past date enters as-of mode and blocks every mutation handler", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    act(() => result.current.setAsOfDate("2020-01-01"));
    expect(result.current.isAsOf).toBe(true);

    act(() => result.current.handleDividendOpen());
    act(() => result.current.handleFreeSharesOpen());
    act(() => result.current.handleDepositOpen());
    act(() => result.current.handleWithdrawalOpen());
    act(() => result.current.handleOpenBalanceOpen());
    act(() =>
      result.current.handleBuyOpen({
        accountName: "Main",
        assetId: "a1",
        assetName: "A1",
        assetCurrency: "EUR",
        showExchangeRate: false,
      }),
    );
    act(() =>
      result.current.handleSellOpen({
        accountName: "Main",
        assetId: "a1",
        assetName: "A1",
        assetCurrency: "EUR",
        showExchangeRate: false,
        holdingQuantityMicro: 1_000_000,
      }),
    );

    expect(result.current.dividendOpen).toBe(false);
    expect(result.current.freeSharesOpen).toBe(false);
    expect(result.current.depositOpen).toBe(false);
    expect(result.current.withdrawalOpen).toBe(false);
    expect(result.current.openBalanceOpen).toBe(false);
    expect(result.current.buyTarget).toBeNull();
    expect(result.current.sellTarget).toBeNull();
  });

  it("clearing the date returns to the live view", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    act(() => result.current.setAsOfDate("2020-01-01"));
    expect(result.current.isAsOf).toBe(true);
    act(() => result.current.setAsOfDate(""));
    expect(result.current.isAsOf).toBe(false);
  });

  // handleAddTransaction navigates (URL-driven modal) rather than setting state;
  // in as-of mode it must be a no-op (no navigate call).
  it("blocks handleAddTransaction in as-of mode (no navigate)", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    act(() => result.current.setAsOfDate("2020-01-01"));
    expect(result.current.isAsOf).toBe(true);

    act(() => result.current.handleAddTransaction());

    expect(mockNavigate).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// ACD-054 — performance-column period: since-start default, per-account
// persistence, and the as-of pin to since-start (windowed returns are a
// live-view metric).
// ---------------------------------------------------------------------------

describe("useAccountDetailsView — performance period (ACD-054)", () => {
  beforeEach(() => {
    localStorage.clear();
    useAppStore.setState({
      assets: [],
      accounts: [{ id: "acc-1", name: "Main", currency: "EUR" }] as never,
      fetchAssets: mockFetchAssets,
    } as never);
  });

  it("defaults to since_start when no preference is stored", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    expect(result.current.perfPeriod).toBe("since_start");
  });

  it("initializes from the stored per-account preference", () => {
    setPerfPeriod("acc-1", "ytd");
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    expect(result.current.perfPeriod).toBe("ytd");
  });

  it("setPerfPeriod updates the state and persists the choice", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    act(() => result.current.setPerfPeriod("five_years"));
    expect(result.current.perfPeriod).toBe("five_years");
    expect(getPerfPeriod("acc-1")).toBe("five_years");
  });

  it("pins the period to since_start in the as-of view without losing the stored choice", () => {
    setPerfPeriod("acc-1", "one_year");
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    expect(result.current.perfPeriod).toBe("one_year");

    act(() => result.current.setAsOfDate("2020-01-01"));
    expect(result.current.perfPeriod).toBe("since_start");

    act(() => result.current.setAsOfDate(""));
    expect(result.current.perfPeriod).toBe("one_year");
  });

  it("setPerfPeriod is inert in the as-of view (no state change, no persistence)", () => {
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    act(() => result.current.setAsOfDate("2020-01-01"));

    act(() => result.current.setPerfPeriod("ten_years"));

    expect(result.current.perfPeriod).toBe("since_start");
    expect(getPerfPeriod("acc-1")).toBeNull();
  });
});

describe("useAccountDetailsView — management fees gate (FEE-076)", () => {
  beforeEach(() => {
    useAppStore.setState({
      assets: [],
      fetchAssets: mockFetchAssets,
    } as never);
  });

  it("derives managementFeesEnabled from the account catalog", () => {
    useAppStore.setState({
      accounts: [
        { id: "acc-1", name: "Main", currency: "EUR", management_fees_enabled: true },
      ] as never,
    });
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    expect(result.current.managementFeesEnabled).toBe(true);
  });

  it("is false for a disabled account and for an unknown account", () => {
    useAppStore.setState({
      accounts: [
        { id: "acc-1", name: "Main", currency: "EUR", management_fees_enabled: false },
      ] as never,
    });
    const { result } = renderHook(() => useAccountDetailsView("acc-1"));
    expect(result.current.managementFeesEnabled).toBe(false);

    const { result: unknown } = renderHook(() => useAccountDetailsView("acc-missing"));
    expect(unknown.current.managementFeesEnabled).toBe(false);
  });
});
