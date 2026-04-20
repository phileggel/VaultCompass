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

/**
 * Client-side validation for the sell transaction form (SEL-022).
 * Same base rules as purchase, plus oversell guard against maxQuantityMicro.
 * Returns the first i18n error key, or null if the form is valid.
 */
export function validateSellForm(
  data: TransactionFormData,
  qtyMicro: number,
  totalMicro: number,
  maxQuantityMicro: number,
): string | null {
  const base = validateTransactionForm(data, qtyMicro, totalMicro);
  if (base) return base;
  if (qtyMicro > maxQuantityMicro) return "transaction.error_validation_oversell";
  return null;
}
