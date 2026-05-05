import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  redirect,
} from "@tanstack/react-router";
import { AccountDetailsView } from "@/features/account_details";
import { AccountManager } from "@/features/accounts";
import { AssetManager } from "@/features/assets";
import { CategoryManager } from "@/features/categories";
import { DesignSystemPage } from "@/features/design-system";
import { SettingsPage } from "@/features/settings";
import { AddTransactionPage, TransactionListPage } from "@/features/transactions";
import { getLastPath } from "@/lib/lastPath";
import { AppShell } from "./AppShell";

const rootRoute = createRootRoute({ component: AppShell });

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  beforeLoad: () => {
    const path = getLastPath();
    if (path === "/assets") {
      throw redirect({
        to: "/assets",
        search: { createNew: undefined, returnPath: undefined },
      });
    }
    throw redirect({ to: path });
  },
});

const assetsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/assets",
  component: AssetManager,
  validateSearch: (search: Record<string, unknown>) => ({
    createNew: typeof search.createNew === "string" ? search.createNew : undefined,
    returnPath: typeof search.returnPath === "string" ? search.returnPath : undefined,
  }),
});

const accountsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/accounts",
  component: AccountManager,
});

const accountDetailsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/accounts/$accountId",
  component: AccountDetailsView,
});

const transactionListRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/accounts/$accountId/transactions/$assetId",
  component: TransactionListPage,
  validateSearch: (search: Record<string, unknown>) => ({
    pendingTransactionAssetId:
      typeof search.pendingTransactionAssetId === "string"
        ? search.pendingTransactionAssetId
        : undefined,
  }),
});

const addTransactionRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/transactions/new",
  component: AddTransactionPage,
  validateSearch: (search: Record<string, unknown>) => ({
    prefillAssetId: typeof search.prefillAssetId === "string" ? search.prefillAssetId : undefined,
    prefillAccountId:
      typeof search.prefillAccountId === "string" ? search.prefillAccountId : undefined,
  }),
});

const categoriesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/categories",
  component: CategoryManager,
});

const designSystemRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/design-system",
  component: import.meta.env.DEV ? DesignSystemPage : () => null,
});

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: SettingsPage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  assetsRoute,
  accountsRoute,
  accountDetailsRoute,
  transactionListRoute,
  addTransactionRoute,
  categoriesRoute,
  designSystemRoute,
  settingsRoute,
]);

export const router = createRouter({
  routeTree,
  history: createHashHistory(),
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
