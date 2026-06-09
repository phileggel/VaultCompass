import { useNavigate } from "@tanstack/react-router";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { fetchPriceErrorToI18n } from "@/features/accounts/shared/presenter";
import { connectionGateway } from "@/features/connections/gateway";
import { openModalSearch } from "@/lib/modalSearch";
import { useSnackbar } from "@/ui/components/snackbar/snackbarStore";
import { accountDetailsGateway } from "../gateway";

/**
 * MKT-115 / MKT-131 / MKT-132 / MKT-133 — Per-account "Refresh prices" hook for the
 * AccountDetailsView header.
 *
 * Calls `accountDetailsGateway.fetchAccountAssetPrices(accountId)` and surfaces the
 * result via the global snackbar. Success dispatches `mkt.fetch_dispatched`; errors
 * route through `fetchPriceErrorToI18n` (F27 layer 3) for typed key + severity.
 */
export function useRefreshAccountPrices(accountId: string): {
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
      // KEY-040 — gate on a stored provider key (see useRefreshGlobalPrices).
      const connections = await connectionGateway.getProviderConnections();
      const stooq =
        connections?.status === "ok"
          ? connections.data.find((c) => c.provider === "Stooq")
          : undefined;
      if (stooq && !stooq.has_key) {
        openModalSearch(navigate, { modal: "connections" });
        return;
      }
      const result = await accountDetailsGateway.fetchAccountAssetPrices(accountId);
      if (result.status === "ok") {
        showSnackbar(t("mkt.fetch_dispatched"), "info");
        return;
      }
      const msg = fetchPriceErrorToI18n(result.error);
      showSnackbar(t(msg.key, msg.vars), msg.severity);
    } finally {
      setIsPending(false);
    }
  }, [accountId, navigate, showSnackbar, t]);

  return { isPending, refresh };
}
