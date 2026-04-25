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
