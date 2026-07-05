import { useNavigate } from "@tanstack/react-router";
import {
  ArrowDownToLine,
  ArrowUpFromLine,
  History,
  Lock,
  LockOpen,
  Minus,
  Percent,
  Plus,
  Search,
} from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { patchModalSearch } from "@/lib/modalSearch";
import type { StoredPerfPeriod } from "@/lib/perfPeriodStorage";
import { IconButton } from "@/ui/components/button/IconButton";
import { useCachedAccounts, useCachedAssets } from "../gateway";
import { PnlCell } from "../shared/PnlCell";
import { type HoldingRowViewModel, selectPerformanceCell } from "../shared/presenter";
import type { ModalTarget, SellTarget } from "../shared/types";

type HoldingRowProps = {
  row: HoldingRowViewModel;
  accountId: string;
  onBuy: (target: ModalTarget) => void;
  onSell: (target: SellTarget) => void;
  onPriceHistory: (assetId: string) => void;
  /** Cash variant — open Deposit modal (CSH-091). */
  onDeposit?: () => void;
  /** Cash variant — open Withdrawal modal (CSH-091). */
  onWithdraw?: () => void;
  /** MKT-153/156 — toggle the asset's price-refresh lock. */
  onTogglePriceRefreshLock?: (assetId: string, currentlyBlocked: boolean) => void;
  /** FEE-011 — open the recurring fee-schedule modal for this holding. */
  onManageFee?: (assetId: string, assetName: string) => void;
  /** FEE-076 — render the Management Fees column; false when the account has the mechanism disabled. */
  showManagementFees?: boolean;
  /** ACD-054 — selected period for the Performance % column. */
  perfPeriod?: StoredPerfPeriod;
  /** As-of (read-only past-date view): hide every mutating action button. */
  readOnly?: boolean;
};

