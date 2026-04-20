import type { Transaction } from "@/bindings";
import { microToDecimal } from "@/lib/microUnits";

/** Display-ready shape for a transaction row. */
export interface TransactionRowViewModel {
  id: string;
  accountId: string;
  assetId: string;
  assetName: string;
  accountName: string;
  /** Transaction type label (e.g. "Purchase"). */
  type: string;
  date: string;
  /** Formatted quantity string (3 decimal places). */
  quantity: string;
  /** Formatted unit price string (3 decimal places). */
  unitPrice: string;
  /** Formatted exchange rate string (3 decimal places). */
  exchangeRate: string;
  /** Formatted fees string (3 decimal places). */
  fees: string;
  /** Formatted total amount string (3 decimal places). */
  totalAmount: string;
  note: string | null;
  /** Formatted realized P&L string (3 decimal places), null for Purchase rows (SEL-041). */
  realizedPnl: string | null;
  /** Raw realized P&L in micro-units — used for sign-based color styling (SEL-043). */
  realizedPnlRaw: number | null;
}

/**
 * Maps a raw Transaction + contextual names to a display-ready ViewModel (TRX-024).
 */
export function toTransactionRow(
  tx: Transaction,
  assetName: string,
  accountName: string,
): TransactionRowViewModel {
  return {
    id: tx.id,
    accountId: tx.account_id,
    assetId: tx.asset_id,
    assetName,
    accountName,
    type: tx.transaction_type,
    date: tx.date,
    quantity: microToDecimal(tx.quantity),
    unitPrice: microToDecimal(tx.unit_price),
    exchangeRate: microToDecimal(tx.exchange_rate),
    fees: microToDecimal(tx.fees),
    totalAmount: microToDecimal(tx.total_amount),
    note: tx.note ?? null,
    realizedPnl: tx.realized_pnl != null ? microToDecimal(tx.realized_pnl) : null,
    realizedPnlRaw: tx.realized_pnl ?? null,
  };
}
