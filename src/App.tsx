import { RouterProvider } from "@tanstack/react-router";
import { Loader2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ProviderConnection } from "@/bindings";
import { accountGateway } from "@/features/accounts/gateway";
import { connectionGateway } from "@/features/connections/gateway";
import { shellGateway } from "@/features/shell/gateway";
import { getAutoFetch } from "@/lib/autoFetchStorage";
import { logger } from "@/lib/logger";
import { getUseStooqApiKey } from "@/lib/stooqKeyModeStorage";
import { useAppStore } from "@/lib/store";
import { router } from "./router";

/**
 * KEY-041 — pure predicate: should the launch auto-fetch run? True only when the
 * price provider (Stooq) has a stored key. A key in any storage tier qualifies; an
 * absent key (or empty list) skips the launch fetch silently — no dialog (that is
 * reserved for explicit user-triggered refresh, KEY-040).
 */
export function shouldLaunchFetch(connections: ProviderConnection[]): boolean {
  return connections.find((c) => c.provider === "Stooq")?.has_key === true;
}

function App() {
  const [dbError, setDbError] = useState<string | null>(null);
  const init = useAppStore((state) => state.init);
  const isInitialized = useAppStore((state) => state.isInitialized);
  const { t } = useTranslation();

  useEffect(() => {
    logger.info("[App] mounted");
    // R18 — listen for critical migration failure from backend
    const unlistenPromise = shellGateway.onMigrationError((message) => {
      setDbError(message);
    });
    return () => {
      unlistenPromise.then((fn) => fn()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    return init();
  }, [init]);

  // MKT-121 — fire-and-forget auto-fetch when the user has enabled the setting.
  // Runs once after init completes; per-asset results arrive via AssetPriceUpdated events.
  // Dispatch-level failures (FetchAlreadyRunning, NoFetchableHoldings, DatabaseError) are
  // logged server-side via the FE logger — no startup snackbar to avoid noise on launch.
  useEffect(() => {
    if (!isInitialized) return;
    if (!getAutoFetch()) return;
    (async () => {
      try {
        // KEY-050 — read the device-local fetch mode. KEY-052: in keyless mode the
        // KEY-041 no-key launch skip does not apply — dispatch anonymously.
        const useApiKey = getUseStooqApiKey();
        if (useApiKey) {
          // KEY-041 — skip the launch fetch silently when no provider key is stored
          // (no dialog on cold start; absence surfaces via the per-holding diagnostics).
          const connections = await connectionGateway.getProviderConnections();
          if (connections.status !== "ok" || !shouldLaunchFetch(connections.data)) {
            return;
          }
        }
        const result = await accountGateway.fetchAllAssetPrices(useApiKey);
        if (result.status === "error") {
          logger.warn("[App] auto-fetch dispatch returned error", { code: result.error.code });
        }
      } catch (error) {
        logger.error("[App] auto-fetch dispatch threw", { error });
      }
    })();
  }, [isInitialized]);

  // R18 — critical migration error: app blocked with error message
  if (dbError) {
    return (
      <div className="h-screen flex items-center justify-center bg-m3-surface p-8">
        <div className="max-w-md text-center flex flex-col gap-4">
          <p className="text-m3-error font-medium text-lg">{t("app.migration_error")}</p>
          <p className="text-m3-on-surface-variant text-sm font-mono">{dbError}</p>
        </div>
      </div>
    );
  }

  // R17 — loading screen while migrations/init are running
  if (!isInitialized) {
    return (
      <div className="h-screen flex items-center justify-center bg-m3-surface">
        <div className="flex flex-col items-center gap-3">
          <Loader2 className="w-6 h-6 animate-spin text-m3-primary" />
          <p className="text-m3-on-surface-variant text-sm">{t("app.migration_in_progress")}</p>
        </div>
      </div>
    );
  }

  return <RouterProvider router={router} />;
}

export default App;
