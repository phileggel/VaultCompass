import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { OpenHoldingCommandError, OpenHoldingDTO, Transaction } from "@/bindings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockInvoke = vi.mocked(invoke);

// Import after mock is registered so bindings.ts picks up the mock
const { accountDetailsGateway } = await import("./gateway");

describe("accountDetailsGateway — openHolding", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // TRX-042 — happy path: openHolding calls open_holding with wrapped dto and returns Transaction
  it("openHolding returns Transaction on success", async () => {
    const dto: OpenHoldingDTO = {
      account_id: "account-1",
      asset_id: "asset-1",
      date: "2024-01-15",
      quantity: 5_000_000,
      total_cost: 500_000_000,
    };
    const mockTransaction: Transaction = {
      id: "tx-open-1",
      account_id: "account-1",
      asset_id: "asset-1",
      transaction_type: "OpeningBalance",
      date: "2024-01-15",
      quantity: 5_000_000,
      unit_price: 100_000_000,
      exchange_rate: 1_000_000,
      fees: 0,
      total_amount: 500_000_000,
      note: null,
      realized_pnl: null,
      created_at: "2024-01-15T10:00:00Z",
    };
    // bindings.ts wraps the TAURI_INVOKE result in { status: "ok", data: ... }
    mockInvoke.mockResolvedValue(mockTransaction);

    const result = await accountDetailsGateway.openHolding(dto);

    expect(result).toEqual({ status: "ok", data: mockTransaction });
    expect(mockInvoke).toHaveBeenCalledWith("open_holding", { dto });
  });

  // TRX-056 — AccountNotFound error is surfaced as { status: "error", error: { code: "AccountNotFound" } }
  it("openHolding returns AccountNotFound on unknown account", async () => {
    const dto: OpenHoldingDTO = {
      account_id: "no-such-account",
      asset_id: "asset-1",
      date: "2024-01-15",
      quantity: 1_000_000,
      total_cost: 100_000_000,
    };
    const err: OpenHoldingCommandError = { code: "AccountNotFound" };
    // bindings.ts catches the rejection and returns { status: "error", error: e }
    mockInvoke.mockRejectedValue(err);

    const result = await accountDetailsGateway.openHolding(dto);

    expect(result).toEqual({ status: "error", error: err });
    expect(mockInvoke).toHaveBeenCalledWith("open_holding", { dto });
  });

  // TRX-056 — AssetNotFound error is surfaced correctly
  it("openHolding returns AssetNotFound on unknown asset", async () => {
    const dto: OpenHoldingDTO = {
      account_id: "account-1",
      asset_id: "no-such-asset",
      date: "2024-01-15",
      quantity: 1_000_000,
      total_cost: 100_000_000,
    };
    const err: OpenHoldingCommandError = { code: "AssetNotFound" };
    mockInvoke.mockRejectedValue(err);

    const result = await accountDetailsGateway.openHolding(dto);

    expect(result).toEqual({ status: "error", error: err });
    expect(mockInvoke).toHaveBeenCalledWith("open_holding", { dto });
  });

  // TRX-050 — ArchivedAsset error is surfaced correctly
  it("openHolding returns ArchivedAsset when asset is archived", async () => {
    const dto: OpenHoldingDTO = {
      account_id: "account-1",
      asset_id: "archived-asset",
      date: "2024-01-15",
      quantity: 1_000_000,
      total_cost: 100_000_000,
    };
    const err: OpenHoldingCommandError = { code: "ArchivedAsset" };
    mockInvoke.mockRejectedValue(err);

    const result = await accountDetailsGateway.openHolding(dto);

    expect(result).toEqual({ status: "error", error: err });
  });

  // TRX-044 — QuantityNotPositive error is surfaced correctly
  it("openHolding returns QuantityNotPositive when quantity is zero or negative", async () => {
    const dto: OpenHoldingDTO = {
      account_id: "account-1",
      asset_id: "asset-1",
      date: "2024-01-15",
      quantity: 0,
      total_cost: 100_000_000,
    };
    const err: OpenHoldingCommandError = { code: "QuantityNotPositive" };
    mockInvoke.mockRejectedValue(err);

    const result = await accountDetailsGateway.openHolding(dto);

    expect(result).toEqual({ status: "error", error: err });
  });

  // TRX-045 — InvalidTotalCost error is surfaced correctly
  it("openHolding returns InvalidTotalCost when total_cost is zero or negative", async () => {
    const dto: OpenHoldingDTO = {
      account_id: "account-1",
      asset_id: "asset-1",
      date: "2024-01-15",
      quantity: 1_000_000,
      total_cost: 0,
    };
    const err: OpenHoldingCommandError = { code: "InvalidTotalCost" };
    mockInvoke.mockRejectedValue(err);

    const result = await accountDetailsGateway.openHolding(dto);

    expect(result).toEqual({ status: "error", error: err });
  });

  // TRX-046 — DateInFuture error is surfaced correctly
  it("openHolding returns DateInFuture when date is in the future", async () => {
    const dto: OpenHoldingDTO = {
      account_id: "account-1",
      asset_id: "asset-1",
      date: "2099-12-31",
      quantity: 1_000_000,
      total_cost: 100_000_000,
    };
    const err: OpenHoldingCommandError = { code: "DateInFuture" };
    mockInvoke.mockRejectedValue(err);

    const result = await accountDetailsGateway.openHolding(dto);

    expect(result).toEqual({ status: "error", error: err });
  });

  // TRX-046 — DateTooOld error is surfaced correctly
  it("openHolding returns DateTooOld when date is before 1900-01-01", async () => {
    const dto: OpenHoldingDTO = {
      account_id: "account-1",
      asset_id: "asset-1",
      date: "1899-12-31",
      quantity: 1_000_000,
      total_cost: 100_000_000,
    };
    const err: OpenHoldingCommandError = { code: "DateTooOld" };
    mockInvoke.mockRejectedValue(err);

    const result = await accountDetailsGateway.openHolding(dto);

    expect(result).toEqual({ status: "error", error: err });
  });

  // InvalidDate error is surfaced correctly
  it("openHolding returns InvalidDate when date string cannot be parsed", async () => {
    const dto: OpenHoldingDTO = {
      account_id: "account-1",
      asset_id: "asset-1",
      date: "not-a-date",
      quantity: 1_000_000,
      total_cost: 100_000_000,
    };
    const err: OpenHoldingCommandError = { code: "InvalidDate" };
    mockInvoke.mockRejectedValue(err);

    const result = await accountDetailsGateway.openHolding(dto);

    expect(result).toEqual({ status: "error", error: err });
  });
});
