import { useNavigate, useParams } from "@tanstack/react-router";
import {
  ChevronDown,
  Coins,
  Gift,
  PlusCircle,
  RefreshCw,
  RotateCcw,
  ScrollText,
  TrendingUp,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  getClosedSectionOpen,
  setClosedSectionOpen as persistClosedSectionOpen,
} from "@/lib/closedSectionStorage";
import { logger } from "@/lib/logger";
import { Button } from "@/ui/components/button/Button";
import { IconButton } from "@/ui/components/button/IconButton";
import { FAB } from "@/ui/components/fab/FAB";
import { DateField } from "@/ui/components/field/DateField";
import { BuyTransactionModal } from "../buy_transaction/BuyTransactionModal";
import { DepositTransactionModal } from "../deposit_transaction/DepositTransactionModal";
import { DividendTransactionModal } from "../dividend_transaction/DividendTransactionModal";
import { FreeSharesModal } from "../free_shares_transaction/FreeSharesModal";
import { OpenBalanceModal } from "../open_balance/OpenBalanceModal";
import { PriceHistoryModal } from "../price_history/PriceHistoryModal";
import { useRefreshAccountPrices } from "../refresh_prices/useRefreshAccountPrices";
import { SellTransactionModal } from "../sell_transaction/SellTransactionModal";
import { WithdrawalTransactionModal } from "../withdrawal_transaction/WithdrawalTransactionModal";
import { ClosedHoldingRow } from "./ClosedHoldingRow";
import { HoldingRow } from "./HoldingRow";
import { useAccountDetailsView } from "./useAccountDetailsView";

