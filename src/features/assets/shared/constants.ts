import type { AssetClass } from "@/bindings";

export const SYSTEM_CATEGORY_ID = "default-uncategorized";

export const ASSET_CLASSES: AssetClass[] = [
  "RealEstate",
  "Cash",
  "Stocks",
  "Bonds",
  "ETF",
  "MutualFunds",
  "DigitalAsset",
  "Derivatives",
];

export const RISK_LEVELS = [1, 2, 3, 4, 5];

/** Default risk level per asset class — R3. */
export const DEFAULT_RISK_BY_CLASS: Record<AssetClass, number> = {
  Cash: 1,
  Bonds: 2,
  RealEstate: 2,
  MutualFunds: 3,
  ETF: 3,
  Stocks: 4,
  DigitalAsset: 5,
  Derivatives: 5,
};
