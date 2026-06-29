import { describe, expect, it } from "vitest";
import {
  accountMutationErrorToI18n,
  fetchPriceErrorToI18n,
  formatAccountRowTotalUnrealizedPnl,
  formatAccountRowYtdPerformancePct,
} from "./presenter";

// F27 layer-3 presenter — covers the reachable `AccountError` codes for the
// account-BC mutation surface (add / update / delete / deletion-summary), all
// consumed by useAccounts.
describe("accountMutationErrorToI18n", () => {
  it("InvalidCurrency interpolates the currency payload", () => {
    expect(accountMutationErrorToI18n({ code: "InvalidCurrency", currency: "ZZZ" })).toEqual({
      key: "error.InvalidCurrency",
      vars: { currency: "ZZZ" },
    });
  });

  it("AccountNotFound maps to its flat key (account_id payload not surfaced)", () => {
    expect(accountMutationErrorToI18n({ code: "AccountNotFound", account_id: "acc-1" })).toEqual({
      key: "error.AccountNotFound",
    });
  });

  it.each([
    "NameEmpty",
    "NameAlreadyExists",
    "DatabaseError",
  ] as const)("%s unit variant maps to its flat error key", (code) => {
    expect(accountMutationErrorToI18n({ code })).toEqual({ key: `error.${code}` });
  });

  it("an unreachable BC-wide code falls back to error.Unknown", () => {
    expect(accountMutationErrorToI18n({ code: "Oversell", available: 1, requested: 2 })).toEqual({
      key: "error.Unknown",
    });
  });
});

// F27 layer-3 presenter — exhaustive variant coverage for fetch-price snackbar
// dispatch. Composes AssetError + AccountError + FetchPriceTask
// (the AssetError contribution is the same DatabaseError code as the account-side).
describe("fetchPriceErrorToI18n", () => {
  it("FetchAlreadyRunning dispatches info snackbar", () => {
    expect(fetchPriceErrorToI18n({ code: "FetchAlreadyRunning" })).toEqual({
      key: "mkt.fetch_already_running",
      severity: "info",
    });
  });

  it("NoFetchableHoldings dispatches info snackbar", () => {
    expect(fetchPriceErrorToI18n({ code: "NoFetchableHoldings" })).toEqual({
      key: "mkt.fetch_no_holdings",
      severity: "info",
    });
  });

  it("AccountNotFound dispatches dedicated error snackbar", () => {
    expect(fetchPriceErrorToI18n({ code: "AccountNotFound", account_id: "acc-1" })).toEqual({
      key: "error.AccountNotFound",
      severity: "error",
    });
  });

  it.each([
    "DatabaseError",
    "UnknownError",
    "NameAlreadyExists",
  ] as const)("%s falls through to the generic DatabaseError snackbar", (code) => {
    expect(fetchPriceErrorToI18n({ code })).toEqual({
      key: "error.DatabaseError",
      severity: "error",
    });
  });

  it("an unreachable BC-wide code falls back to the generic error.Unknown snackbar", () => {
    expect(fetchPriceErrorToI18n({ code: "Oversell", available: 1, requested: 2 })).toEqual({
      key: "error.Unknown",
      severity: "error",
    });
  });
});

// ACC-023 — formatAccountRowTotalUnrealizedPnl: account-currency micros → formatted string
// or "—" when null. Mirrors the HoldingRowViewModel.unrealizedPnl pattern from
// account_details/shared/presenter.ts (microToFormatted with 2 decimals).
describe("formatAccountRowTotalUnrealizedPnl", () => {
  it("returns '—' when total_unrealized_pnl is null", () => {
    expect(formatAccountRowTotalUnrealizedPnl(null)).toBe("—");
  });

  it("formats a positive value (micros) to 2 decimal places", () => {
    // 1_250_000 micros = 1.25 in account currency
    expect(formatAccountRowTotalUnrealizedPnl(1_250_000)).toBe("1,25");
  });

  it("formats a negative value (micros) to 2 decimal places with leading minus", () => {
    // -3_700_000 micros = -3.70 in account currency
    expect(formatAccountRowTotalUnrealizedPnl(-3_700_000)).toBe("-3,70");
  });

  it("formats zero as '0.00'", () => {
    expect(formatAccountRowTotalUnrealizedPnl(0)).toBe("0,00");
  });
});

// ACC-024 — formatAccountRowYtdPerformancePct: micro-percent → signed formatted string
// or "—" when null. 8_000_000 micro-percent = 8.00%, with explicit '+' for positives.
describe("formatAccountRowYtdPerformancePct", () => {
  it("returns '—' when ytd_performance_pct is null", () => {
    expect(formatAccountRowYtdPerformancePct(null)).toBe("—");
  });

  it("formats a positive micro-percent with a leading '+' sign", () => {
    // 8_000_000 micro-percent = 8.00%
    expect(formatAccountRowYtdPerformancePct(8_000_000)).toBe("+8,00%");
  });

  it("formats a negative micro-percent with a leading '-' sign (no explicit '+')", () => {
    // -3_700_000 micro-percent = -3.70%
    expect(formatAccountRowYtdPerformancePct(-3_700_000)).toBe("-3,70%");
  });

  it("formats zero as '+0.00%' (non-negative, treated as positive sign)", () => {
    expect(formatAccountRowYtdPerformancePct(0)).toBe("+0,00%");
  });
});
