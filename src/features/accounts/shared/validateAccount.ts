// R14 — block submission if name is empty or whitespace-only
export function validateAccountName(name: string): string | null {
  if (name.trim().length === 0) {
    return "account.error_name_required";
  }
  return null;
}
