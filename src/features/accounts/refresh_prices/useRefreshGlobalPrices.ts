import { useNavigate } from "@tanstack/react-router";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { connectionGateway } from "@/features/connections/gateway";
import { openModalSearch } from "@/lib/modalSearch";
import { useSnackbar } from "@/ui/components/snackbar/snackbarStore";
import { accountGateway } from "../gateway";
import { fetchPriceErrorToI18n } from "../shared/presenter";

/**
 * MKT-115 / MKT-133 — Global "Refresh prices" hook for the AccountManager header.
 *
 * Calls `accountGateway.fetchAllAssetPrices()` and surfaces the result via the
 * global snackbar. Success dispatches `mkt.fetch_dispatched`; errors route
 * through `fetchPriceErrorToI18n` (F27 layer 3) for typed key + severity.
 *
 * `isPending` toggles for the duration of the gateway call so the button can disable itself.
 */
export function useRefreshGlobalPrices(): {
  isPending: boolean;
  refresh: () => Promise<void>;
} {
  const [isPending, setIsPending] = useState(false);
  const showSnackbar = useSnackbar();
  const navigate = useNavigate();
  const { t } = useTranslation();

  const refresh = useCallback(async () => {
    setIsPending(true);
    try {
      // KEY-040 — gate on a stored provider key: when Stooq has no key, route the
      // user to the Connections dialog instead of dispatching a fetch that cannot
      // succeed. A non-ok / indeterminate key check degrades to a dispatch (the
      // backend short-circuits per KEY-044 and the snackbar surfaces the outcome).
      const connections = await connectionGateway.getProviderConnections();
      const stooq =
        connections?.status === "ok"
          ? connections.data.find((c) => c.provider === "Stooq")
          : undefined;
      if (stooq && !stooq.has_key) {
        openModalSearch(navigate, { modal: "connections" });
        return;
      }
      const result = await accountGateway.fetchAllAssetPrices();
      if (result.status === "ok") {
        showSnackbar(t("mkt.fetch_dispatched"), "info");
        return;
      }
      const msg = fetchPriceErrorToI18n(result.error);
      showSnackbar(t(msg.key, msg.vars), msg.severity);
    } finally {
      setIsPending(false);
    }
  }, [navigate, showSnackbar, t]);

  return { isPending, refresh };
}
