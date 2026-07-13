import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ScheduledFetchRun, ScheduledFetchStatus } from "@/bindings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockInvoke = vi.mocked(invoke);

// Import after mock is registered so bindings.ts picks up the mock
const { settingsGateway } = await import("./gateway");

// ---------------------------------------------------------------------------
// configureScheduledFetch
// ---------------------------------------------------------------------------

describe("settingsGateway — configureScheduledFetch", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // SPF-012 — ok pass-through
  it("configureScheduledFetch passes through ok result (SPF-012)", async () => {
    mockInvoke.mockResolvedValue(null);

    const result = await settingsGateway.configureScheduledFetch(true, "19:00");

    expect(result).toEqual({ status: "ok", data: null });
    expect(mockInvoke).toHaveBeenCalledWith("configure_scheduled_fetch", {
      enabled: true,
      triggerTime: "19:00",
    });
  });

  // SPF-019 — malformed trigger time rejected
  it("configureScheduledFetch passes through InvalidTriggerTime error (SPF-019)", async () => {
    mockInvoke.mockRejectedValue({ code: "InvalidTriggerTime" });

    const result = await settingsGateway.configureScheduledFetch(true, "25:99");

    expect(result).toEqual({ status: "error", error: { code: "InvalidTriggerTime" } });
  });

  // SPF-013 — OS schedule registration failure
  it("configureScheduledFetch passes through ScheduleRegistrationFailed error (SPF-013)", async () => {
    mockInvoke.mockRejectedValue({ code: "ScheduleRegistrationFailed" });

    const result = await settingsGateway.configureScheduledFetch(true, "19:00");

    expect(result).toEqual({ status: "error", error: { code: "ScheduleRegistrationFailed" } });
  });

  // SPF-013 — OS schedule removal failure (disabling)
  it("configureScheduledFetch passes through ScheduleRemovalFailed error (SPF-013)", async () => {
    mockInvoke.mockRejectedValue({ code: "ScheduleRemovalFailed" });

    const result = await settingsGateway.configureScheduledFetch(false, "19:00");

    expect(result).toEqual({ status: "error", error: { code: "ScheduleRemovalFailed" } });
  });

  // infrastructure failure
  it("configureScheduledFetch passes through DatabaseError", async () => {
    mockInvoke.mockRejectedValue({ code: "DatabaseError" });

    const result = await settingsGateway.configureScheduledFetch(true, "19:00");

    expect(result).toEqual({ status: "error", error: { code: "DatabaseError" } });
  });
});

// ---------------------------------------------------------------------------
// getScheduledFetchStatus
// ---------------------------------------------------------------------------

describe("settingsGateway — getScheduledFetchStatus", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // SPF-052 — ok pass-through, fresh install (no run yet)
  it("getScheduledFetchStatus passes through ok result with no last run (SPF-052)", async () => {
    const status: ScheduledFetchStatus = {
      configuration: { enabled: false, trigger_time: "22:15" },
      last_run: null,
    };
    mockInvoke.mockResolvedValue(status);

    const result = await settingsGateway.getScheduledFetchStatus();

    expect(result).toEqual({ status: "ok", data: status });
    expect(mockInvoke).toHaveBeenCalledWith("get_scheduled_fetch_status");
  });

  // SPF-052 — ok pass-through with a completed run
  it("getScheduledFetchStatus passes through ok result with a completed run", async () => {
    const run: ScheduledFetchRun = {
      executed_at: "2026-07-12T19:00:00Z",
      trigger_date: "2026-07-12",
      outcome: "Succeeded",
      updated_count: 12,
      skipped_count: 2,
    };
    const status: ScheduledFetchStatus = {
      configuration: { enabled: true, trigger_time: "19:00" },
      last_run: run,
    };
    mockInvoke.mockResolvedValue(status);

    const result = await settingsGateway.getScheduledFetchStatus();

    expect(result).toEqual({ status: "ok", data: status });
  });

  // infrastructure failure
  it("getScheduledFetchStatus passes through DatabaseError", async () => {
    mockInvoke.mockRejectedValue({ code: "DatabaseError" });

    const result = await settingsGateway.getScheduledFetchStatus();

    expect(result).toEqual({ status: "error", error: { code: "DatabaseError" } });
  });
});
