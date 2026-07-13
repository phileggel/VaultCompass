import { describe, expect, it } from "vitest";
import type { ScheduledFetchError, ScheduledFetchRun } from "@/bindings";
import { formatScheduledFetchStatusLine, scheduledFetchErrorToI18n } from "./presenter";

// ---------------------------------------------------------------------------
// scheduledFetchErrorToI18n — F27 error → i18n key (one test per ScheduledFetchError variant)
// ---------------------------------------------------------------------------

describe("scheduledFetchErrorToI18n", () => {
  // SPF-019 — malformed trigger time
  it("maps InvalidTriggerTime to its flat i18n key (SPF-019)", () => {
    const error: ScheduledFetchError = { code: "InvalidTriggerTime" };
    expect(scheduledFetchErrorToI18n(error)).toEqual({
      key: "error.scheduled_fetch.InvalidTriggerTime",
    });
  });

  // SPF-013 — OS schedule registration failure
  it("maps ScheduleRegistrationFailed to its flat i18n key (SPF-013)", () => {
    const error: ScheduledFetchError = { code: "ScheduleRegistrationFailed" };
    expect(scheduledFetchErrorToI18n(error)).toEqual({
      key: "error.scheduled_fetch.ScheduleRegistrationFailed",
    });
  });

  // SPF-013 — OS schedule removal failure
  it("maps ScheduleRemovalFailed to its flat i18n key (SPF-013)", () => {
    const error: ScheduledFetchError = { code: "ScheduleRemovalFailed" };
    expect(scheduledFetchErrorToI18n(error)).toEqual({
      key: "error.scheduled_fetch.ScheduleRemovalFailed",
    });
  });

  // infrastructure failure
  it("maps DatabaseError to its flat i18n key", () => {
    const error: ScheduledFetchError = { code: "DatabaseError" };
    expect(scheduledFetchErrorToI18n(error)).toEqual({
      key: "error.scheduled_fetch.DatabaseError",
    });
  });
});

// ---------------------------------------------------------------------------
// formatScheduledFetchStatusLine — SPF-052: "when it ran, outcome, counts"
// ---------------------------------------------------------------------------

describe("formatScheduledFetchStatusLine (SPF-052)", () => {
  // Fresh install — no run has ever executed
  it("returns the no-download-yet key when lastRun is null", () => {
    expect(formatScheduledFetchStatusLine(null)).toEqual({
      key: "settings.scheduled_fetch.status_none",
    });
  });

  // Succeeded outcome carries executedAt + both counts
  it("returns the succeeded key with executedAt/updatedCount/skippedCount for a successful run", () => {
    const run: ScheduledFetchRun = {
      executed_at: "2026-07-12T19:00:00Z",
      trigger_date: "2026-07-12",
      outcome: "Succeeded",
      updated_count: 12,
      skipped_count: 2,
    };
    expect(formatScheduledFetchStatusLine(run)).toEqual({
      key: "settings.scheduled_fetch.status_succeeded",
      vars: { executedAt: "2026-07-12T19:00:00Z", updatedCount: 12, skippedCount: 2 },
    });
  });

  // Failed outcome — shown inline here only, no popup (SPF-052)
  it("returns the failed key with executedAt for a failed run", () => {
    const run: ScheduledFetchRun = {
      executed_at: "2026-07-12T19:00:00Z",
      trigger_date: "2026-07-12",
      outcome: "Failed",
      updated_count: 0,
      skipped_count: 0,
    };
    expect(formatScheduledFetchStatusLine(run)).toEqual({
      key: "settings.scheduled_fetch.status_failed",
      vars: { executedAt: "2026-07-12T19:00:00Z" },
    });
  });

  // SkippedAlreadyRun outcome — once-per-day guard exit
  it("returns the skipped key with executedAt for a guard-skipped run", () => {
    const run: ScheduledFetchRun = {
      executed_at: "2026-07-12T19:00:00Z",
      trigger_date: "2026-07-12",
      outcome: "SkippedAlreadyRun",
      updated_count: 0,
      skipped_count: 0,
    };
    expect(formatScheduledFetchStatusLine(run)).toEqual({
      key: "settings.scheduled_fetch.status_skipped",
      vars: { executedAt: "2026-07-12T19:00:00Z" },
    });
  });
});
