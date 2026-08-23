import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  Account,
  AccountDeletionSummary,
  AccountError,
  AccountSummary,
  CreateAccountDTO,
  UpdateAccountDTO,
} from "@/bindings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// Mock the events.event.listen surface used by subscribeToEvents — the real
// implementation is a tauri-specta __makeEvents__ wrapper around Tauri's
// runtime event system, which we don't want to bring into a Vitest run.
const mockEventListen = vi.fn<
  (cb: (e: { payload: { type: string } }) => void) => Promise<() => void>
>(() => Promise.resolve(() => {}));
vi.mock("@/bindings", async () => {
  const actual = (await vi.importActual("@/bindings")) as Record<string, unknown>;
  return {
    ...actual,
    events: { event: { listen: (cb: unknown) => mockEventListen(cb as never) } },
  };
});

const mockInvoke = vi.mocked(invoke);
const { accountGateway } = await import("./gateway");

const makeAccount = (): Account => ({
  id: "acc-1",
  name: "My Account",
  bank_name: "",
  currency: "EUR",
  update_frequency: "ManualMonth",
  management_fees_enabled: false,
});

describe("accountGateway", () => {
  beforeEach(() => vi.clearAllMocks());

  // ── getAccounts ──────────────────────────────────────────────────────────────

  it("getAccounts returns list on success", async () => {
    const accounts = [makeAccount()];
    mockInvoke.mockResolvedValue(accounts);
    const result = await accountGateway.getAccounts();
    expect(result).toEqual({ status: "ok", data: accounts });
    expect(mockInvoke).toHaveBeenCalledWith("get_accounts");
  });

  it("getAccounts surfaces DatabaseError on repo failure", async () => {
    const err: AccountError = { code: "DatabaseError" };
    mockInvoke.mockRejectedValue(err);
    const result = await accountGateway.getAccounts();
    expect(result).toEqual({ status: "error", error: err });
  });

  // ── addAccount ───────────────────────────────────────────────────────────────

  it("addAccount returns Account on success", async () => {
    const dto: CreateAccountDTO = {
      name: "New Account",
      bank_name: "",
      currency: "EUR",
      update_frequency: "ManualMonth",
      management_fees_enabled: false,
    };
    const account = makeAccount();
    mockInvoke.mockResolvedValue(account);
    const result = await accountGateway.addAccount(dto);
    expect(result).toEqual({ status: "ok", data: account });
    expect(mockInvoke).toHaveBeenCalledWith("add_account", { dto });
  });

  it("addAccount returns NameAlreadyExists error", async () => {
    const dto: CreateAccountDTO = {
      name: "Duplicate",
      bank_name: "",
      currency: "EUR",
      update_frequency: "ManualMonth",
      management_fees_enabled: false,
    };
    const err: AccountError = { code: "NameAlreadyExists" };
    mockInvoke.mockRejectedValue(err);
    const result = await accountGateway.addAccount(dto);
    expect(result).toEqual({ status: "error", error: err });
  });

  it("addAccount surfaces InvalidCurrency with currency payload", async () => {
    const dto: CreateAccountDTO = {
      name: "Test",
      bank_name: "",
      currency: "XYZ",
      update_frequency: "ManualMonth",
      management_fees_enabled: false,
    };
    const err: AccountError = { code: "InvalidCurrency", currency: "XYZ" };
    mockInvoke.mockRejectedValue(err);
    const result = await accountGateway.addAccount(dto);
    expect(result).toEqual({ status: "error", error: err });
  });

  it("addAccount surfaces NameEmpty domain error", async () => {
    const dto: CreateAccountDTO = {
      name: "  ",
      bank_name: "",
      currency: "EUR",
      update_frequency: "ManualMonth",
      management_fees_enabled: false,
    };
    const err: AccountError = { code: "NameEmpty" };
    mockInvoke.mockRejectedValue(err);
    const result = await accountGateway.addAccount(dto);
    expect(result).toEqual({ status: "error", error: err });
  });

  it("addAccount surfaces DatabaseError on repo failure", async () => {
    const dto: CreateAccountDTO = {
      name: "Test",
      bank_name: "",
      currency: "EUR",
      update_frequency: "ManualMonth",
      management_fees_enabled: false,
    };
    const err: AccountError = { code: "DatabaseError" };
    mockInvoke.mockRejectedValue(err);
    const result = await accountGateway.addAccount(dto);
    expect(result).toEqual({ status: "error", error: err });
  });

  // ── updateAccount ─────────────────────────────────────────────────────────────

  it("updateAccount returns updated Account on success", async () => {
    const dto: UpdateAccountDTO = {
      id: "acc-1",
      name: "Renamed",
      bank_name: "",
      currency: "USD",
      update_frequency: "ManualMonth",
      management_fees_enabled: false,
    };
    const account = { ...makeAccount(), name: "Renamed" };
    mockInvoke.mockResolvedValue(account);
    const result = await accountGateway.updateAccount(dto);
    expect(result).toEqual({ status: "ok", data: account });
    expect(mockInvoke).toHaveBeenCalledWith("update_account", { dto });
  });

  it("updateAccount returns error on failure", async () => {
    const dto: UpdateAccountDTO = {
      id: "acc-1",
      name: "X",
      bank_name: "",
      currency: "EUR",
      update_frequency: "ManualMonth",
      management_fees_enabled: false,
    };
    const err: AccountError = { code: "NameAlreadyExists" };
    mockInvoke.mockRejectedValue(err);
    const result = await accountGateway.updateAccount(dto);
    expect(result).toEqual({ status: "error", error: err });
  });

  // ── deleteAccount ─────────────────────────────────────────────────────────────

  it("deleteAccount returns null on success", async () => {
    mockInvoke.mockResolvedValue(null);
    const result = await accountGateway.deleteAccount("acc-1");
    expect(result).toEqual({ status: "ok", data: null });
    expect(mockInvoke).toHaveBeenCalledWith("delete_account", { id: "acc-1" });
  });

  it("deleteAccount surfaces DatabaseError on repo failure", async () => {
    const err: AccountError = { code: "DatabaseError" };
    mockInvoke.mockRejectedValue(err);
    const result = await accountGateway.deleteAccount("acc-1");
    expect(result).toEqual({ status: "error", error: err });
  });

  // ── getAccountDeletionSummary ─────────────────────────────────────────────────

  it("getAccountDeletionSummary returns summary on success", async () => {
    const summary: AccountDeletionSummary = {
      holding_count: 2,
      transaction_count: 5,
    };
    mockInvoke.mockResolvedValue(summary);
    const result = await accountGateway.getAccountDeletionSummary("acc-1");
    expect(result).toEqual({ status: "ok", data: summary });
    expect(mockInvoke).toHaveBeenCalledWith("get_account_deletion_summary", {
      accountId: "acc-1",
    });
  });

  it("getAccountDeletionSummary surfaces DatabaseError on repo failure", async () => {
    const err: AccountError = { code: "DatabaseError" };
    mockInvoke.mockRejectedValue(err);
    const result = await accountGateway.getAccountDeletionSummary("missing");
    expect(result).toEqual({ status: "error", error: err });
  });
});

