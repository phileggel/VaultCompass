import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useAddTransaction } from "./useAddTransaction";

const mockAddTransaction = vi.fn();

vi.mock("../useTransactions", () => ({
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

const fakeSubmit = { preventDefault: vi.fn() } as unknown as React.FormEvent;

describe("useAddTransaction", () => {
  beforeEach(() => {
    mockAddTransaction.mockReset();
  });

  // TRX-011 — pre-fill assetId from props
  it("pre-fills assetId from prefillAssetId prop", () => {
    const { result } = renderHook(() => useAddTransaction({ prefillAssetId: "asset-1" }));
    expect(result.current.formData.assetId).toBe("asset-1");
  });

  // TRX-011 — pre-fill accountId from props
  it("pre-fills accountId from prefillAccountId prop", () => {
    const { result } = renderHook(() => useAddTransaction({ prefillAccountId: "account-1" }));
    expect(result.current.formData.accountId).toBe("account-1");
  });

  // TRX-026 — totalAmountDisplay is derived from micro values when quantity changes
  it("recalculates totalAmountDisplay when quantity changes", async () => {
    const { result } = renderHook(() => useAddTransaction());

    await act(async () => {
      result.current.handleChange("unitPrice", "100");
      result.current.handleChange("exchangeRate", "1");
      result.current.handleChange("fees", "0");
      result.current.handleChange("quantity", "2");
    });

    // 2 * 100 * 1 + 0 = 200.000
    expect(result.current.totalAmountDisplay).toBe("200.000");
  });

  // TRX-029 — archived asset triggers confirmation dialog on submit
  it("sets showArchivedConfirm when submitting with an archived asset", async () => {
    const { result } = renderHook(() => useAddTransaction({ prefillAccountId: "account-1" }));

    await act(async () => {
      result.current.handleChange("assetId", "asset-archived");
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.showArchivedConfirm).toBe(true);
  });

  // TRX-029 — cancelling archived confirmation does not submit
  it("handleCancelArchived resets showArchivedConfirm without submitting", async () => {
    const { result } = renderHook(() => useAddTransaction({ prefillAccountId: "account-1" }));

    await act(async () => {
      result.current.handleChange("assetId", "asset-archived");
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });
    expect(result.current.showArchivedConfirm).toBe(true);

    await act(async () => {
      result.current.handleCancelArchived();
    });

    expect(result.current.showArchivedConfirm).toBe(false);
    expect(mockAddTransaction).not.toHaveBeenCalled();
  });

  // TRX-020 — validation: empty accountId blocks submit
  it("sets error and does not submit when accountId is empty", async () => {
    const onSubmitSuccess = vi.fn();
    const { result } = renderHook(() => useAddTransaction({ onSubmitSuccess }));

    await act(async () => {
      result.current.handleChange("assetId", "asset-1");
      result.current.handleChange("date", "2024-01-15");
      result.current.handleChange("quantity", "1");
      result.current.handleChange("unitPrice", "10");
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).not.toBeNull();
    expect(mockAddTransaction).not.toHaveBeenCalled();
    expect(onSubmitSuccess).not.toHaveBeenCalled();
  });

  // Backend error keeps modal open and exposes error
  it("sets error and does not call onSubmitSuccess on backend error", async () => {
    mockAddTransaction.mockResolvedValue({ data: null, error: "Invariant mismatch" });
    const onSubmitSuccess = vi.fn();
    const { result } = renderHook(() =>
      useAddTransaction({
        prefillAccountId: "account-1",
        prefillAssetId: "asset-1",
        onSubmitSuccess,
      }),
    );

    await act(async () => {
      result.current.handleChange("date", "2024-01-15");
      result.current.handleChange("quantity", "1");
      result.current.handleChange("unitPrice", "10");
      result.current.handleChange("exchangeRate", "1");
      result.current.handleChange("fees", "0");
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).toBe("Invariant mismatch");
    expect(onSubmitSuccess).not.toHaveBeenCalled();
  });

  // Success calls onSubmitSuccess
  it("calls onSubmitSuccess on success", async () => {
    mockAddTransaction.mockResolvedValue({
      data: { id: "tx-1" },
      error: null,
    });
    const onSubmitSuccess = vi.fn();
    const { result } = renderHook(() =>
      useAddTransaction({
        prefillAccountId: "account-1",
        prefillAssetId: "asset-1",
        onSubmitSuccess,
      }),
    );

    await act(async () => {
      result.current.handleChange("date", "2024-01-15");
      result.current.handleChange("quantity", "1");
      result.current.handleChange("unitPrice", "10");
      result.current.handleChange("exchangeRate", "1");
      result.current.handleChange("fees", "0");
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(result.current.error).toBeNull();
    expect(onSubmitSuccess).toHaveBeenCalledTimes(1);
  });

  // handleSubmit with archived asset → does not call addTransaction (waits for confirmation)
  it("handleSubmit with archived asset does not submit immediately", async () => {
    const onSubmitSuccess = vi.fn();
    const { result } = renderHook(() =>
      useAddTransaction({ prefillAccountId: "account-1", onSubmitSuccess }),
    );

    await act(async () => {
      result.current.handleChange("assetId", "asset-archived");
    });

    await act(async () => {
      await result.current.handleSubmit(fakeSubmit);
    });

    expect(mockAddTransaction).not.toHaveBeenCalled();
    expect(onSubmitSuccess).not.toHaveBeenCalled();
  });
});
