import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Account, Asset } from "@/bindings";
import { useAppStore } from "@/lib/store";
import { useBuyTransaction } from "./useBuyTransaction";

const AUTO_RECORD_PRICE_KEY = "auto_record_price";

const { mockBuyHolding, mockRecordAssetPrice, mockGetSnapshot } = vi.hoisted(() => ({
  mockBuyHolding: vi.fn(),
  mockRecordAssetPrice: vi.fn(),
  mockGetSnapshot: vi.fn(),
}));

vi.mock("@/features/transactions/useTransactions", () => ({
  useTransactions: () => ({
    buyHolding: mockBuyHolding,
    sellHolding: vi.fn(),
    correctTransaction: vi.fn(),
    cancelTransaction: vi.fn(),
    getTransactions: vi.fn(),
  }),
}));

vi.mock("../gateway", () => ({
  accountDetailsGateway: {
    recordAssetPrice: mockRecordAssetPrice,
    getHoldingSnapshotAsOf: mockGetSnapshot,
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "en" },
  }),
}));

const BASE_PROPS = {
  accountId: "account-1",
  assetId: "asset-1",
};

const fakeSubmit = { preventDefault: vi.fn() } as unknown as React.FormEvent;

describe("useBuyTransaction", () => {
  beforeEach(() => {
    localStorage.clear();
    mockBuyHolding.mockReset();
    mockRecordAssetPrice.mockReset();
    mockGetSnapshot.mockReset();
    mockGetSnapshot.mockResolvedValue({ status: "ok", data: { quantity: 0, average_price: 0 } });
    useAppStore.setState({
      assets: [{ id: "asset-1", name: "Apple", is_archived: false, currency: "USD" }] as Asset[],
      accounts: [{ id: "account-1", name: "My Account" }] as Account[],
    });
  });

  // MKT-052 — recordPrice defaults to the global toggle value at hook mount (create mode)
  it("recordPrice defaults to false when localStorage auto_record_price is absent", () => {
    const { result } = renderHook(() => useBuyTransaction(BASE_PROPS));
    expect(result.current.recordPrice).toBe(false);
  });

  // TDI-020 — average cost is exposed when the asset is held as of the date.
  it("exposes averageCostAsOfDate when the asset is held as of the date", async () => {
    mockGetSnapshot.mockResolvedValue({
      status: "ok",
      data: { quantity: 2_000_000, average_price: 100_000_000 },
    });
    const { result } = renderHook(() => useBuyTransaction(BASE_PROPS));
    await waitFor(() => expect(result.current.averageCostAsOfDate).not.toBeNull());
  });

  // TDI-021 — average cost is hidden when nothing is held as of the date.
  it("hides averageCostAsOfDate when nothing is held as of the date", async () => {
    const { result } = renderHook(() => useBuyTransaction(BASE_PROPS));
    await waitFor(() => expect(mockGetSnapshot).toHaveBeenCalled());
    expect(result.current.averageCostAsOfDate).toBeNull();
  });

  // MKT-052 — recordPrice is true at mount when localStorage key is "true"
  it("recordPrice defaults to true when localStorage auto_record_price is true", () => {
    localStorage.setItem(AUTO_RECORD_PRICE_KEY, "true");
    const { result } = renderHook(() => useBuyTransaction(BASE_PROPS));
    expect(result.current.recordPrice).toBe(true);
  });

  // MKT-053 — snapshot at mount: changing localStorage after mount does not change recordPrice
  it("does not change recordPrice when localStorage is mutated after mount (snapshot semantics)", async () => {
    localStorage.removeItem(AUTO_RECORD_PRICE_KEY);
    const { result } = renderHook(() => useBuyTransaction(BASE_PROPS));

    expect(result.current.recordPrice).toBe(false);

    // Mutate localStorage directly without re-rendering
    localStorage.setItem(AUTO_RECORD_PRICE_KEY, "true");

    // recordPrice must remain the snapshot value from mount
    expect(result.current.recordPrice).toBe(false);
  });

  // MKT-054 — calls recordAssetPrice when recordPrice is true and price is non-zero
  it("calls recordAssetPrice when recordPrice is true and price is non-zero", async () => {
    localStorage.setItem(AUTO_RECORD_PRICE_KEY, "true");
    mockBuyHolding.mockResolvedValue({ data: { id: "tx-1" }, error: null });
    mockRecordAssetPrice.mockResolvedValue({ status: "ok", data: null });

    const { result } = renderHook(() => useBuyTransaction(BASE_PROPS));

    await act(async () => {
      result.current.handleChange("date", "2024-06-01");
      result.current.handleChange("quantity", "1");
      result.current.handleChange("unitPrice", "100");
      result.current.handleChange("exchangeRate", "1");
      result.current.handleChange("fees", "0");
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(mockRecordAssetPrice).toHaveBeenCalledWith("asset-1", "2024-06-01", 100);
  });

  // MKT-054 — does not call recordAssetPrice when recordPrice is false
  it("does not call recordAssetPrice when recordPrice is false", async () => {
    localStorage.removeItem(AUTO_RECORD_PRICE_KEY);
    mockBuyHolding.mockResolvedValue({ data: { id: "tx-2" }, error: null });

    const { result } = renderHook(() => useBuyTransaction(BASE_PROPS));

    await act(async () => {
      result.current.handleChange("date", "2024-06-01");
      result.current.handleChange("quantity", "1");
      result.current.handleChange("unitPrice", "100");
      result.current.handleChange("exchangeRate", "1");
      result.current.handleChange("fees", "0");
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(mockRecordAssetPrice).not.toHaveBeenCalled();
  });
});
