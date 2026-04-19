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
import { TransactionListPage } from "@/features/transactions";
import { AppShell } from "./AppShell";

const rootRoute = createRootRoute({ component: AppShell });

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({
      to: "/assets",
      search: { createNew: undefined, returnPath: undefined, pendingTransactionAssetId: undefined },
    });
  },
});

const assetsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/assets",
  component: AssetManager,
  validateSearch: (search: Record<string, unknown>) => ({
    createNew: typeof search.createNew === "string" ? search.createNew : undefined,
    returnPath: typeof search.returnPath === "string" ? search.returnPath : undefined,
    pendingTransactionAssetId:
      typeof search.pendingTransactionAssetId === "string"
        ? search.pendingTransactionAssetId
        : undefined,
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
  validateSearch: (search: Record<string, unknown>) => ({
    pendingTransactionAssetId:
      typeof search.pendingTransactionAssetId === "string"
        ? search.pendingTransactionAssetId
        : undefined,
  }),
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

const routeTree = rootRoute.addChildren([
  indexRoute,
  assetsRoute,
  accountsRoute,
  accountDetailsRoute,
  transactionListRoute,
  categoriesRoute,
  designSystemRoute,
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
