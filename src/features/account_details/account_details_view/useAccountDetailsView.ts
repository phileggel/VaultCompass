import { useNavigate } from "@tanstack/react-router";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { HoldingDetail } from "@/bindings";
import { logger } from "@/lib/logger";
import { patchModalSearch } from "@/lib/modalSearch";
import { useAppStore } from "@/lib/store";
import { useSnackbar } from "@/ui/components/snackbar/snackbarStore";
import { accountDetailsGateway } from "../gateway";
import { isCashAsset, priceRefreshLockErrorToI18n } from "../shared/presenter";
import type { ModalTarget, SellTarget } from "../shared/types";
import { useAccountDetails } from "./useAccountDetails";

/**
 * Orchestration hook for AccountDetailsView. Bundles the data hook
 * (`useAccountDetails`) with the modal-state machine (Buy / Sell / Price /
 * Price-history / Open-balance / Deposit / Withdrawal) so the view component
 * stays a thin renderer.
 *
 * Splitting state out of the .tsx avoids the temptation to test the view's
 * orchestration via DOM-level RTL setups; this hook can be unit-tested in
 * isolation.
 */
export function useAccountDetailsView(accountId: string) {
  const navigate = useNavigate();
  const data = useAccountDetails(accountId);
  const accounts = useAppStore((state) => state.accounts);
  const fetchAssets = useAppStore((state) => state.fetchAssets);
  const showSnackbar = useSnackbar();
  const { t } = useTranslation();
  const accountCurrency = accounts.find((a) => a.id === accountId)?.currency ?? "";

  // ---------------------------------------------------------------------------
  // Modal targets / open flags
  // ---------------------------------------------------------------------------
  const [buyTarget, setBuyTarget] = useState<ModalTarget | null>(null);
  const [sellTarget, setSellTarget] = useState<SellTarget | null>(null);
  const [historyTarget, setHistoryTarget] = useState<HoldingDetail | null>(null);
  const [openBalanceOpen, setOpenBalanceOpen] = useState(false);
  const [depositOpen, setDepositOpen] = useState(false);
  const [withdrawalOpen, setWithdrawalOpen] = useState(false);
  const [dividendOpen, setDividendOpen] = useState(false);
  const [freeSharesOpen, setFreeSharesOpen] = useState(false);

  // ---------------------------------------------------------------------------
  // Handlers
  // ---------------------------------------------------------------------------
  // ACD-035/036 — open the Add Transaction modal in place via the shell-mounted
  // AddTransactionModalMount (URL-driven), rather than navigating to a page. No
  // cross-feature import: the FAB only mutates URL params.
  const handleAddTransaction = useCallback(() => {
    patchModalSearch(navigate, { modal: "add-transaction", prefillAccountId: accountId });
  }, [navigate, accountId]);

  const handleBuyOpen = useCallback((target: ModalTarget) => setBuyTarget(target), []);
  const handleBuyClose = useCallback(() => setBuyTarget(null), []);
  const handleBuySuccess = useCallback(() => {
    setBuyTarget(null);
    data.retry();
  }, [data]);

  const handleSellOpen = useCallback((target: SellTarget) => setSellTarget(target), []);
  const handleSellClose = useCallback(() => setSellTarget(null), []);
  const handleSellSuccess = useCallback(() => {
    setSellTarget(null);
    data.retry();
  }, [data]);

  // MKT-072 — open price history modal for a holding
  const handlePriceHistory = useCallback(
    (assetId: string) => {
      const holding = data.holdingDetails.find((h) => h.asset_id === assetId);
      if (holding) setHistoryTarget(holding);
    },
    [data.holdingDetails],
  );
  const handleHistoryClose = useCallback(() => setHistoryTarget(null), []);

  const handleOpenBalanceOpen = useCallback(() => setOpenBalanceOpen(true), []);
  const handleOpenBalanceClose = useCallback(() => setOpenBalanceOpen(false), []);
  const handleOpenBalanceSuccess = useCallback(() => {
    setOpenBalanceOpen(false);
    data.retry();
  }, [data]);

  const handleDepositOpen = useCallback(() => setDepositOpen(true), []);
  const handleDepositClose = useCallback(() => setDepositOpen(false), []);
  const handleDepositSuccess = useCallback(() => {
    setDepositOpen(false);
    data.retry();
  }, [data]);

  const handleWithdrawalOpen = useCallback(() => setWithdrawalOpen(true), []);
  const handleWithdrawalClose = useCallback(() => setWithdrawalOpen(false), []);
  const handleWithdrawalSuccess = useCallback(() => {
    setWithdrawalOpen(false);
    data.retry();
  }, [data]);

  // DIV-012 — dividend modal state (entered from the header "Add" menu)
  const handleDividendOpen = useCallback(() => setDividendOpen(true), []);
  const handleDividendClose = useCallback(() => setDividendOpen(false), []);
  const handleDividendSuccess = useCallback(() => {
    setDividendOpen(false);
    data.retry();
  }, [data]);

  // FSD-010/012 — free-shares modal state (entered from the header "Record" menu).
  const handleFreeSharesOpen = useCallback(() => setFreeSharesOpen(true), []);
  const handleFreeSharesClose = useCallback(() => setFreeSharesOpen(false), []);
  const handleFreeSharesSuccess = useCallback(() => {
    setFreeSharesOpen(false);
    data.retry();
  }, [data]);

  // MKT-153/156/157 — toggle the price-refresh lock on an asset. Calls the
  // block/unblock command, then re-reads the asset list (so the row's lock
  // icon flips from the store, mirroring archive/unarchive) and confirms
  // with a snackbar. Errors surface via the snackbar's i18n pipeline.
  const handleTogglePriceRefreshLock = useCallback(
    async (assetId: string, currentlyBlocked: boolean) => {
      try {
        const res = currentlyBlocked
          ? await accountDetailsGateway.unblockAssetPriceRefresh(assetId)
          : await accountDetailsGateway.blockAssetPriceRefresh(assetId);
        if (res.status === "ok") {
          await fetchAssets();
          showSnackbar(
            t(currentlyBlocked ? "mkt.lock.success_unblocked" : "mkt.lock.success_blocked"),
            "success",
          );
        } else {
          const msg = priceRefreshLockErrorToI18n(res.error);
          showSnackbar(t(msg.key, msg.vars), "error");
        }
      } catch (e) {
        logger.error("Failed to toggle price-refresh lock", { error: e, assetId });
        showSnackbar(t("error.Unknown"), "error");
      }
    },
    [fetchAssets, showSnackbar, t],
  );

  // ---------------------------------------------------------------------------
  // Derived flags
  // ---------------------------------------------------------------------------
  const hasActiveHoldings = data.holdings.length > 0;
  const hasClosedHoldings = data.summary?.hasClosedHoldings ?? false;
  // DIV-011/020 — paying-asset candidates for the dividend modal: active,
  // non-cash holdings (quantity > 0). Memoized so the stable reference does not
  // invalidate the modal's `assetOptions` memo on every parent render.
  const dividendPayingAssets = useMemo(
    () =>
      data.holdingDetails
        .filter((h) => !isCashAsset(h.asset_id) && h.quantity > 0)
        .map((h) => ({
          assetId: h.asset_id,
          assetName: h.asset_name,
          assetCurrency: h.asset_currency,
        })),
    [data.holdingDetails],
  );
  // CSH-095 — banner only fires when other holdings exist (or all-closed) and no cash row.
  const showNoCashBanner =
    data.summary !== null && !data.hasVisibleCashRow && !data.summary.isEmpty;

  return {
    // Data layer (re-exposed)
    isLoading: data.isLoading,
    error: data.error,
    retry: data.retry,
    summary: data.summary,
    holdings: data.holdings,
    holdingDetails: data.holdingDetails,
    closedHoldings: data.closedHoldings,
    hasVisibleCashRow: data.hasVisibleCashRow,
    // Derived
    accountCurrency,
    hasActiveHoldings,
    hasClosedHoldings,
    showNoCashBanner,
    dividendPayingAssets,
    // Modal targets / flags
    buyTarget,
    sellTarget,
    historyTarget,
    openBalanceOpen,
    depositOpen,
    withdrawalOpen,
    dividendOpen,
    freeSharesOpen,
    // Handlers
    handleAddTransaction,
    handleBuyOpen,
    handleBuyClose,
    handleBuySuccess,
    handleSellOpen,
    handleSellClose,
    handleSellSuccess,
    handlePriceHistory,
    handleHistoryClose,
    handleOpenBalanceOpen,
    handleOpenBalanceClose,
    handleOpenBalanceSuccess,
    handleDepositOpen,
    handleDepositClose,
    handleDepositSuccess,
    handleWithdrawalOpen,
    handleWithdrawalClose,
    handleWithdrawalSuccess,
    handleDividendOpen,
    handleDividendClose,
    handleDividendSuccess,
    handleFreeSharesOpen,
    handleFreeSharesClose,
    handleFreeSharesSuccess,
    handleTogglePriceRefreshLock,
  };
}
