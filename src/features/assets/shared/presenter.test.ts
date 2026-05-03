import { describe, expect, it } from "vitest";
import { formatAssetClass, getRiskBadgeClasses } from "./presenter";

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

describe("formatAssetClass", () => {
  // WEB-031 — multi-word class names are split into readable labels
  it("splits compound class names into readable labels", () => {
    expect(formatAssetClass("RealEstate")).toBe("Real Estate");
    expect(formatAssetClass("MutualFunds")).toBe("Mutual Funds");
    expect(formatAssetClass("DigitalAsset")).toBe("Digital Asset");
  });

  // WEB-031 — single-word and abbreviation classes pass through unchanged
  it("returns single-word and abbreviation classes unchanged", () => {
    expect(formatAssetClass("Cash")).toBe("Cash");
    expect(formatAssetClass("Bonds")).toBe("Bonds");
    expect(formatAssetClass("ETF")).toBe("ETF");
    expect(formatAssetClass("Stocks")).toBe("Stocks");
    expect(formatAssetClass("Derivatives")).toBe("Derivatives");
  });

  // Exhaustiveness — all 8 AssetClass variants are handled
  it("covers all 8 AssetClass variants with distinct non-empty labels", () => {
    const all = [
      "Cash",
      "Bonds",
      "RealEstate",
      "MutualFunds",
      "ETF",
      "Stocks",
      "DigitalAsset",
      "Derivatives",
    ] as const;
    const labels = all.map(formatAssetClass);
    expect(labels.every((l) => l.trim().length > 0)).toBe(true);
    expect(new Set(labels).size).toBe(8);
  });
});
