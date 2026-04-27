import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useBuyTransaction } from "./useBuyTransaction";

const AUTO_RECORD_PRICE_KEY = "auto_record_price";

const mockAddTransaction = vi.fn();

vi.mock("@/features/transactions/useTransactions", () => ({
  useTransactions: () => ({
    addTransaction: mockAddTransaction,
    updateTransaction: vi.fn(),
    deleteTransaction: vi.fn(),
    getTransactions: vi.fn(),
  }),
}));

vi.mock("@/lib/store", () => ({
  useAppStore: vi.fn((selector) =>
    selector({
      assets: [{ id: "asset-1", name: "Apple", is_archived: false, currency: "USD" }],
      accounts: [{ id: "account-1", name: "My Account" }],
    }),
  ),
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
    mockAddTransaction.mockReset();
  });

  // MKT-052 — recordPrice defaults to the global toggle value at hook mount (create mode)
  it("recordPrice defaults to false when localStorage auto_record_price is absent", () => {
    const { result } = renderHook(() => useBuyTransaction(BASE_PROPS));
    expect(result.current.recordPrice).toBe(false);
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

  // MKT-054 — submit forwards recordPrice=true to addTransaction as record_price: true
  it("forwards record_price: true to addTransaction when recordPrice is true", async () => {
    localStorage.setItem(AUTO_RECORD_PRICE_KEY, "true");
    mockAddTransaction.mockResolvedValue({ data: { id: "tx-1" }, error: null });

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

    expect(mockAddTransaction).toHaveBeenCalledWith(
      expect.objectContaining({ record_price: true }),
    );
  });

  // MKT-054 — submit forwards recordPrice=false to addTransaction as record_price: false
  it("forwards record_price: false to addTransaction when recordPrice is false", async () => {
    localStorage.removeItem(AUTO_RECORD_PRICE_KEY);
    mockAddTransaction.mockResolvedValue({ data: { id: "tx-2" }, error: null });

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

    expect(mockAddTransaction).toHaveBeenCalledWith(
      expect.objectContaining({ record_price: false }),
    );
  });
});
