import { describe, expect, it } from "vitest";
import { accountMutationErrorToI18n } from "./presenter";

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
