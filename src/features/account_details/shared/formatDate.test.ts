import { describe, expect, it } from "vitest";
import { formatIsoDate } from "./formatDate";

describe("formatIsoDate", () => {
  it("returns a human-readable string containing the year and day for a valid ISO date", () => {
    const result = formatIsoDate("2024-01-15");
    expect(result).toContain("2024");
    expect(result).toContain("15");
    expect(result).not.toBe("2024-01-15");
  });

  it("returns the raw input unchanged for an invalid date string", () => {
    expect(formatIsoDate("not-a-date")).toBe("not-a-date");
  });

  it("returns the raw input unchanged for an empty string", () => {
    expect(formatIsoDate("")).toBe("");
  });

  it("formats according to the supplied locale", () => {
    const en = formatIsoDate("2024-01-15", "en-US");
    const fr = formatIsoDate("2024-01-15", "fr-FR");
    expect(en).not.toBe(fr);
    expect(en).toContain("Jan");
    expect(fr).toContain("janv");
  });

  it("falls back to system locale when locale is undefined (backwards-compat)", () => {
    // Explicit undefined must match omitted-arg behavior so legacy callers
    // that haven't been migrated to thread i18n.language still render dates.
    expect(formatIsoDate("2024-01-15", undefined)).toBe(formatIsoDate("2024-01-15"));
  });
});
