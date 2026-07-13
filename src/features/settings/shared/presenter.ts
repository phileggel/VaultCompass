import type { ScheduledFetchError, ScheduledFetchRun } from "@/bindings";
import type { I18nMessage } from "@/ui/format/i18n";

/**
 * F27 — Maps a `ScheduledFetchError` to an i18n key. Pure: no React, no
 * `useTranslation`. The exhaustive switch on `code` lets TypeScript catch any
 * new variant at compile time. Keys live under the `error.scheduled_fetch.*`
 * namespace so they don't collide with the shared `error.*` codes used by
 * other bounded contexts.
 */
export function scheduledFetchErrorToI18n(error: ScheduledFetchError): I18nMessage {
  switch (error.code) {
    case "InvalidTriggerTime":
    case "ScheduleRegistrationFailed":
    case "ScheduleRemovalFailed":
    case "DatabaseError":
      return { key: `error.scheduled_fetch.${error.code}` };
    default: {
      const _exhaustive: never = error;
      return _exhaustive;
    }
  }
}

/**
 * SPF-052 — Formats the Settings section's status line from the most recent
 * run: when it ran, its outcome, and its counts. `null` (fresh install, no
 * run has ever executed) maps to a distinct "no download yet" key.
 */
export function formatScheduledFetchStatusLine(lastRun: ScheduledFetchRun | null): I18nMessage {
  if (lastRun === null) {
    return { key: "settings.scheduled_fetch.status_none" };
  }
  switch (lastRun.outcome) {
    case "Succeeded":
      return {
        key: "settings.scheduled_fetch.status_succeeded",
        vars: {
          executedAt: lastRun.executed_at,
          updatedCount: lastRun.updated_count,
          skippedCount: lastRun.skipped_count,
        },
      };
    case "Failed":
      return {
        key: "settings.scheduled_fetch.status_failed",
        vars: { executedAt: lastRun.executed_at },
      };
    case "SkippedAlreadyRun":
      return {
        key: "settings.scheduled_fetch.status_skipped",
        vars: { executedAt: lastRun.executed_at },
      };
    default: {
      const _exhaustive: never = lastRun.outcome;
      return _exhaustive;
    }
  }
}
