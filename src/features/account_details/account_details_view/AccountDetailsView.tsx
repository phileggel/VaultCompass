import { useNavigate, useParams } from "@tanstack/react-router";
import { Plus, Search } from "lucide-react";
import { useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { logger } from "@/lib/logger";
import { Button } from "@/ui/components/button/Button";
import { IconButton } from "@/ui/components/button/IconButton";
import type { HoldingRowViewModel } from "../shared/presenter";
import { useAccountDetails } from "./useAccountDetails";

type HoldingRowProps = {
  row: HoldingRowViewModel;
  accountId: string;
};

function HoldingRow({ row, accountId }: HoldingRowProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const handleAddTransaction = useCallback(() => {
    navigate({
      to: "/transactions/new",
      search: { prefillAccountId: accountId, prefillAssetId: row.assetId },
    });
  }, [navigate, accountId, row.assetId]);

  const handleViewTransactions = useCallback(() => {
    navigate({
      to: "/accounts/$accountId/transactions/$assetId",
      params: { accountId, assetId: row.assetId },
      search: { pendingTransactionAssetId: undefined },
    });
  }, [navigate, accountId, row.assetId]);

  return (
    <tr className="m3-tr">
      <td className="m3-td">
        <div className="flex flex-col">
          <span className="font-medium text-m3-on-surface">{row.assetName}</span>
          <span className="text-xs text-m3-on-surface-variant">{row.assetReference}</span>
        </div>
      </td>
      <td className="m3-td text-right tabular-nums">{row.quantity}</td>
      <td className="m3-td text-right tabular-nums">{row.averagePrice}</td>
      <td className="m3-td text-right tabular-nums font-medium">{row.costBasis}</td>
      <td className="m3-td">
        <div className="flex items-center gap-1">
          <IconButton
            icon={<Plus size={16} />}
            size="sm"
            aria-label={t("account_details.add_transaction")}
            onClick={handleAddTransaction}
          />
          <IconButton
            icon={<Search size={16} />}
            size="sm"
            aria-label={t("transaction.list_title")}
            onClick={handleViewTransactions}
          />
        </div>
      </td>
    </tr>
  );
}

export function AccountDetailsView() {
  const { t } = useTranslation();
  const { accountId } = useParams({ from: "/accounts/$accountId" });
  const navigate = useNavigate();
  const { isLoading, error, retry, holdings, summary } = useAccountDetails(accountId);

  useEffect(() => {
    logger.info("[AccountDetailsView] mounted");
  }, []);

  const handleAddTransaction = useCallback(() => {
    navigate({
      to: "/transactions/new",
      search: { prefillAccountId: accountId, prefillAssetId: undefined },
    });
  }, [navigate, accountId]);

  return (
    <div className="flex h-full flex-col gap-4 overflow-hidden py-2 px-2">
      <div className="flex-1 flex flex-col min-w-0 bg-m3-surface-container rounded-[28px] shadow-elevation-1 overflow-hidden">
        {/* Summary header */}
        <div className="px-6 py-4 bg-m3-surface-container-high">
          {isLoading ? (
            <div className="h-4 w-32 bg-m3-surface-variant rounded animate-pulse" />
          ) : summary ? (
            <div className="flex items-center justify-between">
              <p className="text-sm text-m3-on-surface-variant">
                {t("account_details.total_cost_basis")}:{" "}
                <span className="font-semibold text-m3-on-surface">{summary.totalCostBasis}</span>
              </p>
              {/* ACD-036 — non-empty state CTA */}
              {!summary.isEmpty && !summary.isAllClosed && (
                <Button
                  variant="tonal"
                  size="sm"
                  icon={<Plus size={14} />}
                  onClick={handleAddTransaction}
                >
                  {t("account_details.add_transaction")}
                </Button>
              )}
            </div>
          ) : null}
        </div>

        {/* Content */}
        <div className="flex-1 overflow-auto">
          {isLoading ? (
            /* ACD-037 — loading skeletons */
            <div className="animate-pulse p-4 space-y-3">
              {[1, 2, 3].map((i) => (
                <div key={i} className="h-10 bg-m3-surface-variant rounded-lg" />
              ))}
            </div>
          ) : error ? (
            /* ACD-038 — error state */
            <div className="flex flex-col items-center justify-center h-full gap-3 py-12">
              <span className="text-m3-error text-sm">{t("account_details.error_load")}</span>
              <Button variant="secondary" size="sm" onClick={retry}>
                {t("action.retry")}
              </Button>
            </div>
          ) : summary?.isEmpty ? (
            /* ACD-034 — no positions at all */
            <div className="flex flex-col items-center justify-center h-full gap-4 py-12">
              <p className="text-m3-on-surface-variant italic">
                {t("account_details.empty_no_positions")}
              </p>
              {/* ACD-035 — empty state CTA */}
              <Button
                variant="primary"
                size="sm"
                icon={<Plus size={14} />}
                onClick={handleAddTransaction}
              >
                {t("account_details.add_transaction")}
              </Button>
            </div>
          ) : summary?.isAllClosed ? (
            /* ACD-034 — all positions closed */
            <div className="flex flex-col items-center justify-center h-full gap-4 py-12">
              <p className="text-m3-on-surface-variant italic">
                {t("account_details.empty_all_closed")}
              </p>
              {/* ACD-035 — empty (all closed) CTA */}
              <Button
                variant="primary"
                size="sm"
                icon={<Plus size={14} />}
                onClick={handleAddTransaction}
              >
                {t("account_details.add_transaction")}
              </Button>
            </div>
          ) : (
            /* Holdings table */
            <div className="m3-table-container flex-1">
              <table className="w-full border-collapse">
                <thead className="sticky top-0 bg-m3-surface-container z-10">
                  <tr>
                    <th className="m3-th">{t("account_details.column_asset")}</th>
                    <th className="m3-th text-right">{t("account_details.column_quantity")}</th>
                    <th className="m3-th text-right">{t("account_details.column_avg_price")}</th>
                    <th className="m3-th text-right">{t("account_details.column_cost_basis")}</th>
                    <th className="m3-th">{t("transaction.column_actions")}</th>
                  </tr>
                </thead>
                <tbody>
                  {holdings.map((row) => (
                    <HoldingRow key={row.assetId} row={row} accountId={accountId} />
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
