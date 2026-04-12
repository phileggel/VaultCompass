import { create } from "zustand";

interface TransactionStore {
  /**
   * Last active (accountId, assetId) pair — tracked for event-driven refresh (TRX-038).
   * Updated when the user views a transaction list.
   */
  lastFetchedKey: { accountId: string; assetId: string } | null;
  setLastFetchedKey: (key: { accountId: string; assetId: string }) => void;
  /**
   * Called when a TransactionUpdated event fires (TRX-038).
   * Stub: holdings display is not yet implemented; placeholder for future refresh logic.
   */
  refreshHoldings: () => void;
}

export const useTransactionStore = create<TransactionStore>((set, get) => ({
  lastFetchedKey: null,

  setLastFetchedKey: (key) => set({ lastFetchedKey: key }),

  refreshHoldings: () => {
    // TRX-038 — triggered on TransactionUpdated event.
    // When a holdings display is implemented, this will re-fetch data for lastFetchedKey.
    const { lastFetchedKey } = get();
    if (lastFetchedKey) {
      // Future: fetch holdings for lastFetchedKey.accountId
    }
  },
}));
