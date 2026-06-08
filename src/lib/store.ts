import { getName, getVersion } from "@tauri-apps/api/app";
import i18next from "i18next";
import { create } from "zustand";
import { type Account, type Asset, type AssetCategory, events } from "../bindings";
import { accountGateway } from "../features/accounts/gateway";
import { assetGateway } from "../features/assets/gateway";
import { categoryGateway } from "../features/categories/gateway";
import { type SnackbarVariant, useSnackbarStore } from "../ui/components/snackbar/snackbarStore";
import { logger } from "./logger";

/**
 * Builds the snackbar for a finished price-fetch task (MKT-145), or `null` when
 * the task fully succeeded (`skipped === 0`) and nothing should be shown.
 */
export function buildPriceFetchFeedback(
  ok: number,
  skipped: number,
): { message: string; variant: SnackbarVariant } | null {
  if (skipped === 0) return null;
  return ok > 0
    ? { message: i18next.t("mkt.fetch_completed_partial", { ok, skipped }), variant: "info" }
    : { message: i18next.t("mkt.fetch_completed_failed", { skipped }), variant: "error" };
}

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
        set({
          assetsError: `error.${result.error.code}`,
          isLoadingAssets: false,
        });
      }
    },

    fetchCategories: async () => {
      set({ isLoadingCategories: true, categoriesError: null });
      const result = await categoryGateway.getCategories();
      if (result.status === "ok") {
        set({ categories: result.data, isLoadingCategories: false });
      } else {
        set({
          categoriesError: `error.${result.error.code}`,
          isLoadingCategories: false,
        });
      }
    },

    fetchAccounts: async () => {
      set({ isLoadingAccounts: true, accountsError: null });
      const result = await accountGateway.getAccounts();
      if (result.status === "ok") {
        set({ accounts: result.data, isLoadingAccounts: false });
      } else {
        set({
          accountsError: `error.${result.error.code}`,
          isLoadingAccounts: false,
        });
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
      const locallyHandledEvents = new Set(["TransactionUpdated", "CurrencyRateUpdated"]);

      // Setup event listeners
      const unlistenPromise = events.event.listen((event) => {
        const payload = event.payload;
        // The global store is the app's single always-on event sink, so a
        // launch-time fetch-failure snackbar surfaces regardless of the open view.
        if (payload.type === "AssetPriceFetchCompleted") {
          const feedback = buildPriceFetchFeedback(payload.ok, payload.skipped);
          if (feedback) {
            useSnackbarStore.getState().show(feedback.message, feedback.variant);
          }
          return;
        }
        const handler = eventMap[payload.type];
        if (handler) {
          handler();
        } else if (!locallyHandledEvents.has(payload.type)) {
          logger.debug("[store] unhandled event", { type: payload.type });
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
