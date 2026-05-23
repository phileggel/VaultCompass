import { describe, expect, it } from "vitest";
import { accountMutationErrorToI18n, fetchPriceErrorToI18n } from "./presenter";

// F27 layer-3 presenter — exhaustive variant coverage across AccountCrudError
// and AccountApplicationError (the two account-BC error surfaces consumed by
// useAccounts: add/update use Crud, delete + deletion-summary use Application).
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
});

// F27 layer-3 presenter — exhaustive variant coverage for fetch-price snackbar
// dispatch. Composes AssetError + AccountApplicationError + FetchPriceTask
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
});
