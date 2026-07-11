export type ModalTarget = {
  accountName: string;
  assetId: string;
  assetName: string;
  assetCurrency: string;
  showExchangeRate: boolean;
};

export type SellTarget = ModalTarget & {
  holdingQuantityMicro: number;
};

/** The holding a stock split rescales — quantity, average and latest price feed the modal's preview and price prefill (SPL-061/040). */
export type SplitTarget = {
  assetId: string;
  assetName: string;
  /** Holding quantity in micro-units. */
  holdingQuantityMicro: number;
  /** Average price in micro-units. */
  averagePriceMicro: number;
  /** Latest recorded price in micro-units, or null when none exists (SPL-040). */
  currentPriceMicro: number | null;
};

/** A holding a price can be recorded against — active, non-cash (MKT-010/011). */
export interface PriceableAsset {
  assetId: string;
  assetName: string;
  assetCurrency: string;
}
