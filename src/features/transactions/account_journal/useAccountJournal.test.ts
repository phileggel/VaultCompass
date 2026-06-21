import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Account, Asset, Transaction } from "@/bindings";
import { useAppStore } from "@/lib/store";
import { useAccountJournal } from "./useAccountJournal";

vi.mock("@tanstack/react-router", () => ({
  useParams: () => ({ accountId: "account-1" }),
}));

const mockGetAll = vi.fn();
const mockSubscribe = vi.fn();

vi.mock("../gateway", () => ({
  transactionGateway: {
    getAllTransactionsForAccount: (...args: unknown[]) => mockGetAll(...args),
    subscribeToEvents: (...args: unknown[]) => mockSubscribe(...args),
  },
}));

vi.mock("@/lib/logger", () => ({ logger: { error: vi.fn() } }));

const MICRO = 1_000_000;

const tx = (over: Partial<Transaction>): Transaction =>
  ({
    id: "tx",
    account_id: "account-1",
    asset_id: "asset-1",
    transaction_type: "Purchase",
    date: "2024-01-01",
    quantity: MICRO,
    unit_price: MICRO,
    exchange_rate: MICRO,
    fees: 0,
    total_amount: 100 * MICRO,
    note: null,
    realized_pnl: null,
    created_at: "2024-01-01T00:00:00.000001Z",
    ...over,
  }) as Transaction;

describe("useAccountJournal", () => {
  let eventCallback: ((type: string) => void) | undefined;

  beforeEach(() => {
    vi.clearAllMocks();
    eventCallback = undefined;
    useAppStore.setState({
      assets: [
        { id: "asset-1", name: "Apple" },
        { id: "asset-2", name: "Google" },
      ] as Asset[],
      accounts: [{ id: "account-1", name: "My Account" }] as Account[],
    });
    mockGetAll.mockResolvedValue({
      status: "ok",
      data: [
        tx({ id: "a", asset_id: "asset-1", date: "2024-01-01", total_amount: 100 * MICRO }),
        tx({
          id: "b",
          asset_id: "asset-2",
          date: "2024-03-01",
          transaction_type: "Sell",
          total_amount: 500 * MICRO,
        }),
        tx({ id: "c", asset_id: "asset-1", date: "2024-02-01", total_amount: 300 * MICRO }),
      ],
    });
    mockSubscribe.mockImplementation(async (cb: (type: string) => void) => {
      eventCallback = cb;
      return () => {};
    });
  });

  it("loads all account transactions, sorted latest-first by default", async () => {
    const { result } = renderHook(() => useAccountJournal());
    await act(async () => {});
    expect(mockGetAll).toHaveBeenCalledWith("account-1");
    expect(result.current.filteredSortedRows.map((r) => r.id)).toEqual(["b", "c", "a"]);
  });

  it("filters by asset", async () => {
    const { result } = renderHook(() => useAccountJournal());
    await act(async () => {});
    act(() => result.current.setFilter("assetId", "asset-1"));
    expect(result.current.filteredSortedRows.map((r) => r.id)).toEqual(["c", "a"]);
  });

  it("filters by transaction type", async () => {
    const { result } = renderHook(() => useAccountJournal());
    await act(async () => {});
    act(() => result.current.setFilter("type", "Sell"));
    expect(result.current.filteredSortedRows.map((r) => r.id)).toEqual(["b"]);
  });

  it("filters by amount range (inclusive)", async () => {
    const { result } = renderHook(() => useAccountJournal());
    await act(async () => {});
    act(() => {
      result.current.setFilter("amountMin", "200");
      result.current.setFilter("amountMax", "400");
    });
    expect(result.current.filteredSortedRows.map((r) => r.id)).toEqual(["c"]);
  });

  it("clears all filters", async () => {
    const { result } = renderHook(() => useAccountJournal());
    await act(async () => {});
    act(() => result.current.setFilter("type", "Sell"));
    expect(result.current.filteredSortedRows).toHaveLength(1);
    act(() => result.current.clearFilters());
    expect(result.current.filteredSortedRows).toHaveLength(3);
  });

  it("re-fetches on a TransactionUpdated event", async () => {
    const { result } = renderHook(() => useAccountJournal());
    await act(async () => {});
    expect(mockGetAll).toHaveBeenCalledTimes(1);
    await act(async () => {
      eventCallback?.("TransactionUpdated");
    });
    expect(mockGetAll).toHaveBeenCalledTimes(2);
    // unrelated event is ignored
    await act(async () => {
      eventCallback?.("AssetUpdated");
    });
    expect(mockGetAll).toHaveBeenCalledTimes(2);
    expect(result.current.hasTransactions).toBe(true);
  });

  it("surfaces an i18n error when the load fails", async () => {
    mockGetAll.mockResolvedValue({ status: "error", error: { code: "DatabaseError" } });
    const { result } = renderHook(() => useAccountJournal());
    await act(async () => {});
    expect(result.current.error).not.toBeNull();
    expect(result.current.hasTransactions).toBe(false);
  });
});
