import { describe, expect, it } from "vitest";
import { toTransactionRow, transactionMutationErrorToI18n } from "./presenter";

const MICRO = 1_000_000;

const openingBalanceTx = {
  id: "tx-ob",
  account_id: "account-1",
  asset_id: "asset-1",
  transaction_type: "OpeningBalance" as const,
  date: "2024-01-10",
  quantity: 2 * MICRO,
  unit_price: 50 * MICRO, // stored computed value (TRX-047: total_cost / quantity)
  exchange_rate: 1 * MICRO,
  fees: 0,
  total_amount: 100 * MICRO,
  note: null,
  realized_pnl: null,
  created_at: "2024-01-10T00:00:00Z",
};

describe("toTransactionRow — OpeningBalance", () => {
  // TRX-052: type field carries "OpeningBalance" so the component can build the i18n key
  it("TRX-052: type is 'OpeningBalance' for i18n key construction", () => {
    const row = toTransactionRow(openingBalanceTx, "Apple", "My Account");
    expect(row.type).toBe("OpeningBalance");
  });

  // TRX-053: unit price column shows the stored computed unit_price (total_cost / quantity)
  it("TRX-053: unitPrice shows stored unit_price value", () => {
    const row = toTransactionRow(openingBalanceTx, "Apple", "My Account");
    expect(row.unitPrice).toBe("50,000");
  });

  // TRX-054: realized P&L is null — no P&L event for opening balance entries
  it("TRX-054: realizedPnl and realizedPnlRaw are null", () => {
    const row = toTransactionRow(openingBalanceTx, "Apple", "My Account");
    expect(row.realizedPnl).toBeNull();
    expect(row.realizedPnlRaw).toBeNull();
  });
});

describe("toTransactionRow — cash transactions (CSH-101)", () => {
  const depositTx = {
    id: "tx-dep",
    account_id: "account-1",
    asset_id: "system-cash-eur",
    transaction_type: "Deposit" as const,
    date: "2025-06-15",
    quantity: 250 * MICRO,
    unit_price: 1 * MICRO,
    exchange_rate: 1 * MICRO,
    fees: 0,
    total_amount: 250 * MICRO,
    note: null,
    realized_pnl: null,
    created_at: "2025-06-15T10:00:00Z",
  };
  const withdrawalTx = { ...depositTx, transaction_type: "Withdrawal" as const, id: "tx-wd" };

  // CSH-101 / TXL-023 — Deposit type round-trips for the i18n key
  it("Deposit type label is 'Deposit'", () => {
    const row = toTransactionRow(depositTx, "Cash EUR", "My Account");
    expect(row.type).toBe("Deposit");
  });

  // CSH-101 / TXL-023 — Withdrawal type round-trips for the i18n key
  it("Withdrawal type label is 'Withdrawal'", () => {
    const row = toTransactionRow(withdrawalTx, "Cash EUR", "My Account");
    expect(row.type).toBe("Withdrawal");
  });

  // TXL-022 — realized P&L is null on cash transactions; UI renders "—"
  it("Deposit realizedPnl is null", () => {
    const row = toTransactionRow(depositTx, "Cash EUR", "My Account");
    expect(row.realizedPnl).toBeNull();
  });
});

// F27 layer-3 presenter — exhaustive variant coverage across HoldingTransactionError
// and OpenHoldingError. Payload-bearing variants get pre-formatted micros (presenter
// owns the data formatting; component owns t()).
describe("transactionMutationErrorToI18n", () => {
  it("InsufficientCash interpolates balance (formatted to 2 decimals) and currency", () => {
    expect(
      transactionMutationErrorToI18n({
        code: "InsufficientCash",
        current_balance_micros: 50_000_000,
        currency: "EUR",
      }),
    ).toEqual({
      key: "cash.insufficient_cash_inline",
      vars: { balance: "50,00", currency: "EUR" },
    });
  });

  it("Oversell interpolates available + requested (formatted to 6 decimals)", () => {
    expect(
      transactionMutationErrorToI18n({
        code: "Oversell",
        available: 1_500_000,
        requested: 2_000_000,
      }),
    ).toEqual({
      key: "error.Oversell",
      vars: { available: "1,500000", requested: "2,000000" },
    });
  });

  it.each([
    "ClosedPosition",
    "CascadingOversell",
    "TransactionNotFound",
    "AccountNotFound",
    "NameAlreadyExists",
    "DatabaseError",
    "InvalidDate",
    "DateInFuture",
    "DateTooOld",
    "QuantityNotPositive",
    "AmountNotPositive",
    "UnitPriceNegative",
    "FeesNegative",
    "ExchangeRateNotPositive",
    "TotalAmountNotPositive",
    "AssetNotFound",
    "ArchivedAsset",
    "OpeningBalanceOnCashAsset",
    "InvalidTotalCost",
  ] as const)("%s unit variant maps to its flat error key", (code) => {
    // `AccountNotFound` carries `account_id` payload; we strip it because the
    // presenter falls through to the flat key regardless. Other codes are unit.
    const err =
      code === "AccountNotFound"
        ? { code, account_id: "acc-1" }
        : ({ code } as Parameters<typeof transactionMutationErrorToI18n>[0]);
    expect(transactionMutationErrorToI18n(err)).toEqual({ key: `error.${code}` });
  });
});
