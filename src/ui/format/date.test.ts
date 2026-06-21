import { describe, expect, it } from "vitest";
import { formatIsoDateNumeric } from "./date";

describe("formatIsoDateNumeric", () => {
  it("formats an ISO date as French numeric DD/MM/YYYY", () => {
    expect(formatIsoDateNumeric("2026-06-14", "fr")).toBe("14/06/2026");
  });

  it("formats an ISO date as US numeric M/D/YYYY", () => {
    expect(formatIsoDateNumeric("2026-06-14", "en")).toBe("6/14/2026");
  });

  it("does not shift the day across a timezone offset (noon anchor)", () => {
    expect(formatIsoDateNumeric("2026-01-01", "fr")).toBe("01/01/2026");
    expect(formatIsoDateNumeric("2026-12-31", "fr")).toBe("31/12/2026");
  });

  it("returns the raw input unchanged when it does not parse", () => {
    expect(formatIsoDateNumeric("not-a-date", "fr")).toBe("not-a-date");
  });
});