// ── fetchAllAssetPrices (MKT-130) ─────────────────────────────────────────────
// The accounts gateway owns this call independently of accountDetailsGateway —
// the AccountManager refresh button belongs to the accounts feature (plan §
// "src/features/accounts/gateway.ts — DO NOT re-export from accountDetailsGateway").

describe("accountGateway — fetchAllAssetPrices (MKT-130)", () => {
  beforeEach(() => vi.clearAllMocks());

  // MKT-130 — happy path: dispatch acknowledged, returns null
  it("fetchAllAssetPrices returns null on successful dispatch", async () => {
    mockInvoke.mockResolvedValue(null);
    const result = await accountGateway.fetchAllAssetPrices();
    expect(result).toEqual({ status: "ok", data: null });
    expect(mockInvoke).toHaveBeenCalledWith("fetch_all_asset_prices");
  });

  // MKT-113 — in-flight guard
  it("fetchAllAssetPrices surfaces FetchAlreadyRunning when another fetch is in progress", async () => {
    const error = { code: "FetchAlreadyRunning" };
    mockInvoke.mockRejectedValue(error);
    const result = await accountGateway.fetchAllAssetPrices();
    expect(result).toEqual({ status: "error", error });
  });

  // MKT-111 — no fetchable holdings in scope
  it("fetchAllAssetPrices surfaces NoFetchableHoldings when no active holdings are derivable", async () => {
    const error = { code: "NoFetchableHoldings" };
    mockInvoke.mockRejectedValue(error);
    const result = await accountGateway.fetchAllAssetPrices();
    expect(result).toEqual({ status: "error", error });
  });

  // DatabaseError from asset BC
  it("fetchAllAssetPrices surfaces DatabaseError on infrastructure failure", async () => {
    const error = { code: "DatabaseError" };
    mockInvoke.mockRejectedValue(error);
    const result = await accountGateway.fetchAllAssetPrices();
    expect(result).toEqual({ status: "error", error });
  });

  // UnknownError catch-all
  it("fetchAllAssetPrices surfaces UnknownError on unexpected runtime failure", async () => {
    const error = { code: "UnknownError" };
    mockInvoke.mockRejectedValue(error);
    const result = await accountGateway.fetchAllAssetPrices();
    expect(result).toEqual({ status: "error", error });
  });

  // ── getAccountSummaries (ACC-021) ────────────────────────────────────────────

  it("getAccountSummaries returns the enriched list on success", async () => {
    const summaries: AccountSummary[] = [
      {
        id: "acc-1",
        name: "Main",
        currency: "EUR",
        update_frequency: "ManualMonth",
        total_global_value: 470_000_000,
        total_unrealized_pnl: null,
        ytd_performance_pct: null,
        has_inconsistent_holding: false,
      },
    ];
    mockInvoke.mockResolvedValue(summaries);
    const result = await accountGateway.getAccountSummaries();
    expect(result).toEqual({ status: "ok", data: summaries });
    expect(mockInvoke).toHaveBeenCalledWith("get_account_summaries");
  });

  it("getAccountSummaries surfaces DatabaseError when the use case fails", async () => {
    const err: AccountError = { code: "DatabaseError" };
    mockInvoke.mockRejectedValue(err);
    const result = await accountGateway.getAccountSummaries();
    expect(result).toEqual({ status: "error", error: err });
  });

  // ── subscribeToEvents ────────────────────────────────────────────────────────

  it("subscribeToEvents forwards the event payload type to the caller", async () => {
    type Listener = (e: { payload: { type: string } }) => void;
    const captured: { current: Listener | null } = { current: null };
    const unlisten = vi.fn();
    mockEventListen.mockImplementation((cb: Listener) => {
      captured.current = cb;
      return Promise.resolve(unlisten);
    });
    const callback = vi.fn();

    const result = await accountGateway.subscribeToEvents(callback);

    expect(mockEventListen).toHaveBeenCalledTimes(1);
    // Simulate the underlying event firing — the gateway adapter strips the
    // envelope and forwards only the `payload.type` string.
    captured.current?.({ payload: { type: "AccountUpdated" } });
    expect(callback).toHaveBeenCalledWith("AccountUpdated");
    expect(result).toBe(unlisten);
  });
});
