import { getName, getVersion } from "@tauri-apps/api/app";
import { create } from "zustand";
import { type Account, type Asset, type AssetCategory, commands, events } from "../bindings";
import { accountGateway } from "../features/accounts/gateway";
import { assetGateway } from "../features/assets/gateway";
import { logger } from "./logger";

interface AppState {
  // Application metadata
  appName: string;
  appVersion: string;

  // Data
  assets: Asset[];
  categories: AssetCategory[];
  accounts: Account[];

  // Loading states
  isLoadingAssets: boolean;
  isLoadingCategories: boolean;
  isLoadingAccounts: boolean;
  isInitialized: boolean;

  // Error handling
  assetsError: string | null;
  categoriesError: string | null;
  accountsError: string | null;

  // Actions
  fetchAssets: () => Promise<void>;
  fetchCategories: () => Promise<void>;
  fetchAccounts: () => Promise<void>;

  // Initialization
  isAnyLoading: () => boolean;
  init: () => () => void;
}

export const useAppStore = create<AppState>((set, get) => {
  return {
    appName: "VaultCompass",
    appVersion: "...",
    assets: [],
    categories: [],
    accounts: [],
    isLoadingAssets: false,
    isLoadingCategories: false,
    isLoadingAccounts: false,
    isInitialized: false,
    assetsError: null,
    categoriesError: null,
    accountsError: null,

    fetchAssets: async () => {
      set({ isLoadingAssets: true, assetsError: null });
      const result = await assetGateway.getAssetsWithArchived();
      if (result.status === "ok") {
        set({ assets: result.data, isLoadingAssets: false });
      } else {
        set({ assetsError: result.error, isLoadingAssets: false });
      }
    },

    fetchCategories: async () => {
      set({ isLoadingCategories: true, categoriesError: null });
      const result = await commands.getCategories();
      if (result.status === "ok") {
        set({ categories: result.data, isLoadingCategories: false });
      } else {
        set({ categoriesError: result.error, isLoadingCategories: false });
      }
    },

    fetchAccounts: async () => {
      set({ isLoadingAccounts: true, accountsError: null });
      const result = await accountGateway.getAccounts();
      if (result.status === "ok") {
        set({ accounts: result.data, isLoadingAccounts: false });
      } else {
        set({ accountsError: result.error, isLoadingAccounts: false });
      }
    },

    isAnyLoading: () => {
      const state = get();
      return state.isLoadingAssets || state.isLoadingCategories || state.isLoadingAccounts;
    },

    init: () => {
      if (get().isInitialized) {
        return () => {};
      }

      const { fetchAssets, fetchCategories, fetchAccounts } = get();

      const fetchMetadata = async () => {
        try {
          const [name, version] = await Promise.all([getName(), getVersion()]);
          set({ appName: name, appVersion: version });
        } catch (e) {
          logger.error("[store] failed to fetch app metadata", e);
        }
      };

      // initial parallelized fetch
      Promise.all([fetchAssets(), fetchCategories(), fetchAccounts(), fetchMetadata()]).then(() => {
        set({ isInitialized: true });
      });

      // Bus Event
      const eventMap: Record<string, () => void> = {
        AssetUpdated: fetchAssets,
        CategoryUpdated: fetchCategories,
        AccountUpdated: fetchAccounts,
      };

      // Events handled locally by feature hooks (e.g. useAccountDetails) — not global store concerns
      const locallyHandledEvents = new Set(["TransactionUpdated"]);

      // Setup event listeners
      const unlistenPromise = events.event.listen((event) => {
        const handler = eventMap[event.payload.type];
        if (handler) {
          handler();
        } else if (!locallyHandledEvents.has(event.payload.type)) {
          logger.debug("[store] unhandled event", { type: event.payload.type });
        }
      });

      // Return cleanup function
      return () => {
        unlistenPromise.then((unlisten) => unlisten());
        set({ isInitialized: false });
      };
    },
  };
});
