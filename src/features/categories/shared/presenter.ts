import type { CategoryCrudError } from "@/bindings";
import type { I18nMessage } from "@/ui/format/i18n";

export const SYSTEM_CATEGORY_ID = "default-uncategorized";
/** Cash category seeded by ensure_cash_asset (CSH-017) — hidden from category lists. */
export const SYSTEM_CASH_CATEGORY_ID = "system-cash-category";

export function isSystemCategory(id: string): boolean {
  return id === SYSTEM_CATEGORY_ID || id === SYSTEM_CASH_CATEGORY_ID;
}

/**
 * F27 — Maps any category-BC mutation error (add / update / delete) to a
 * category-scoped i18n key. Pure function, no React, no useTranslation.
 *
 * Encodes the project's domain mapping:
 * - `DuplicateName` → name-collision wording
 * - `SystemReadonly` / `SystemProtected` → system-category protection wording
 * - Everything else (LabelEmpty / NotFound / DatabaseError) → generic fallback
 *
 * No payload-bearing variants worth interpolating today — `NotFound { id }`
 * exposes internal IDs that don't help the user.
 */
export function categoryMutationErrorToI18n(err: CategoryCrudError): I18nMessage {
  switch (err.code) {
    case "DuplicateName":
      return { key: "category.error_duplicate" };
    case "SystemReadonly":
      return { key: "category.error_system_readonly" };
    case "SystemProtected":
      return { key: "category.error_system_protected" };
    case "NotFound":
    case "DatabaseError":
    case "LabelEmpty":
      return { key: "category.error_generic" };
    default: {
      const _exhaustive: never = err;
      return _exhaustive;
    }
  }
}
