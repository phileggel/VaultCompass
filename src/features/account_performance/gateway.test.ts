import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  AccountApplicationError,
  AccountPerformanceResponse,
  PerformancePeriod,
} from "@/bindings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockInvoke = vi.mocked(invoke);

// Import after mock is registered so bindings.ts picks up the mock
const { accountPerformanceGateway } = await import("./gateway");

const makeYearRow = (overrides: Partial<PerformancePeriod> = {}): PerformancePeriod => ({
  year: 2025,
  month: null,
  end_value: 10_000_000_000,
  period_over_period: { gain: 500_000_000, pct: 5_000_000 },
  year_to_date: null,
  since_inception: { gain: 500_000_000, pct: 5_000_000 },
  ...overrides,
});

const makeMonthRow = (overrides: Partial<PerformancePeriod> = {}): PerformancePeriod => ({
  year: 2025,
  month: 5,
  end_value: 10_000_000_000,
  period_over_period: { gain: 200_000_000, pct: 2_000_000 },
  year_to_date: { gain: 350_000_000, pct: 3_500_000 },
  since_inception: { gain: 500_000_000, pct: 5_000_000 },
  ...overrides,
});

const makeResponse = (
  overrides: Partial<AccountPerformanceResponse> = {},
): AccountPerformanceResponse => ({
  account_name: "My Portfolio",
  currency: "EUR",
  month_view_available: true,
  yearly: [makeYearRow()],
  monthly: [makeMonthRow()],
  ...overrides,
});

describe("accountPerformanceGateway — getAccountPerformance", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // PRF-016 happy path — gateway passes the ok result through unchanged (F27)
  it("passes through ok result with full AccountPerformanceResponse", async () => {
    const response = makeResponse();
    mockInvoke.mockResolvedValue(response);

    const result = await accountPerformanceGateway.getAccountPerformance("account-1");

    expect(result.status).toBe("ok");
    if (result.status === "ok") {
      expect(result.data.account_name).toBe("My Portfolio");
      expect(result.data.currency).toBe("EUR");
      expect(result.data.month_view_available).toBe(true);
      expect(result.data.yearly).toHaveLength(1);
      expect(result.data.monthly).toHaveLength(1);
    }
    expect(mockInvoke).toHaveBeenCalledWith("get_account_performance", {
      accountId: "account-1",
    });
  });

  // F27 — gateway does NOT throw; it returns the error result unchanged
  // PRF-016 — AccountNotFound when account_id does not correspond to an existing account
  it("passes through AccountNotFound error result (PRF-016)", async () => {
    const err: AccountApplicationError = {
      code: "AccountNotFound",
      account_id: "no-such-account",
    };
    mockInvoke.mockRejectedValue(err);

    const result = await accountPerformanceGateway.getAccountPerformance("no-such-account");

    expect(result.status).toBe("error");
    if (result.status === "error") {
      expect(result.error.code).toBe("AccountNotFound");
    }
    expect(mockInvoke).toHaveBeenCalledWith("get_account_performance", {
      accountId: "no-such-account",
    });
  });

  // PRF-027 — DatabaseError when the read fails during computation
  it("passes through DatabaseError result (PRF-027)", async () => {
    const err: AccountApplicationError = { code: "DatabaseError" };
    mockInvoke.mockRejectedValue(err);

    const result = await accountPerformanceGateway.getAccountPerformance("account-1");

    expect(result.status).toBe("error");
    if (result.status === "error") {
      expect(result.error.code).toBe("DatabaseError");
    }
  });

  // PRF-043 — empty result (no transactions) is an ok result with empty arrays
  it("passes through ok result with empty yearly and monthly arrays (PRF-043)", async () => {
    const response = makeResponse({ yearly: [], monthly: [] });
    mockInvoke.mockResolvedValue(response);

    const result = await accountPerformanceGateway.getAccountPerformance("account-1");

    expect(result.status).toBe("ok");
    if (result.status === "ok") {
      expect(result.data.yearly).toHaveLength(0);
      expect(result.data.monthly).toHaveLength(0);
    }
  });

  // PRF-013 — month_view_available=false passes through unchanged
  it("passes through month_view_available=false for ManualMonth/ManualYear accounts (PRF-013)", async () => {
    const response = makeResponse({ month_view_available: false, monthly: [] });
    mockInvoke.mockResolvedValue(response);

    const result = await accountPerformanceGateway.getAccountPerformance("account-1");

    expect(result.status).toBe("ok");
    if (result.status === "ok") {
      expect(result.data.month_view_available).toBe(false);
      expect(result.data.monthly).toHaveLength(0);
    }
  });
});
