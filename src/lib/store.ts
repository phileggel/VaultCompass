import { create } from "zustand";
import { type Account, type Asset, type AssetCategory, commands, events } from "../bindings";
import { accountGateway } from "../features/accounts/gateway";
import { assetGateway } from "../features/assets/gateway";

interface AppState {
  assets: Asset[];
  categories: AssetCategory[];
  accounts: Account[];

  // Loading states
  isLoadingAssets: boolean;
  isLoadingCategories: boolean;
  isLoadingAccounts: boolean;
  isInitialized: boolean;

  // Error handling
  error: string | null;

  // Actions
  fetchAssets: () => Promise<void>;
  fetchCategories: () => Promise<void>;
  fetchAccounts: () => Promise<void>;

  // Initialization
  isAnyLoading: () => boolean;
  init: () => () => void;
}

export const useAppStore = create<AppState>((set, get) => {
  const handleFetch = async <T>(
    loadingKey: keyof AppState,
    fetchFn: () => Promise<{ status: "ok"; data: T } | { status: "error"; error: string }>,
    onSuccess: (data: T) => void,
  ) => {
    set({ [loadingKey]: true, error: null } as Partial<AppState>);

    const result = await fetchFn();

    if (result.status === "ok") {
      onSuccess(result.data);
      set({ [loadingKey]: false } as Partial<AppState>);
    } else {
      set({ error: result.error, [loadingKey]: false } as Partial<AppState>);
    }
  };

  return {
    assets: [],
    categories: [],
    accounts: [],
    isLoadingAssets: false,
    isLoadingCategories: false,
    isLoadingAccounts: false,
    isInitialized: false,
    error: null,

    fetchAssets: () =>
      handleFetch("isLoadingAssets", assetGateway.getAssets, (data) => set({ assets: data })),

    fetchCategories: () =>
      handleFetch("isLoadingCategories", commands.getCategories, (data) =>
        set({ categories: data }),
      ),

    fetchAccounts: () =>
      handleFetch("isLoadingAccounts", accountGateway.getAccounts, (data) =>
        set({ accounts: data }),
      ),

    isAnyLoading: () => {
      const state = get();
      return state.isLoadingAssets || state.isLoadingCategories || state.isLoadingAccounts;
    },

    init: () => {
      if (get().isInitialized) {
        return () => {};
      }

      const { fetchAssets, fetchCategories, fetchAccounts } = get();

      // initial parallelized fetch
      Promise.all([fetchAssets(), fetchCategories(), fetchAccounts()]).then(() => {
        set({ isInitialized: true });
      });

      // Bus Event
      const eventMap: Record<string, () => void> = {
        AssetUpdated: fetchAssets,
        CategoryUpdated: fetchCategories,
        AccountUpdated: fetchAccounts,
      };

      // Setup event listeners
      const unlistenPromise = events.event.listen((event) => {
        const handler = eventMap[event.payload.type];
        if (handler) {
          handler();
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
