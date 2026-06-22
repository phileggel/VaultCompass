import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { fetchPriceForDateErrorToI18n } from "@/features/accounts/shared/presenter";
import { useSnackbar } from "@/ui/components/snackbar/snackbarStore";
import { accountDetailsGateway } from "../gateway";

/** Today as an ISO `yyyy-mm-dd` string — the default valuation date. */
const todayIso = (): string => new Date().toISOString().slice(0, 10);

/**
 * Date-scoped price fetch for the account: picks a date, fetches each fetchable
 * holding's Yahoo close at (or carried back to) that date, and stores it keyed to
 * the chosen date. Surfaces the outcome via the global snackbar — a success when
 * every asset resolved, an info line naming the unavailable assets otherwise.
 * Keyless (ADR-017). Isolated from the latest-price refresh path.
 */
export function useFetchAccountPricesForDate(
  accountId: string,
  onDone: () => void,
): {
  date: string;
  setDate: (date: string) => void;
  isSubmitting: boolean;
  submit: () => Promise<void>;
} {
  const [date, setDate] = useState(todayIso);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const showSnackbar = useSnackbar();
  const { t } = useTranslation();

  const submit = useCallback(async () => {
    setIsSubmitting(true);
    try {
      const result = await accountDetailsGateway.fetchAccountAssetPricesForDate(accountId, date);
      if (result.status === "ok") {
        const { stored, missing } = result.data;
        if (missing.length === 0) {
          showSnackbar(t("mkt.fetch_date_stored", { count: stored }), "success");
        } else {
          showSnackbar(
            t("mkt.fetch_date_partial", { count: stored, missing: missing.join(", ") }),
            "info",
          );
        }
        onDone();
        return;
      }
      const msg = fetchPriceForDateErrorToI18n(result.error);
      showSnackbar(t(msg.key), msg.severity);
    } finally {
      setIsSubmitting(false);
    }
  }, [accountId, date, showSnackbar, t, onDone]);

  return { date, setDate, isSubmitting, submit };
}
