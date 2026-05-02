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
});
