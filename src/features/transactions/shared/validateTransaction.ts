import type { TransactionFormData } from "./types";

/**
 * Client-side validation for the transaction form (mirrors TRX-020).
 * Receives already-computed micro-unit values to avoid redundant conversions.
 * Returns the first i18n error key, or null if the form is valid.
 */
export function validateTransactionForm(
  data: TransactionFormData,
  qtyMicro: number,
  totalMicro: number,
): string | null {
  if (!data.accountId) return "transaction.error_validation_account";
  if (!data.assetId) return "transaction.error_validation_asset";
  if (!data.date) return "transaction.error_validation_date";
  if (qtyMicro <= 0) return "transaction.error_validation_quantity";
  if (totalMicro <= 0) return "transaction.error_validation_total";
  return null;
}
