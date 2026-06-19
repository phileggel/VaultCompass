import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AccountSummary } from "@/bindings";
import { logger } from "@/lib/logger";
import { useAccountSummaries } from "./useAccountSummaries";

const mockGetAccountSummaries = vi.fn();
const mockSubscribeToEvents = vi.fn<(cb: (type: string) => void) => Promise<() => void>>(() =>
  Promise.resolve(() => {}),
);

vi.mock("./gateway", () => ({
  accountGateway: {
    getAccountSummaries: () => mockGetAccountSummaries(),
    subscribeToEvents: (cb: (type: string) => void) => mockSubscribeToEvents(cb),
  },
}));

vi.mock("@/lib/logger", () => ({
  logger: { error: vi.fn(), info: vi.fn() },
}));

const makeSummary = (overrides: Partial<AccountSummary> = {}): AccountSummary => ({
  id: "acc-1",
  name: "Main",
  currency: "EUR",
  update_frequency: "ManualMonth",
  total_global_value: 100_000_000,
  total_unrealized_pnl: null,
  ytd_performance_pct: null,
  ...overrides,
});

describe("useAccountSummaries", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockSubscribeToEvents.mockImplementation(() => Promise.resolve(() => {}));
  });

  // ACC-021 — happy path: gateway returns list → summaries state populated, isLoading cleared
  it("populates summaries from gateway result on mount", async () => {
    const summaries = [makeSummary(), makeSummary({ id: "acc-2", name: "Side" })];
    mockGetAccountSummaries.mockResolvedValue({ status: "ok", data: summaries });

    const { result } = renderHook(() => useAccountSummaries());
    await act(async () => {});

    expect(result.current.summaries).toEqual(summaries);
    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  // Typed-error path — DatabaseError surfaces via presenter
  it("maps backend DatabaseError to error.DatabaseError and clears isLoading", async () => {
    mockGetAccountSummaries.mockResolvedValue({
      status: "error",
      error: { code: "DatabaseError" },
    });

    const { result } = renderHook(() => useAccountSummaries());
    await act(async () => {});

    expect(result.current.error).toEqual({ key: "error.DatabaseError" });
    expect(result.current.isLoading).toBe(false);
    expect(result.current.summaries).toEqual([]);
  });

  // Throw path — gateway rejection falls back to UNKNOWN_ERROR
  it("falls back to UNKNOWN_ERROR when gateway throws", async () => {
    mockGetAccountSummaries.mockRejectedValue(new Error("boom"));

    const { result } = renderHook(() => useAccountSummaries());
    await act(async () => {});

    expect(result.current.error).toEqual({ key: "error.Unknown" });
    expect(result.current.isLoading).toBe(false);
    expect(logger.error).toHaveBeenCalledWith("[useAccountSummaries] fetch threw", {
      error: expect.any(Error),
    });
  });

  // ACC-021 — re-fetches when AccountUpdated / AssetPriceUpdated events fire
  it("re-fetches summaries when relevant events arrive", async () => {
    let capturedCallback: ((type: string) => void) | null = null;
    mockSubscribeToEvents.mockImplementation((cb: (type: string) => void) => {
      capturedCallback = cb;
      return Promise.resolve(() => {});
    });
    mockGetAccountSummaries.mockResolvedValue({ status: "ok", data: [] });

    renderHook(() => useAccountSummaries());
    await act(async () => {});
    const beforeCount = mockGetAccountSummaries.mock.calls.length;

    await act(async () => {
      capturedCallback?.("AssetPriceUpdated");
    });

    expect(mockGetAccountSummaries.mock.calls.length).toBeGreaterThan(beforeCount);
  });

  // Unrelated events do NOT trigger a re-fetch (cheap noise filter)
  it("ignores unrelated event types", async () => {
    let capturedCallback: ((type: string) => void) | null = null;
    mockSubscribeToEvents.mockImplementation((cb: (type: string) => void) => {
      capturedCallback = cb;
      return Promise.resolve(() => {});
    });
    mockGetAccountSummaries.mockResolvedValue({ status: "ok", data: [] });

    renderHook(() => useAccountSummaries());
    await act(async () => {});
    const beforeCount = mockGetAccountSummaries.mock.calls.length;

    await act(async () => {
      capturedCallback?.("SomethingUnrelated");
    });

    expect(mockGetAccountSummaries.mock.calls.length).toBe(beforeCount);
  });
});
