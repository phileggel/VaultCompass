import { AlertCircle, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { formatIsoDateTime } from "@/ui/format/date";
import { useSyncIndicator } from "./useSyncIndicator";

/**
 * SYN-063 — compact shell indicator: the last-sync time plus an attention
 * badge when the status carries failures, notices or inconsistent holdings.
 * Hidden while sync is disabled (SYN-010).
 */
export function SyncIndicator() {
  const { t, i18n } = useTranslation();
  const { visible, lastSyncCompletedAt, needsAttention } = useSyncIndicator();

  if (!visible) {
    return null;
  }

  const lastSync =
    lastSyncCompletedAt === null
      ? t("sync.indicator.never")
      : t("sync.indicator.last_sync", {
          when: formatIsoDateTime(lastSyncCompletedAt, i18n.language),
        });

  return (
    <div
      id="sync-indicator"
      data-testid="sync-indicator"
      className="flex items-center gap-2 text-xs text-white/90"
      title={lastSync}
    >
      <RefreshCw size={14} aria-label={t("sync.indicator.label")} role="img" />
      <span className="hidden sm:inline">{lastSync}</span>
      {needsAttention && (
        <AlertCircle
          id="sync-indicator-attention"
          data-testid="sync-indicator-attention"
          size={14}
          className="text-m3-warning"
          role="img"
          aria-label={t("sync.indicator.attention")}
        />
      )}
    </div>
  );
}
