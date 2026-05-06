export const SYSTEM_CATEGORY_ID = "default-uncategorized";
/** Cash category seeded by ensure_cash_asset (CSH-017) — hidden from category lists. */
export const SYSTEM_CASH_CATEGORY_ID = "system-cash-category";

export function isSystemCategory(id: string): boolean {
  return id === SYSTEM_CATEGORY_ID || id === SYSTEM_CASH_CATEGORY_ID;
}

/**
 * Excludes the Cash Category from a category list (CSH-017).
 * Default uncategorized stays visible because users explicitly pick it for new assets.
 */
export function excludeCashCategory<T extends { id: string }>(categories: T[]): T[] {
  return categories.filter((c) => c.id !== SYSTEM_CASH_CATEGORY_ID);
}
