// R14 — block submission if name is empty or whitespace-only
export function validateAccountName(name: string): string | null {
  if (name.trim().length === 0) {
    return "account.error_name_required";
  }
  return null;
}

// Block submission if currency is not a 3-letter uppercase ISO 4217 code
export function validateAccountCurrency(currency: string): string | null {
  if (!/^[A-Z]{3}$/.test(currency.trim())) {
    return "account.error_currency_invalid";
  }
  return null;
}
