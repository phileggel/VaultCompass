import { useNavigate, useParams } from "@tanstack/react-router";
import { ChevronDown, History, RefreshCw, ScrollText, TrendingUp } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  getClosedSectionOpen,
  setClosedSectionOpen as persistClosedSectionOpen,
} from "@/lib/closedSectionStorage";
import { logger } from "@/lib/logger";
import { Button } from "@/ui/components/button/Button";
import { FAB } from "@/ui/components/fab/FAB";
import { BuyTransactionModal } from "../buy_transaction/BuyTransactionModal";
import { DepositTransactionModal } from "../deposit_transaction/DepositTransactionModal";
import { DividendTransactionModal } from "../dividend_transaction/DividendTransactionModal";
import { FreeSharesModal } from "../free_shares_transaction/FreeSharesModal";
import { HoldingsAsOfModal } from "../holdings_as_of/HoldingsAsOfModal";
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
  // DIV-012 — consolidated header "Add" dropdown open/close state.
  const [addMenuOpen, setAddMenuOpen] = useState(false);
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

  const runFromAddMenu = useCallback((action: () => void) => {
    setAddMenuOpen(false);
    action();
  }, []);

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
            <div className="flex items-center justify-between">
              <div className="flex gap-6 flex-wrap">
                <p className="text-sm text-m3-on-surface-variant">
                  {t("account_details.total_cost_basis")}:{" "}
                  <span className="font-semibold text-m3-on-surface">
                    {view.summary.totalCostBasis}
                  </span>
                </p>
                {view.summary.totalRealizedPnlRaw !== 0 && (
                  <p className="text-sm text-m3-on-surface-variant">
                    {t("account_details.total_realized_pnl")}:{" "}
                    <span
                      className={`font-semibold ${
                        view.summary.totalRealizedPnlRaw < 0 ? "text-m3-error" : "text-m3-success"
                      }`}
                    >
                      {view.summary.totalRealizedPnl}
                    </span>
                  </p>
                )}
                {/* MKT-041 — total unrealized P&L */}
                {view.summary.totalUnrealizedPnl !== "—" && (
                  <p className="text-sm text-m3-on-surface-variant">
                    {t("account_details.total_unrealized_pnl")}:{" "}
                    <span className="font-semibold text-m3-on-surface">
                      {view.summary.totalUnrealizedPnl}
                    </span>
                  </p>
                )}
                {/* CSH-094 — Global Value (cash + priced holdings, account currency) */}
                <p className="text-sm text-m3-on-surface-variant">
                  {t("account_details.total_global_value")}:{" "}
                  <span className="font-semibold text-m3-on-surface">
                    {view.summary.totalGlobalValue}
                  </span>
                </p>
                {/* DIV-073 — total dividends received (shown only when any recorded) */}
                {view.summary.totalDividendsReceivedRaw !== 0 && (
                  <p
                    id="account-details-total-dividends"
                    className="text-sm text-m3-on-surface-variant"
                  >
                    {t("account_details.total_dividends_received")}:{" "}
                    <span className="font-semibold text-m3-on-surface">
                      {view.summary.totalDividendsReceived}
                    </span>
                  </p>
                )}
              </div>
              {/* TRX-055 — open balance always accessible (migration tool for any account state) */}
              {/* ACD-036 — add transaction only when active holdings exist */}
              <div className="flex gap-2">
                {/* PRF-010 — per-account "Performance" entry point (navigates via router path) */}
                <Button
                  id="account-details-performance"
                  variant="secondary"
                  size="sm"
                  icon={<TrendingUp size={14} />}
                  onClick={() =>
                    void navigate({
                      to: "/accounts/$accountId/performance",
                      params: { accountId },
                    })
                  }
                  aria-label={t("account_details.action_performance")}
                >
                  {t("account_details.action_performance")}
                </Button>

                {/* TRX-036 — per-account overall transaction journal */}
                <Button
                  id="account-details-journal"
                  variant="secondary"
                  size="sm"
                  icon={<ScrollText size={14} />}
                  onClick={() =>
                    void navigate({
                      to: "/accounts/$accountId/journal",
                      params: { accountId },
                    })
                  }
                  aria-label={t("account_details.action_journal")}
                >
                  {t("account_details.action_journal")}
                </Button>
                {/* Read-only "holdings as of a past date" entry point */}
                <Button
                  id="account-details-as-of"
                  variant="secondary"
                  size="sm"
                  icon={<History size={14} />}
                  onClick={view.handleAsOfOpen}
                  aria-label={t("account_details.action_holdings_as_of")}
                >
                  {t("account_details.action_holdings_as_of")}
                </Button>
                {/* MKT-131 — per-account "Refresh prices" entry point */}
                <Button
                  id="account-details-refresh-prices"
                  variant="secondary"
                  size="sm"
                  icon={<RefreshCw size={14} />}
                  loading={isRefreshPending}
                  onClick={() => void refreshPrices()}
                  aria-label={t("account_details.action_refresh_prices")}
                >
                  {t("account_details.action_refresh_prices")}
                </Button>
                {/* DIV-012 — consolidated "Record" dropdown (Open balance /
                    Dividend / Free shares). Cash Deposit/Withdraw are NOT here —
                    they live on the always-present cash row (CSH-019). */}
                <div className="relative">
                  <Button
                    id="account-details-add-menu"
                    variant="secondary"
                    size="sm"
                    icon={<ChevronDown size={14} />}
                    aria-haspopup="menu"
                    aria-expanded={addMenuOpen}
                    onClick={() => setAddMenuOpen((open) => !open)}
                    onKeyDown={(e) => {
                      if (e.key === "Escape") setAddMenuOpen(false);
                    }}
                  >
                    {t("account_details.action_add_menu")}
                  </Button>
                  {addMenuOpen && (
                    <>
                      {/* click-away backdrop */}
                      <button
                        type="button"
                        aria-hidden="true"
                        tabIndex={-1}
                        className="fixed inset-0 z-20 cursor-default"
                        onClick={() => setAddMenuOpen(false)}
                      />
                      <div
                        role="menu"
                        aria-label={t("account_details.action_add_menu")}
                        className="absolute right-0 mt-1 z-30 min-w-[200px] rounded-2xl bg-m3-surface-container-high shadow-elevation-2 py-1"
                        onKeyDown={(e) => {
                          if (e.key === "Escape") setAddMenuOpen(false);
                        }}
                      >
                        {/* CSH-019 — cash Deposit/Withdraw live on the cash row, not this menu */}
                        {/* TRX-055 — Open balance (keeps its shipped "Add a position" label) */}
                        <button
                          type="button"
                          role="menuitem"
                          id="add-menu-open-balance"
                          className="w-full text-left px-4 py-2 text-sm text-m3-on-surface hover:bg-m3-surface-container-highest"
                          onClick={() => runFromAddMenu(view.handleOpenBalanceOpen)}
                        >
                          {t("account_details.action_open_balance")}
                        </button>
                        {/* DIV-010 — Record dividend */}
                        <button
                          type="button"
                          role="menuitem"
                          id="add-menu-dividend"
                          className="w-full text-left px-4 py-2 text-sm text-m3-on-surface hover:bg-m3-surface-container-highest"
                          onClick={() => runFromAddMenu(view.handleDividendOpen)}
                        >
                          {t("account_details.action_record_dividend")}
                        </button>
                        {/* FSD-010 — Record free shares */}
                        <button
                          type="button"
                          role="menuitem"
                          id="add-menu-free-shares"
                          className="w-full text-left px-4 py-2 text-sm text-m3-on-surface hover:bg-m3-surface-container-highest"
                          onClick={() => runFromAddMenu(view.handleFreeSharesOpen)}
                        >
                          {t("account_details.action_record_free_shares")}
                        </button>
                      </div>
                    </>
                  )}
                </div>
              </div>
            </div>
          ) : null}
        </div>

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

      {/* Read-only holdings-as-of modal (mounted only while open so the hook
          re-seeds the date to today on every open) */}
      {view.asOfOpen && (
        <HoldingsAsOfModal isOpen onClose={view.handleAsOfClose} accountId={accountId} />
      )}

      {/* ACD-035/036 — add-transaction entry point is a global FAB (replaces the
          former contextual "Add Transaction" buttons in the header / empty states) */}
      <FAB
        id="account-details-add-transaction-fab"
        onClick={view.handleAddTransaction}
        label={t("account_details.add_transaction")}
      />
    </div>
  );
}
