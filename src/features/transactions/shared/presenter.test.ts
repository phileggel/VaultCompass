import { describe, expect, it } from "vitest";
import { toTransactionRow } from "./presenter";

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
