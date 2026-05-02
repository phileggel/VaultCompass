import { renderHook } from "@testing-library/react";
import { act } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { BuyHoldingDTO, CorrectTransactionDTO, SellHoldingDTO, Transaction } from "@/bindings";

const {
  mockBuyHolding,
  mockSellHolding,
  mockCorrectTransaction,
  mockCancelTransaction,
  mockGetTransactions,
} = vi.hoisted(() => ({
  mockBuyHolding: vi.fn(),
  mockSellHolding: vi.fn(),
  mockCorrectTransaction: vi.fn(),
  mockCancelTransaction: vi.fn(),
  mockGetTransactions: vi.fn(),
}));

vi.mock("./gateway", () => ({
  transactionGateway: {
    buyHolding: mockBuyHolding,
    sellHolding: mockSellHolding,
    correctTransaction: mockCorrectTransaction,
    cancelTransaction: mockCancelTransaction,
    getTransactions: mockGetTransactions,
  },
}));

vi.mock("@/lib/logger", () => ({
  logger: { error: vi.fn(), info: vi.fn() },
}));

const { useTransactions } = await import("./useTransactions");

const makeTx = (): Transaction => ({
  id: "tx-1",
  account_id: "acc-1",
  asset_id: "asset-1",
  transaction_type: "Purchase",
  date: "2024-01-15",
  quantity: 1_000_000,
  unit_price: 100_000_000,
  exchange_rate: 1_000_000,
  fees: 0,
  total_amount: 100_000_000,
  note: null,
  realized_pnl: null,
  created_at: "2024-01-15T10:00:00Z",
});

const buyDto: BuyHoldingDTO = {
  account_id: "acc-1",
  asset_id: "asset-1",
  date: "2024-01-15",
  quantity: 1_000_000,
  unit_price: 100_000_000,
  exchange_rate: 1_000_000,
  fees: 0,
  note: null,
};

describe("useTransactions", () => {
  beforeEach(() => {
    mockBuyHolding.mockReset();
    mockSellHolding.mockReset();
    mockCorrectTransaction.mockReset();
    mockCancelTransaction.mockReset();
    mockGetTransactions.mockReset();
  });

  // ── buyHolding ────────────────────────────────────────────────────────────────

  it("buyHolding returns data on success", async () => {
    const tx = makeTx();
    mockBuyHolding.mockResolvedValue({ status: "ok", data: tx });
    const { result } = renderHook(() => useTransactions());
    let ret: { data: Transaction | null; error: string | null } = { data: null, error: null };
    await act(async () => {
      ret = await result.current.buyHolding(buyDto);
    });
    expect(mockBuyHolding).toHaveBeenCalledWith(buyDto);
    expect(ret.data).toEqual(tx);
    expect(ret.error).toBeNull();
  });

  it("buyHolding returns error code on failure", async () => {
    mockBuyHolding.mockResolvedValue({ status: "error", error: { code: "AccountNotFound" } });
    const { result } = renderHook(() => useTransactions());
    let ret: { data: Transaction | null; error: string | null } = { data: null, error: null };
    await act(async () => {
      ret = await result.current.buyHolding(buyDto);
    });
    expect(ret.error).toBe("error.AccountNotFound");
  });

  // ── sellHolding ───────────────────────────────────────────────────────────────

  it("sellHolding returns data on success", async () => {
    const tx = makeTx();
    mockSellHolding.mockResolvedValue({ status: "ok", data: tx });
    const { result } = renderHook(() => useTransactions());
    const sellDto: SellHoldingDTO = {
      account_id: "acc-1",
      asset_id: "asset-1",
      date: "2024-02-01",
      quantity: 500_000,
      unit_price: 110_000_000,
      exchange_rate: 1_000_000,
      fees: 0,
      note: null,
    };
    let ret: { data: Transaction | null; error: string | null } = { data: null, error: null };
    await act(async () => {
      ret = await result.current.sellHolding(sellDto);
    });
    expect(ret.data).toEqual(tx);
    expect(ret.error).toBeNull();
  });

  it("sellHolding returns Oversell error code on failure", async () => {
    mockSellHolding.mockResolvedValue({
      status: "error",
      error: { code: "Oversell", available: 500_000, requested: 999_000_000 },
    });
    const { result } = renderHook(() => useTransactions());
    let ret: { data: Transaction | null; error: string | null } = { data: null, error: null };
    await act(async () => {
      ret = await result.current.sellHolding({
        account_id: "acc-1",
        asset_id: "asset-1",
        date: "2024-02-01",
        quantity: 999_000_000,
        unit_price: 100_000_000,
        exchange_rate: 1_000_000,
        fees: 0,
        note: null,
      });
    });
    expect(ret.error).toBe("error.Oversell");
  });

  // ── correctTransaction ────────────────────────────────────────────────────────

  it("correctTransaction returns data on success", async () => {
    const tx = makeTx();
    mockCorrectTransaction.mockResolvedValue({ status: "ok", data: tx });
    const { result } = renderHook(() => useTransactions());
    const dto: CorrectTransactionDTO = {
      date: "2024-01-20",
      quantity: 2_000_000,
      unit_price: 90_000_000,
      exchange_rate: 1_000_000,
      fees: 0,
      note: null,
    };
    let ret: { data: Transaction | null; error: string | null } = { data: null, error: null };
    await act(async () => {
      ret = await result.current.correctTransaction("tx-1", "acc-1", dto);
    });
    expect(mockCorrectTransaction).toHaveBeenCalledWith("tx-1", "acc-1", dto);
    expect(ret.data).toEqual(tx);
  });

  // ── cancelTransaction ─────────────────────────────────────────────────────────

  it("cancelTransaction returns null error on success", async () => {
    mockCancelTransaction.mockResolvedValue({ status: "ok", data: null });
    const { result } = renderHook(() => useTransactions());
    let ret: { error: string | null } = { error: "sentinel" };
    await act(async () => {
      ret = await result.current.cancelTransaction("tx-1", "acc-1");
    });
    expect(mockCancelTransaction).toHaveBeenCalledWith("tx-1", "acc-1");
    expect(ret.error).toBeNull();
  });

  // ── getTransactions ───────────────────────────────────────────────────────────

  it("getTransactions returns list on success", async () => {
    const txList = [makeTx()];
    mockGetTransactions.mockResolvedValue({ status: "ok", data: txList });
    const { result } = renderHook(() => useTransactions());
    let ret: Transaction[] = [];
    await act(async () => {
      ret = await result.current.getTransactions("acc-1", "asset-1");
    });
    expect(mockGetTransactions).toHaveBeenCalledWith("acc-1", "asset-1");
    expect(ret).toEqual(txList);
  });

  it("getTransactions returns empty array on error", async () => {
    mockGetTransactions.mockResolvedValue({ status: "error", error: { code: "Unknown" } });
    const { result } = renderHook(() => useTransactions());
    let ret: Transaction[] = [makeTx()];
    await act(async () => {
      ret = await result.current.getTransactions("acc-1", "asset-1");
    });
    expect(ret).toEqual([]);
  });
});
