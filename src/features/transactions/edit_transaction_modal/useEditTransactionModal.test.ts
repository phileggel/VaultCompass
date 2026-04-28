import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Transaction } from "@/bindings";
import { useEditTransactionModal } from "./useEditTransactionModal";

const { mockCorrectTransaction, mockRecordAssetPrice } = vi.hoisted(() => ({
  mockCorrectTransaction: vi.fn(),
  mockRecordAssetPrice: vi.fn(),
}));

vi.mock("../useTransactions", () => ({
  useTransactions: () => ({
    buyHolding: vi.fn(),
    sellHolding: vi.fn(),
    correctTransaction: mockCorrectTransaction,
    cancelTransaction: vi.fn(),
    getTransactions: vi.fn(),
  }),
}));

vi.mock("../gateway", () => ({
  transactionGateway: {
    recordAssetPrice: mockRecordAssetPrice,
  },
}));

vi.mock("@/lib/store", () => ({
  useAppStore: vi.fn((selector) =>
    selector({
      assets: [
        { id: "asset-1", name: "Apple", is_archived: false, currency: "USD" },
        { id: "asset-archived", name: "OldCo", is_archived: true, currency: "USD" },
      ],
      accounts: [{ id: "account-1", name: "My Account" }],
    }),
  ),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "fr" },
  }),
}));

const MICRO = 1_000_000;

// 2 units @ 50.0 each, rate=1.0, fees=0 → total=100_000_000
const baseTransaction: Transaction = {
  id: "tx-existing",
  account_id: "account-1",
  asset_id: "asset-1",
  transaction_type: "Purchase",
  date: "2024-01-10",
  quantity: 2 * MICRO,
  unit_price: 50 * MICRO,
  exchange_rate: 1 * MICRO,
  fees: 0,
  total_amount: 100 * MICRO,
  note: "initial note",
  realized_pnl: null,
  created_at: "2024-01-10T00:00:00Z",
};

describe("useEditTransactionModal", () => {
  beforeEach(() => {
    localStorage.clear();
    mockCorrectTransaction.mockReset();
    mockRecordAssetPrice.mockReset();
  });

  // Pre-fill: micro-unit values are converted to decimal strings; totalAmount is derived
  it("pre-fills formData from transaction (micro → decimal)", () => {
    const { result } = renderHook(() => useEditTransactionModal({ transaction: baseTransaction }));
    expect(result.current.formData.quantity).toBe("2.000");
    expect(result.current.formData.unitPrice).toBe("50.000");
    expect(result.current.formData.exchangeRate).toBe("1.000");
    expect(result.current.formData.note).toBe("initial note");
    // totalAmount is derived from micro values, not stored in formData
    expect(result.current.totalAmountDisplay).toBe("100,000");
  });

  // Submit calls correctTransaction with correct args: (id, accountId, dto)
  it("calls correctTransaction with correct args on submit", async () => {
    mockCorrectTransaction.mockResolvedValue({ data: { id: "tx-existing" }, error: null });
    const onSubmitSuccess = vi.fn();
    const { result } = renderHook(() =>
      useEditTransactionModal({ transaction: baseTransaction, onSubmitSuccess }),
    );

    const fakeSubmit = { preventDefault: vi.fn() } as unknown as React.FormEvent;

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(mockCorrectTransaction).toHaveBeenCalledWith(
      "tx-existing",
      "account-1",
      expect.objectContaining({
        quantity: 2 * MICRO,
        unit_price: 50 * MICRO,
        exchange_rate: 1 * MICRO,
      }),
    );
    expect(onSubmitSuccess).toHaveBeenCalledTimes(1);
  });

  // Backend error stays modal open
  it("sets error and does not call onSubmitSuccess on backend error", async () => {
    mockCorrectTransaction.mockResolvedValue({ data: null, error: "Not found" });
    const onSubmitSuccess = vi.fn();
    const { result } = renderHook(() =>
      useEditTransactionModal({ transaction: baseTransaction, onSubmitSuccess }),
    );

    const fakeSubmit = { preventDefault: vi.fn() } as unknown as React.FormEvent;
    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).toBe("Not found");
    expect(onSubmitSuccess).not.toHaveBeenCalled();
  });

  // MKT-052 — recordPrice is always false on edit regardless of localStorage
  it("recordPrice is false on edit mount even when localStorage auto_record_price is true", () => {
    localStorage.setItem("auto_record_price", "true");
    const { result } = renderHook(() => useEditTransactionModal({ transaction: baseTransaction }));
    expect(result.current.recordPrice).toBe(false);
  });

  // MKT-054 — calls recordAssetPrice when recordPrice is manually set to true
  it("calls recordAssetPrice when recordPrice is manually set to true", async () => {
    mockCorrectTransaction.mockResolvedValue({ data: { id: "tx-existing" }, error: null });
    mockRecordAssetPrice.mockResolvedValue({ status: "ok", data: null });

    const { result } = renderHook(() => useEditTransactionModal({ transaction: baseTransaction }));

    await act(async () => {
      result.current.setRecordPrice(true);
    });

    const fakeSubmit = { preventDefault: vi.fn() } as unknown as React.FormEvent;
    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(mockRecordAssetPrice).toHaveBeenCalledWith("asset-1", "2024-01-10", 50);
  });
});
