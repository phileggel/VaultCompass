import type { ArchiveAssetError, AssetClass, AssetCrudError, DeleteAssetError } from "@/bindings";
import type { I18nMessage } from "@/ui/format/i18n";
import { DEFAULT_RISK_BY_CLASS } from "./constants";

/**
 * F27 — Maps any asset-BC mutation error (add / update / archive / unarchive / delete)
 * to an i18n key + interpolation vars. Pure function, no React, no useTranslation.
 *
 * Exhaustive switch on `code`: TypeScript catches new variants at compile time.
 */
export function assetMutationErrorToI18n(
  err: AssetCrudError | ArchiveAssetError | DeleteAssetError,
): I18nMessage {
  switch (err.code) {
    case "InvalidExchange":
      return { key: "error.InvalidExchange", vars: { exchange_code: err.exchange_code } };
    case "InvalidCurrency":
      return { key: "error.InvalidCurrency", vars: { currency: err.currency } };
    case "NameEmpty":
    case "ReferenceEmpty":
    case "InvalidRiskLevel":
    case "Archived":
    case "CashAssetNotEditable":
    case "NotFound":
    case "DatabaseError":
    case "AccountNotFound":
    case "NameAlreadyExists":
    case "ActiveHoldings":
    case "ExistingTransactions":
    case "DuplicateName":
      return { key: `error.${err.code}` };
    default: {
      const _exhaustive: never = err;
      return _exhaustive;
    }
  }
}

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

/** Returns a localised label for an asset class — WEB-031.
 *  Exhaustive switch ensures new variants are caught at compile time. */
export function formatAssetClass(assetClass: AssetClass, t: (key: string) => string): string {
  switch (assetClass) {
    case "Cash":
      return t("asset.class.Cash");
    case "Bonds":
      return t("asset.class.Bonds");
    case "RealEstate":
      return t("asset.class.RealEstate");
    case "MutualFunds":
      return t("asset.class.MutualFunds");
    case "ETF":
      return t("asset.class.ETF");
    case "Stocks":
      return t("asset.class.Stocks");
    case "DigitalAsset":
      return t("asset.class.DigitalAsset");
    case "Derivatives":
      return t("asset.class.Derivatives");
  }
}
