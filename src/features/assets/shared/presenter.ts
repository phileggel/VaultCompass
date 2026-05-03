import type { AssetClass } from "@/bindings";
import { DEFAULT_RISK_BY_CLASS } from "./constants";

/** Returns Tailwind classes for the risk badge — R11 (5 distinct colours). */
export function getRiskBadgeClasses(riskLevel: number): string {
  switch (riskLevel) {
    case 1:
      return "bg-green-100 text-green-700";
    case 2:
      return "bg-green-200 text-green-800";
    case 3:
      return "bg-orange-100 text-orange-700";
    case 4:
      return "bg-red-100 text-red-700";
    case 5:
      return "bg-red-200 text-red-800";
    default:
      return "bg-gray-100 text-gray-600";
  }
}

/** Returns the default risk level for the given asset class — R3/R10. */
export function getDefaultRisk(assetClass: AssetClass): number {
  return DEFAULT_RISK_BY_CLASS[assetClass];
}

/** Returns a human-readable label for an asset class — WEB-031. */
export function formatAssetClass(assetClass: AssetClass): string {
  switch (assetClass) {
    case "Cash":
      return "Cash";
    case "Bonds":
      return "Bonds";
    case "RealEstate":
      return "Real Estate";
    case "MutualFunds":
      return "Mutual Funds";
    case "ETF":
      return "ETF";
    case "Stocks":
      return "Stocks";
    case "DigitalAsset":
      return "Digital Asset";
    case "Derivatives":
      return "Derivatives";
  }
}