export function HoldingRow({
  row,
  accountId,
  onBuy,
  onSell,
  onPriceHistory,
  onDeposit,
  onWithdraw,
  onTogglePriceRefreshLock,
  onManageFee,
  showManagementFees = true,
  perfPeriod = "since_start",
  readOnly = false,
}: HoldingRowProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const assets = useCachedAssets();
  const accounts = useCachedAccounts();

  const buildTarget = useCallback((): ModalTarget => {
    const asset = assets.find((a) => a.id === row.assetId);
    const account = accounts.find((a) => a.id === accountId);
    return {
      accountName: account?.name ?? accountId,
      assetId: row.assetId,
      assetName: row.assetName,
      assetCurrency: asset?.currency ?? "",
      showExchangeRate: asset && account ? asset.currency !== account.currency : false,
    };
  }, [assets, accounts, accountId, row]);

  const handleBuy = useCallback(() => {
    onBuy(buildTarget());
  }, [onBuy, buildTarget]);

  const handleSell = useCallback(() => {
    onSell({ ...buildTarget(), holdingQuantityMicro: row.quantityMicro });
  }, [onSell, buildTarget, row.quantityMicro]);

  const handleViewTransactions = useCallback(() => {
    navigate({
      to: "/accounts/$accountId/transactions/$assetId",
      params: { accountId, assetId: row.assetId },
      search: { pendingTransactionAssetId: undefined },
    });
  }, [navigate, accountId, row.assetId]);

  const handleEditMissingTicker = useCallback(() => {
    patchModalSearch(navigate, {
      modal: "edit-asset",
      editAssetId: row.assetId,
      focusField: "reference",
    });
  }, [navigate, row.assetId]);

  // FXR-012 — open the Record-FX-rate modal pre-filled with the foreign asset's
  // currency → the account currency, via the shell URL-modal mount (no cross-
  // feature import). `fxFrom` is the asset currency; `fxTo` the account currency.
  const handleRecordFxRate = useCallback(() => {
    const account = accounts.find((a) => a.id === accountId);
    patchModalSearch(navigate, {
      modal: "record-fx-rate",
      fxFrom: row.assetCurrency,
      fxTo: account?.currency ?? "",
    });
  }, [navigate, accounts, accountId, row.assetCurrency]);

  const handlePriceHistory = useCallback(() => {
    onPriceHistory(row.assetId);
  }, [onPriceHistory, row.assetId]);

  const handleManageFee = useCallback(() => {
    onManageFee?.(row.assetId, row.assetName);
  }, [onManageFee, row.assetId, row.assetName]);

  const asset = assets.find((a) => a.id === row.assetId);
  const isArchived = asset?.is_archived ?? false;
  const isPriceRefreshBlocked = asset?.price_refresh_blocked ?? false;

  const handleTogglePriceRefreshLock = useCallback(() => {
    onTogglePriceRefreshLock?.(row.assetId, isPriceRefreshBlocked);
  }, [onTogglePriceRefreshLock, row.assetId, isPriceRefreshBlocked]);

  // Double-click a holding row to open the (router-driven) Edit Asset modal;
  // archived assets are not editable, mirroring the disabled edit affordance.
  const handleOpenAssetDetail = useCallback(() => {
    if (isArchived) return;
    patchModalSearch(navigate, { modal: "edit-asset", editAssetId: row.assetId });
  }, [navigate, row.assetId, isArchived]);

  // CSH-091 — cash row variant: no Buy/Sell/Inspect, only Deposit/Withdraw.
  if (row.isCash) {
    return (
      <tr className="m3-tr">
        <td className="m3-td">
          <div className="flex flex-col">
            <span className="font-medium text-m3-on-surface">{row.assetName}</span>
            <span className="text-xs text-m3-on-surface-variant">{row.assetReference}</span>
          </div>
        </td>
        <td className="m3-td text-right tabular-nums font-medium">{row.quantity}</td>
        <td className="m3-td" />
        <td className="m3-td" />
        <td className="m3-td" />
        <td className="m3-td" />
        {/* ACD-052 — cash weight % of the account's Global Value */}
        <td id={`holding-weight-pct-${row.assetId}`} className="m3-td text-right tabular-nums">
          {row.weightPct}
        </td>
        <td className="m3-td" />
        <td className="m3-td" />
        {/* DIV-072 — dividends / total-return columns are blank for the cash row */}
        <td className="m3-td" />
        <td className="m3-td" />
        {/* FEE-052 — management fees column is blank for the cash row (FEE-076: absent when disabled) */}
        {showManagementFees && <td className="m3-td" />}
        <td className="m3-td">
          <div className="flex items-center gap-1">
            {/* As-of view is read-only: Deposit/Withdraw are hidden (CSH-091). */}
            {!readOnly && (
              <>
                <IconButton
                  icon={<ArrowDownToLine size={16} />}
                  variant="success"
                  size="sm"
                  id={`action-record-deposit-${row.assetId}`}
                  aria-label={t("cash.action_record_deposit")}
                  onClick={onDeposit}
                />
                <IconButton
                  icon={<ArrowUpFromLine size={16} />}
                  variant="error"
                  size="sm"
                  id={`action-record-withdrawal-${row.assetId}`}
                  aria-label={t("cash.action_record_withdrawal")}
                  onClick={onWithdraw}
                  disabled={row.quantityMicro <= 0}
                />
              </>
            )}
            {/* CSH-110 — view the cash transaction history (deposits/withdrawals) */}
            <IconButton
              icon={<Search size={16} />}
              size="sm"
              id={`action-view-transactions-${row.assetId}`}
              aria-label={t("transaction.list_title")}
              onClick={handleViewTransactions}
            />
          </div>
        </td>
      </tr>
    );
  }

  // ACD-054 — the Performance % cell for the selected period; since-start keeps
  // the existing figure, the windowed periods read their Simple Dietz return.
  const performanceCell = selectPerformanceCell(row, perfPeriod);

  return (
    <tr className="m3-tr" onDoubleClick={readOnly ? undefined : handleOpenAssetDetail}>
      <td className="m3-td">
        <div className="flex flex-col">
          <span className="font-medium text-m3-on-surface">{row.assetName}</span>
          <span className="text-xs text-m3-on-surface-variant">{row.assetReference}</span>
        </div>
      </td>
      <td id={`holding-quantity-${row.assetId}`} className="m3-td text-right tabular-nums">
        {row.quantity}
      </td>
      <td className="m3-td text-right tabular-nums">{row.averagePrice}</td>
      {/* SEL-042 — Realized P&L */}
      <td className="m3-td text-right">
        <PnlCell value={row.realizedPnl} raw={row.realizedPnlRaw} />
      </td>
      {/* MKT-030 — Current price; MKT-032 diagnostic states; MKT-140 staleness; MKT-142 source badge */}
      <td id={`holding-current-price-${row.assetId}`} className="m3-td text-right tabular-nums">
        {row.currentPrice.kind === "present" ? (
          <div className="flex flex-col items-end gap-0.5">
            <span>{row.currentPrice.formatted}</span>
            <div className="flex items-center gap-1.5">
              {row.sourceLabel && (
                <span className="text-[10px] uppercase tracking-wide px-1.5 py-0.5 rounded bg-m3-surface-container-highest text-m3-on-surface-variant">
                  {t(row.sourceLabel)}
                </span>
              )}
              {row.staleness && (
                <span className="text-[10px] text-m3-on-surface-variant">
                  {t(row.staleness.key, row.staleness.params)}
                </span>
              )}
            </div>
          </div>
        ) : row.currentPrice.kind === "missing_ticker" ? (
          /* As-of view is read-only: the edit-missing-ticker shortcut (a write)
             is hidden; the plain state text shows instead. */
          !readOnly ? (
            <button
              type="button"
              id={`action-edit-missing-ticker-${row.assetId}`}
              onClick={handleEditMissingTicker}
              className="text-m3-primary text-sm underline-offset-2 hover:underline focus:underline focus:outline-none"
            >
              {t("mkt.price_state.missing_ticker")}
            </button>
          ) : (
            <span className="text-m3-on-surface-variant text-sm">
              {t("mkt.price_state.missing_ticker")}
            </span>
          )
        ) : (
          <span className="text-m3-on-surface-variant text-sm">
            {t("mkt.price_state.no_price_available")}
          </span>
        )}
      </td>
      {/* MKT-143 — Current value = current price × quantity (asset currency) */}
      <td id={`holding-current-value-${row.assetId}`} className="m3-td text-right tabular-nums">
        {row.currentValue}
      </td>
      {/* ACD-052 — Weight % of the account's Global Value */}
      <td id={`holding-weight-pct-${row.assetId}`} className="m3-td text-right tabular-nums">
        {row.weightPct}
      </td>
      {/* MKT-032/034 — Unrealized P&L; FXR-012 — foreign-currency holdings with a
          price but no usable rate show a Record-FX-rate shortcut instead of "—" */}
      <td className="m3-td text-right">
        {row.unrealizedPnl !== "—" ? (
          <div className="flex flex-col items-end gap-0.5">
            <PnlCell value={row.unrealizedPnl} raw={row.unrealizedPnlRaw ?? 0} />
            {row.fxStaleness && (
              <span className="text-[10px] text-m3-on-surface-variant">
                {t(row.fxStaleness.key, row.fxStaleness.params)}
              </span>
            )}
          </div>
        ) : row.currentPrice.kind === "present" && !readOnly ? (
          /* As-of view is read-only: the Record-FX-rate shortcut (a write) is
             hidden. */
          <button
            type="button"
            data-testid={`action-record-fx-rate-${row.assetId}`}
            onClick={handleRecordFxRate}
            className="text-m3-primary text-sm underline-offset-2 hover:underline focus:underline focus:outline-none"
          >
            {t("currency.action_record_fx_rate")}
          </button>
        ) : (
          <span className="text-m3-on-surface-variant">{row.unrealizedPnl}</span>
        )}
      </td>
      {/* MKT-035 / ACD-054 — Performance % over the selected period */}
      <td className="m3-td text-right tabular-nums">
        {performanceCell.formatted !== "—" ? (
          <span
            className={
              performanceCell.raw !== null && performanceCell.raw < 0
                ? "text-m3-loss"
                : "text-m3-gain"
            }
          >
            {performanceCell.formatted}
          </span>
        ) : (
          <span className="text-m3-on-surface-variant">{performanceCell.formatted}</span>
        )}
      </td>
      {/* DIV-072 — Dividends received (always shown) */}
      <td
        id={`holding-dividends-received-${row.assetId}`}
        className="m3-td text-right tabular-nums"
      >
        {row.dividendsReceived}
      </td>
      {/* DIV-072 — Total return % (price + dividends); "—" when not computable */}
      <td className="m3-td text-right tabular-nums">
        {row.totalReturnPct !== "—" ? (
          <span
            className={
              row.totalReturnPctRaw !== null && row.totalReturnPctRaw < 0
                ? "text-m3-loss"
                : "text-m3-gain"
            }
          >
            {row.totalReturnPct}
          </span>
        ) : (
          <span className="text-m3-on-surface-variant">{row.totalReturnPct}</span>
        )}
      </td>
      {/* FEE-052 — Management fees deducted; FEE-074 — the active schedule's annual
          rate rides along when one exists; FEE-076 — column absent when the account
          has the mechanism disabled */}
      {showManagementFees && (
        <td id={`holding-management-fees-${row.assetId}`} className="m3-td text-right tabular-nums">
          {row.feeRatePct !== null ? (
            <>
              {row.managementFees}{" "}
              <span
                id={`holding-fee-rate-${row.assetId}`}
                className="text-m3-on-surface-variant text-xs"
              >
                · {row.feeRatePct}
              </span>
            </>
          ) : (
            row.managementFees
          )}
        </td>
      )}
      <td className="m3-td">
        <div className="flex items-center gap-1">
          {/* As-of view is read-only: Buy/Sell/price-history/lock are hidden. */}
          {!readOnly && (
            <>
              {/* TRX-041 — Buy modal from holding row */}
              <IconButton
                icon={<Plus size={16} />}
                variant="success"
                size="sm"
                id={`action-buy-${row.assetId}`}
                aria-label={t("transaction.action_buy")}
                onClick={handleBuy}
              />
              {/* SEL-010 — Sell button; disabled when asset is archived (SEL-037) */}
              <IconButton
                icon={<Minus size={16} />}
                variant="error"
                size="sm"
                id={`action-sell-${row.assetId}`}
                aria-label={t("transaction.action_sell")}
                onClick={handleSell}
                disabled={isArchived}
              />
              {/* MKT-070 — Price history button (active holdings only); add-price lives inside */}
              {row.canEnterPrice && (
                <IconButton
                  icon={<History size={16} />}
                  size="sm"
                  id={`action-price-history-${row.assetId}`}
                  aria-label={t("account_details.action_price_history")}
                  onClick={handlePriceHistory}
                />
              )}
              {/* MKT-153 — Lock toggle: blocks/allows automated price fetches (ADR-014) */}
              {onTogglePriceRefreshLock && (
                <IconButton
                  icon={isPriceRefreshBlocked ? <Lock size={16} /> : <LockOpen size={16} />}
                  size="sm"
                  id={`action-toggle-price-refresh-${row.assetId}`}
                  aria-label={t(
                    isPriceRefreshBlocked ? "mkt.lock.action_unblock" : "mkt.lock.action_block",
                  )}
                  onClick={handleTogglePriceRefreshLock}
                />
              )}
              {/* FEE-011 — manage the recurring management-fee schedule for this holding */}
              {onManageFee && (
                <IconButton
                  icon={<Percent size={16} />}
                  size="sm"
                  id={`action-manage-fee-${row.assetId}`}
                  aria-label={t("account_details.action_manage_fee")}
                  onClick={handleManageFee}
                />
              )}
            </>
          )}
          <IconButton
            icon={<Search size={16} />}
            size="sm"
            id={`action-view-transactions-${row.assetId}`}
            aria-label={t("transaction.list_title")}
            onClick={handleViewTransactions}
          />
        </div>
      </td>
    </tr>
  );
}
