import { useNavigate, useParams } from "@tanstack/react-router";
import { Plus } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { HoldingDetail } from "@/bindings";
import { logger } from "@/lib/logger";
import { Button } from "@/ui/components/button/Button";
import { BuyTransactionModal } from "../buy_transaction/BuyTransactionModal";
import { SellTransactionModal } from "../sell_transaction/SellTransactionModal";
import type { ModalTarget, SellTarget } from "../shared/types";
import { ClosedHoldingRow } from "./ClosedHoldingRow";
import { HoldingRow } from "./HoldingRow";
import { PriceModal } from "./PriceModal";
import { useAccountDetails } from "./useAccountDetails";

export function AccountDetailsView() {
  const { t } = useTranslation();
  const { accountId } = useParams({ from: "/accounts/$accountId" });
  const navigate = useNavigate();
  const { isLoading, error, retry, holdings, holdingDetails, closedHoldings, summary } =
    useAccountDetails(accountId);

  const [buyTarget, setBuyTarget] = useState<ModalTarget | null>(null);
  const [sellTarget, setSellTarget] = useState<SellTarget | null>(null);
  const [priceTarget, setPriceTarget] = useState<HoldingDetail | null>(null);

  useEffect(() => {
    logger.info("[AccountDetailsView] mounted");
  }, []);

  const handleAddTransaction = useCallback(() => {
    navigate({
      to: "/transactions/new",
      search: { prefillAccountId: accountId, prefillAssetId: undefined },
    });
  }, [navigate, accountId]);

  const handleBuyClose = useCallback(() => setBuyTarget(null), []);
  const handleSellClose = useCallback(() => setSellTarget(null), []);
  const handlePriceClose = useCallback(() => setPriceTarget(null), []);

  const handleBuySuccess = useCallback(() => {
    setBuyTarget(null);
    retry();
  }, [retry]);

  const handleSellSuccess = useCallback(() => {
    setSellTarget(null);
    retry();
  }, [retry]);

  // MKT-028 — close modal on success; re-fetch happens via AssetPriceUpdated event (MKT-036)
  const handlePriceSuccess = useCallback(() => {
    setPriceTarget(null);
  }, []);

  // MKT-010/013 — find the raw HoldingDetail (no extra fetch) for the price modal
  const handleEnterPrice = useCallback(
    (assetId: string) => {
      const holding = holdingDetails.find((h) => h.asset_id === assetId);
      if (holding) setPriceTarget(holding);
    },
    [holdingDetails],
  );

  const hasActiveHoldings = holdings.length > 0;
  const hasClosedHoldings = summary?.hasClosedHoldings ?? false;

  return (
    <div className="flex h-full flex-col gap-4 overflow-hidden py-2 px-2">
      <div className="flex-1 flex flex-col min-w-0 bg-m3-surface-container rounded-[28px] shadow-elevation-1 overflow-hidden">
        {/* Summary header */}
        <div className="px-6 py-4 bg-m3-surface-container-high">
          {isLoading ? (
            <div className="h-4 w-32 bg-m3-surface-variant rounded animate-pulse" />
          ) : summary ? (
            <div className="flex items-center justify-between">
              <div className="flex gap-6 flex-wrap">
                <p className="text-sm text-m3-on-surface-variant">
                  {t("account_details.total_cost_basis")}:{" "}
                  <span className="font-semibold text-m3-on-surface">{summary.totalCostBasis}</span>
                </p>
                {summary.totalRealizedPnlRaw !== 0 && (
                  <p className="text-sm text-m3-on-surface-variant">
                    {t("account_details.total_realized_pnl")}:{" "}
                    <span
                      className={`font-semibold ${
                        summary.totalRealizedPnlRaw < 0 ? "text-m3-error" : "text-m3-success"
                      }`}
                    >
                      {summary.totalRealizedPnl}
                    </span>
                  </p>
                )}
                {/* MKT-041 — total unrealized P&L */}
                {summary.totalUnrealizedPnl !== "—" && (
                  <p className="text-sm text-m3-on-surface-variant">
                    {t("account_details.total_unrealized_pnl")}:{" "}
                    <span className="font-semibold text-m3-on-surface">
                      {summary.totalUnrealizedPnl}
                    </span>
                  </p>
                )}
              </div>
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
          ) : (
            <div className="flex flex-col">
              {/* Active holdings table */}
              {hasActiveHoldings && (
                <div className="m3-table-container">
                  <table className="w-full border-collapse">
                    <thead className="sticky top-0 bg-m3-surface-container z-10">
                      <tr>
                        <th className="m3-th">{t("account_details.column_asset")}</th>
                        <th className="m3-th text-right">{t("account_details.column_quantity")}</th>
                        <th className="m3-th text-right">
                          {t("account_details.column_avg_price")}
                        </th>
                        <th className="m3-th text-right">
                          {t("account_details.column_cost_basis")}
                        </th>
                        {/* SEL-042 — Realized P&L column */}
                        <th className="m3-th text-right">
                          {t("account_details.column_realized_pnl")}
                        </th>
                        {/* MKT-030 — Current price column */}
                        <th className="m3-th text-right">
                          {t("account_details.column_current_price")}
                        </th>
                        {/* MKT-032/034 — Unrealized P&L column */}
                        <th className="m3-th text-right">
                          {t("account_details.column_unrealized_pnl")}
                        </th>
                        {/* MKT-035 — Performance % column */}
                        <th className="m3-th text-right">
                          {t("account_details.column_performance_pct")}
                        </th>
                        <th className="m3-th">{t("transaction.column_actions")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {holdings.map((row) => (
                        <HoldingRow
                          key={row.assetId}
                          row={row}
                          accountId={accountId}
                          onBuy={setBuyTarget}
                          onSell={setSellTarget}
                          onEnterPrice={handleEnterPrice}
                        />
                      ))}
                    </tbody>
                  </table>
                </div>
              )}

              {/* ACD-048 — Closed positions section */}
              {hasClosedHoldings && (
                <div className="mt-2">
                  <div className="px-6 py-3 bg-m3-surface-container-high">
                    <h3 className="text-sm font-semibold text-m3-on-surface-variant uppercase tracking-wide">
                      {t("account_details.closed_positions_header")}
                    </h3>
                  </div>
                  <table className="w-full border-collapse">
                    <thead className="sticky top-0 bg-m3-surface-container z-10">
                      <tr>
                        <th className="m3-th">{t("account_details.column_asset")}</th>
                        {/* ACD-049 — P&L and last sold date */}
                        <th className="m3-th text-right">
                          {t("account_details.column_realized_pnl")}
                        </th>
                        <th className="m3-th text-right">
                          {t("account_details.column_last_sold_date")}
                        </th>
                        <th className="m3-th">{t("transaction.column_actions")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {closedHoldings.map((row) => (
                        <ClosedHoldingRow key={row.assetId} row={row} accountId={accountId} />
                      ))}
                    </tbody>
                  </table>
                </div>
              )}

              {/* No active holdings but has closed — show CTA */}
              {!hasActiveHoldings && (
                <div className="flex flex-col items-center justify-center gap-4 py-8">
                  <p className="text-m3-on-surface-variant italic">
                    {t("account_details.empty_all_closed")}
                  </p>
                  <Button
                    variant="primary"
                    size="sm"
                    icon={<Plus size={14} />}
                    onClick={handleAddTransaction}
                  >
                    {t("account_details.add_transaction")}
                  </Button>
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* TRX-041 — Buy modal from holding row */}
      {buyTarget && (
        <BuyTransactionModal
          isOpen
          onClose={handleBuyClose}
          accountId={accountId}
          accountName={buyTarget.accountName}
          assetId={buyTarget.assetId}
          assetName={buyTarget.assetName}
          assetCurrency={buyTarget.assetCurrency}
          showExchangeRate={buyTarget.showExchangeRate}
          onSubmitSuccess={handleBuySuccess}
        />
      )}

      {/* SEL-010 — Sell modal */}
      {sellTarget && (
        <SellTransactionModal
          isOpen
          onClose={handleSellClose}
          accountId={accountId}
          accountName={sellTarget.accountName}
          assetId={sellTarget.assetId}
          assetName={sellTarget.assetName}
          assetCurrency={sellTarget.assetCurrency}
          holdingQuantityMicro={sellTarget.holdingQuantityMicro}
          showExchangeRate={sellTarget.showExchangeRate}
          onSubmitSuccess={handleSellSuccess}
        />
      )}

      {/* MKT-010 — Enter price modal */}
      {priceTarget && (
        <PriceModal
          isOpen
          onClose={handlePriceClose}
          holding={priceTarget}
          onSubmitSuccess={handlePriceSuccess}
        />
      )}
    </div>
  );
}
