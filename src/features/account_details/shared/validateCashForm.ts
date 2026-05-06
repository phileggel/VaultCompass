/**
 * Validation helpers for Deposit / Withdrawal forms (CSH-021, CSH-031).
 * Returns an i18n key on failure, or null when the value is acceptable.
 */
export function validateAmount(amount: string): string | null {
  if (amount.length === 0) return "validation.amount_not_positive";
  const n = parseFloat(amount);
  if (!Number.isFinite(n) || n <= 0) return "validation.amount_not_positive";
  return null;
}

export function validateDate(date: string): string | null {
  if (date.length === 0 || !/^\d{4}-\d{2}-\d{2}$/.test(date)) {
    return "validation.invalid_date";
  }
  const today = new Date().toISOString().slice(0, 10);
  if (date > today) return "validation.date_in_future";
  if (date < "1900-01-01") return "validation.date_too_old";
  return null;
}