export function AccountDetailsView() {
  const { t } = useTranslation();
  const { accountId } = useParams({ from: "/accounts/$accountId" });
  const navigate = useNavigate();
  const view = useAccountDetailsView(accountId);
  const { isPending: isRefreshPending, refresh: refreshPrices } =
    useRefreshAccountPrices(accountId);
  // ACD-048 — closed positions section is collapsible; fold state is remembered per account.
  const [closedSectionOpen, setClosedSectionOpen] = useState(() => getClosedSectionOpen(accountId));

  // Restore the remembered fold state when switching to another account without a remount.
  useEffect(() => {
    setClosedSectionOpen(getClosedSectionOpen(accountId));
  }, [accountId]);

  const toggleClosedSection = useCallback(() => {
    const next = !closedSectionOpen;
    setClosedSectionOpen(next);
    persistClosedSectionOpen(accountId, next);
  }, [accountId, closedSectionOpen]);

  useEffect(() => {
    logger.info("[AccountDetailsView] mounted");
  }, []);

  return (
    <div className="flex h-full flex-col gap-4 overflow-hidden py-2 px-2">
      <div className="flex-1 flex flex-col min-w-0 bg-m3-surface-container rounded-[28px] shadow-elevation-1 overflow-hidden">
        {/* Summary header */}
        <div className="px-6 py-4 bg-m3-surface-container-high">
          {view.isLoading ? (
            <div className="h-4 w-32 bg-m3-surface-variant rounded animate-pulse" />
          ) : view.summary ? (
            <div className="flex items-center justify-between gap-4">
              <div className="flex items-center gap-4 min-w-0">
                {/* Read-only "holdings as of a past date" selector. Leads the
                    header, label-less (named via aria-label, F24). Picking a past
                    date switches the page into the read-only as-of view; clearing
                    it (or picking today) returns to the live view. */}
                <div className="w-44 shrink-0">
                  <DateField
                    id="account-details-as-of-date"
                    aria-label={t("account_details.as_of_date_label")}
                    title={t("account_details.as_of_date_label")}
                    placeholder={t("account_details.as_of_today_placeholder")}
                    value={view.asOfDisplayDate}
                    onChange={(e) => view.setAsOfDate(e.target.value)}
                  />
                </div>
                {/* CSH-094 — Global Value (cash + priced holdings, account currency) */}
                <p className="text-sm text-m3-on-surface-variant whitespace-nowrap">
                  {t("account_details.total_global_value")}:{" "}
                  <span className="font-semibold text-m3-on-surface">
                    {view.summary.totalGlobalValue}
                  </span>
                </p>
              </div>
              {/* TRX-055 — open balance always accessible (migration tool for any account state) */}
              {/* ACD-036 — header actions: big square icon buttons, name shown as tooltip */}
              <div className="flex items-center gap-2">
                {/* PRF-010 — per-account "Performance" entry point (navigates via router path) */}
                <IconButton
                  id="account-details-performance"
                  shape="square"
                  size="lg"
                  variant="tonal"
                  icon={<TrendingUp size={20} />}
                  onClick={() =>
                    void navigate({
                      to: "/accounts/$accountId/performance",
                      params: { accountId },
                    })
                  }
                  aria-label={t("account_details.action_performance")}
                  title={t("account_details.action_performance")}
                />
                {/* TRX-036 — per-account overall transaction journal */}
                <IconButton
                  id="account-details-journal"
                  shape="square"
                  size="lg"
                  variant="tonal"
                  icon={<ScrollText size={20} />}
                  onClick={() =>
                    void navigate({
                      to: "/accounts/$accountId/journal",
                      params: { accountId },
                    })
                  }
                  aria-label={t("account_details.action_journal")}
                  title={t("account_details.action_journal")}
                />
                {/* MKT-131 — per-account "Refresh prices"; hidden in read-only as-of */}
                {!view.isAsOf && (
                  <IconButton
                    id="account-details-refresh-prices"
                    shape="square"
                    size="lg"
                    variant="tonal"
                    icon={
                      <RefreshCw size={20} className={isRefreshPending ? "animate-spin" : ""} />
                    }
                    disabled={isRefreshPending}
                    onClick={() => void refreshPrices()}
                    aria-label={t("account_details.action_refresh_prices")}
                    title={t("account_details.action_refresh_prices")}
                  />
                )}
                {/* DIV-012 — Record actions, flattened from the former dropdown into
                    big square buttons. Cash Deposit/Withdraw are NOT here — they live
                    on the always-present cash row (CSH-019). Hidden in read-only as-of. */}
                {!view.isAsOf && (
                  <>
                    {/* TRX-055 — Open balance ("Add a position") */}
                    <IconButton
                      id="add-menu-open-balance"
                      shape="square"
                      size="lg"
                      variant="tonal"
                      icon={<PlusCircle size={20} />}
                      onClick={view.handleOpenBalanceOpen}
                      aria-label={t("account_details.action_open_balance")}
                      title={t("account_details.action_open_balance")}
                    />
                    {/* DIV-010 — Record dividend */}
                    <IconButton
                      id="add-menu-dividend"
                      shape="square"
                      size="lg"
                      variant="tonal"
                      icon={<Coins size={20} />}
                      onClick={view.handleDividendOpen}
                      aria-label={t("account_details.action_record_dividend")}
                      title={t("account_details.action_record_dividend")}
                    />
                    {/* FSD-010 — Record free shares */}
                    <IconButton
                      id="add-menu-free-shares"
                      shape="square"
                      size="lg"
                      variant="tonal"
                      icon={<Gift size={20} />}
                      onClick={view.handleFreeSharesOpen}
                      aria-label={t("account_details.action_record_free_shares")}
                      title={t("account_details.action_record_free_shares")}
                    />
                  </>
                )}
              </div>
            </div>
          ) : null}
        </div>

        {/* Read-only as-of banner: shown while a past date is selected. */}
        {view.isAsOf && (
          <div
            id="account-details-as-of-banner"
            className="flex items-center justify-between gap-3 px-6 py-2 bg-m3-tertiary-container text-m3-on-tertiary-container text-sm"
          >
            <span>{t("account_details.as_of_banner", { date: view.asOfDateFormatted })}</span>
            <Button
              id="account-details-as-of-reset"
              variant="secondary"
              size="sm"
              icon={<RotateCcw size={14} />}
              onClick={() => view.setAsOfDate("")}
            >
              {t("account_details.as_of_back_to_today")}
            </Button>
          </div>
        )}

        {/* Content */}
        <div className="flex-1 overflow-auto">
          {view.isLoading ? (
            /* ACD-037 — loading skeletons */
            <div className="animate-pulse p-4 space-y-3">
              {[1, 2, 3].map((i) => (
                <div key={i} className="h-10 bg-m3-surface-variant rounded-lg" />
              ))}
            </div>
          ) : view.error ? (
            /* ACD-038 — error state */
            <div className="flex flex-col items-center justify-center h-full gap-3 py-12">
              <span className="text-m3-error text-sm">{t("account_details.error_load")}</span>
              <Button variant="secondary" size="sm" onClick={view.retry}>
                {t("action.retry")}
              </Button>
            </div>
          ) : (
            <div className="flex flex-col">
              {/* CSH-095 — active holdings table; the Cash row is always present
                  (even at €0), so this table always renders. */}
              <div className="m3-table-container">
                <table className="w-full border-collapse">
                  <thead className="sticky top-0 bg-m3-surface-container z-10">
                    <tr>
                      <th className="m3-th">{t("account_details.column_asset")}</th>
                      <th className="m3-th text-right">{t("account_details.column_quantity")}</th>
                      <th className="m3-th text-right">{t("account_details.column_avg_price")}</th>
                      {/* SEL-042 — Realized P&L column */}
                      <th className="m3-th text-right">
                        {t("account_details.column_realized_pnl")}
                      </th>
                      {/* MKT-030 — Current price column */}
                      <th className="m3-th text-right">
                        {t("account_details.column_current_price")}
                      </th>
                      {/* MKT-143 — Current value column */}
                      <th className="m3-th text-right">
                        {t("account_details.column_current_value")}
                      </th>
                      {/* MKT-032/034 — Unrealized P&L column */}
                      <th className="m3-th text-right">
                        {t("account_details.column_unrealized_pnl")}
                      </th>
                      {/* MKT-035 — Performance % column */}
                      <th className="m3-th text-right">
                        {t("account_details.column_performance_pct")}
                      </th>
                      {/* DIV-072 — Dividends received column */}
                      <th className="m3-th text-right">
                        {t("account_details.column_dividends_received")}
                      </th>
                      {/* DIV-072 — Total return % column */}
                      <th className="m3-th text-right">
                        {t("account_details.column_total_return_pct")}
                      </th>
                      <th className="m3-th">{t("transaction.column_actions")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {view.holdings.map((row) => (
                      <HoldingRow
                        key={row.assetId}
                        row={row}
                        accountId={accountId}
                        onBuy={view.handleBuyOpen}
                        onSell={view.handleSellOpen}
                        onPriceHistory={view.handlePriceHistory}
                        onDeposit={view.handleDepositOpen}
                        onWithdraw={view.handleWithdrawalOpen}
                        onTogglePriceRefreshLock={view.handleTogglePriceRefreshLock}
                        readOnly={view.isAsOf}
                      />
                    ))}
                  </tbody>
                </table>
              </div>

              {/* ACD-034 / CSH-098 — asset-positions empty message; the Cash row is
                  excluded from the count, so a cash-only account still reads "No positions yet" */}
              {!view.hasNonCashActiveHoldings && (
                <div className="flex flex-col items-center justify-center gap-4 py-8">
                  <p className="text-m3-on-surface-variant italic">
                    {t(
                      view.summary?.isAllClosed
                        ? "account_details.empty_all_closed"
                        : "account_details.empty_no_positions",
                    )}
                  </p>
                </div>
              )}

              {/* ACD-048 — Closed positions section (collapsible) */}
              {view.hasClosedHoldings && (
                <div className="mt-2">
                  <button
                    type="button"
                    id="account-closed-positions-toggle"
                    aria-expanded={closedSectionOpen}
                    onClick={toggleClosedSection}
                    className="w-full flex items-center gap-2 px-6 py-3 bg-m3-surface-container-high text-left hover:bg-m3-surface-container-highest"
                  >
                    <ChevronDown
                      size={16}
                      className={`text-m3-on-surface-variant transition-transform ${
                        closedSectionOpen ? "" : "-rotate-90"
                      }`}
                    />
                    <h3 className="text-sm font-semibold text-m3-on-surface-variant uppercase tracking-wide">
                      {t("account_details.closed_positions_header")}
                    </h3>
                  </button>
                  {closedSectionOpen && (
                    <table className="w-full border-collapse">
                      <thead className="sticky top-0 bg-m3-surface-container z-10">
                        <tr>
                          <th className="m3-th">{t("account_details.column_asset")}</th>
                          {/* ACD-049 — P&L and last sold date */}
                          <th className="m3-th text-right">
                            {t("account_details.column_realized_pnl")}
                          </th>
                          {/* DIV-073 — dividends received + total revenues */}
                          <th className="m3-th text-right">
                            {t("account_details.column_dividends_received")}
                          </th>
                          <th className="m3-th text-right">
                            {t("account_details.column_total_revenues")}
                          </th>
                          <th className="m3-th text-right">
                            {t("account_details.column_last_sold_date")}
                          </th>
                          <th className="m3-th">{t("transaction.column_actions")}</th>
                        </tr>
                      </thead>
                      <tbody>
                        {view.closedHoldings.map((row) => (
                          <ClosedHoldingRow key={row.assetId} row={row} accountId={accountId} />
                        ))}
                      </tbody>
                    </table>
                  )}
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* TRX-041 — Buy modal from holding row */}
      {view.buyTarget && (
        <BuyTransactionModal
          isOpen
          onClose={view.handleBuyClose}
          accountId={accountId}
          accountName={view.buyTarget.accountName}
          assetId={view.buyTarget.assetId}
          assetName={view.buyTarget.assetName}
          assetCurrency={view.buyTarget.assetCurrency}
          showExchangeRate={view.buyTarget.showExchangeRate}
          onSubmitSuccess={view.handleBuySuccess}
        />
      )}

      {/* SEL-010 — Sell modal */}
      {view.sellTarget && (
        <SellTransactionModal
          isOpen
          onClose={view.handleSellClose}
          accountId={accountId}
          accountName={view.sellTarget.accountName}
          assetId={view.sellTarget.assetId}
          assetName={view.sellTarget.assetName}
          assetCurrency={view.sellTarget.assetCurrency}
          holdingQuantityMicro={view.sellTarget.holdingQuantityMicro}
          showExchangeRate={view.sellTarget.showExchangeRate}
          onSubmitSuccess={view.handleSellSuccess}
        />
      )}

      {/* MKT-072 — Price history modal (price entry lives inside via "Add price") */}
      {view.historyTarget && (
        <PriceHistoryModal
          isOpen
          onClose={view.handleHistoryClose}
          holding={view.historyTarget}
          accountId={accountId}
          priceableAssets={view.priceableAssets}
        />
      )}

      {/* TRX-055 — Open balance modal (account pre-filled, user picks asset inside) */}
      <OpenBalanceModal
        isOpen={view.openBalanceOpen}
        onClose={view.handleOpenBalanceClose}
        accountId={accountId}
        accountName={view.summary?.accountName ?? ""}
        assetId=""
        assetName=""
        onSubmitSuccess={view.handleOpenBalanceSuccess}
      />

      {/* CSH-022 — Deposit modal. Mounted only while open so the hook re-seeds the
          date from the stored last-operation date on every open (matches the
          buy/sell/dividend modals). */}
      {view.depositOpen && (
        <DepositTransactionModal
          isOpen
          onClose={view.handleDepositClose}
          accountId={accountId}
          accountName={view.summary?.accountName ?? ""}
          accountCurrency={view.accountCurrency}
          onSubmitSuccess={view.handleDepositSuccess}
        />
      )}

      {/* CSH-032 — Withdrawal modal. Mounted only while open (see deposit note). */}
      {view.withdrawalOpen && (
        <WithdrawalTransactionModal
          isOpen
          onClose={view.handleWithdrawalClose}
          accountId={accountId}
          accountName={view.summary?.accountName ?? ""}
          accountCurrency={view.accountCurrency}
          onSubmitSuccess={view.handleWithdrawalSuccess}
        />
      )}

      {/* DIV-010/020 — Dividend modal (paying asset chosen inside) */}
      {view.dividendOpen && (
        <DividendTransactionModal
          isOpen
          onClose={view.handleDividendClose}
          accountId={accountId}
          accountCurrency={view.accountCurrency}
          heldAssets={view.dividendPayingAssets}
          onSubmitSuccess={view.handleDividendSuccess}
          onRecorded={view.handleDividendRecorded}
        />
      )}

      {/* FSD-010/020 — Free-shares modal (distributing asset chosen inside) */}
      {view.freeSharesOpen && (
        <FreeSharesModal
          isOpen
          onClose={view.handleFreeSharesClose}
          accountId={accountId}
          heldAssets={view.dividendPayingAssets}
          onSubmitSuccess={view.handleFreeSharesSuccess}
        />
      )}

      {/* ACD-035/036 — add-transaction entry point is a global FAB (replaces the
          former contextual "Add Transaction" buttons in the header / empty states).
          Hidden in the read-only as-of view. */}
      {!view.isAsOf && (
        <FAB
          id="account-details-add-transaction-fab"
          onClick={view.handleAddTransaction}
          label={t("account_details.add_transaction")}
        />
      )}
    </div>
  );
}
