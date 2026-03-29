export const SYSTEM_CATEGORY_ID = "default-uncategorized";

export function isSystemCategory(id: string): boolean {
  return id === SYSTEM_CATEGORY_ID;
}
