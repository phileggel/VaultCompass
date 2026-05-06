import { describe, expect, it } from "vitest";
import { isSystemCategory, SYSTEM_CASH_CATEGORY_ID, SYSTEM_CATEGORY_ID } from "./presenter";

describe("isSystemCategory (CSH-017)", () => {
  it("returns true for the default uncategorized id", () => {
    expect(isSystemCategory(SYSTEM_CATEGORY_ID)).toBe(true);
  });

  // CSH-017 — system Cash Category is also flagged as system, hidden from category lists.
  it("returns true for the system Cash Category id", () => {
    expect(isSystemCategory(SYSTEM_CASH_CATEGORY_ID)).toBe(true);
  });

  it("returns false for a regular category id", () => {
    expect(isSystemCategory("user-category-1")).toBe(false);
  });
});
