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

/** A holding a price can be recorded against — active, non-cash (MKT-010/011). */
export interface PriceableAsset {
  assetId: string;
  assetName: string;
  assetCurrency: string;
}
