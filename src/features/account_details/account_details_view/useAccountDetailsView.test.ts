import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useAppStore } from "@/lib/store";
import { useAccountDetailsView } from "./useAccountDetailsView";

const mockBlock = vi.fn();
const mockUnblock = vi.fn();
const mockShowSnackbar = vi.fn();
const mockFetchAssets = vi.fn().mockResolvedValue(undefined);

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
    getAccountDetails: vi.fn(() =>
      Promise.resolve({ status: "error", error: { code: "DatabaseError" } }),
    ),
    subscribeToEvents: vi.fn(() => Promise.resolve(() => {})),
  },
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
