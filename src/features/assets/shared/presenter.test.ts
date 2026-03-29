import { describe, expect, it } from "vitest";
import { getRiskBadgeClasses } from "./presenter";

describe("getRiskBadgeClasses", () => {
  // R11 — 5 distinct colours for risk levels 1–5
  it("returns a distinct class for each risk level 1–5", () => {
    const classes = [1, 2, 3, 4, 5].map(getRiskBadgeClasses);
    const unique = new Set(classes);
    expect(unique.size).toBe(5);
  });

  it("level 1 is green-toned", () => {
    expect(getRiskBadgeClasses(1)).toContain("green");
  });

  it("level 3 is orange-toned", () => {
    expect(getRiskBadgeClasses(3)).toContain("orange");
  });

  it("level 5 is red-toned", () => {
    expect(getRiskBadgeClasses(5)).toContain("red");
  });

  it("returns fallback class for out-of-range level", () => {
    expect(getRiskBadgeClasses(0)).toContain("gray");
    expect(getRiskBadgeClasses(6)).toContain("gray");
  });
});
