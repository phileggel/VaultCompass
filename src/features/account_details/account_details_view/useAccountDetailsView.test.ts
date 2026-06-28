import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useAppStore } from "@/lib/store";
import { useAccountDetailsView } from "./useAccountDetailsView";

const mockBlock = vi.fn();
const mockUnblock = vi.fn();
const mockShowSnackbar = vi.fn();
const mockFetchAssets = vi.fn().mockResolvedValue(undefined);
// Defaults to an error response (most tests don't need holdings); individual
// tests override per-call via `mockResolvedValueOnce` to supply holdings.
const mockGetAccountDetails = vi.fn((..._args: unknown[]) =>
  Promise.resolve({ status: "error", error: { code: "DatabaseError" } }),
);

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => vi.fn(),
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

// ---------------------------------------------------------------------------
// DIV-011/020 — dividendPayingAssets exposes only active, non-cash holdings
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
  ...overrides,
});

describe("useAccountDetailsView — dividendPayingAssets filter (DIV-011/020)", () => {
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

    expect(result.current.dividendPayingAssets).toEqual([
      { assetId: "asset-active", assetName: "Active Co", assetCurrency: "USD" },
    ]);
  });
});
