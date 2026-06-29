import { useNavigate } from "@tanstack/react-router";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { HoldingDetail } from "@/bindings";
import { logger } from "@/lib/logger";
import { patchModalSearch } from "@/lib/modalSearch";
import { useAppStore } from "@/lib/store";
import { useSnackbar } from "@/ui/components/snackbar/snackbarStore";
import { accountDetailsGateway } from "../gateway";
import { formatIsoDate } from "../shared/formatDate";
import { isCashAsset, priceRefreshLockErrorToI18n, toPriceableAssets } from "../shared/presenter";
import type { ModalTarget, SellTarget } from "../shared/types";
import { useAccountDetails } from "./useAccountDetails";

/** Local calendar date as ISO `YYYY-MM-DD` — the as-of selector's "today" default. */
function todayIso(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

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
  // As-of valuation date: "" = live view (today); a non-empty ISO date loads a
  // read-only reconstruction. `isAsOf` is true only for a non-today date — picking
  // today (or clearing) keeps the live, mutable view.
  const [asOfDate, setAsOfDate] = useState("");
  const isAsOf = asOfDate !== "" && asOfDate !== todayIso();
  const data = useAccountDetails(accountId, isAsOf ? asOfDate : "");
  const accounts = useAppStore((state) => state.accounts);
  const fetchAssets = useAppStore((state) => state.fetchAssets);
  const showSnackbar = useSnackbar();
  const { t, i18n } = useTranslation();
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
  // cross-feature import: the FAB only mutates URL params. No-op in as-of (read-only).
  const handleAddTransaction = useCallback(() => {
    if (isAsOf) return;
    patchModalSearch(navigate, { modal: "add-transaction", prefillAccountId: accountId });
  }, [navigate, accountId, isAsOf]);

  const handleBuyOpen = useCallback(
    (target: ModalTarget) => {
      if (isAsOf) return;
      setBuyTarget(target);
    },
    [isAsOf],
  );
  const handleBuyClose = useCallback(() => setBuyTarget(null), []);
  const handleBuySuccess = useCallback(() => {
    setBuyTarget(null);
    data.retry();
  }, [data]);

  const handleSellOpen = useCallback(
    (target: SellTarget) => {
      if (isAsOf) return;
      setSellTarget(target);
    },
    [isAsOf],
  );
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

  const handleOpenBalanceOpen = useCallback(() => {
    if (isAsOf) return;
    setOpenBalanceOpen(true);
  }, [isAsOf]);
  const handleOpenBalanceClose = useCallback(() => setOpenBalanceOpen(false), []);
  const handleOpenBalanceSuccess = useCallback(() => {
    setOpenBalanceOpen(false);
    data.retry();
  }, [data]);

  const handleDepositOpen = useCallback(() => {
    if (isAsOf) return;
    setDepositOpen(true);
  }, [isAsOf]);
  const handleDepositClose = useCallback(() => setDepositOpen(false), []);
  const handleDepositSuccess = useCallback(() => {
    setDepositOpen(false);
    data.retry();
  }, [data]);

  const handleWithdrawalOpen = useCallback(() => {
    if (isAsOf) return;
    setWithdrawalOpen(true);
  }, [isAsOf]);
  const handleWithdrawalClose = useCallback(() => setWithdrawalOpen(false), []);
  const handleWithdrawalSuccess = useCallback(() => {
    setWithdrawalOpen(false);
    data.retry();
  }, [data]);

  // DIV-012 — dividend modal state (entered from the header "Add" menu)
  const handleDividendOpen = useCallback(() => {
    if (isAsOf) return;
    setDividendOpen(true);
  }, [isAsOf]);
  const handleDividendClose = useCallback(() => setDividendOpen(false), []);
  const handleDividendSuccess = useCallback(() => {
    setDividendOpen(false);
    data.retry();
  }, [data]);
  // DIV-010 — "add another" refreshes the data but keeps the modal open.
  const handleDividendRecorded = useCallback(() => {
    data.retry();
  }, [data]);

  // FSD-010/012 — free-shares modal state (entered from the header "Record" menu).
  const handleFreeSharesOpen = useCallback(() => {
    if (isAsOf) return;
    setFreeSharesOpen(true);
  }, [isAsOf]);
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
      if (isAsOf) return;
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
    [fetchAssets, showSnackbar, t, isAsOf],
  );

  // ---------------------------------------------------------------------------
  // Derived flags
  // ---------------------------------------------------------------------------
  // CSH-098 — the asset-positions empty state excludes the always-present Cash row.
  const hasNonCashActiveHoldings = data.holdings.some((row) => !row.isCash);
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
  // MKT-011 — priceable holdings for the price modal's asset combobox. Memoized
  // so the stable reference does not thrash the combobox on every parent render
  // (e.g. an AssetPriceUpdated event while the modal is open).
  const priceableAssets = useMemo(() => toPriceableAssets(data.holdings), [data.holdings]);
  return {
    // Data layer (re-exposed)
    isLoading: data.isLoading,
    error: data.error,
    retry: data.retry,
    summary: data.summary,
    holdings: data.holdings,
    holdingDetails: data.holdingDetails,
    closedHoldings: data.closedHoldings,
    // As-of (read-only past-date valuation)
    asOfDate,
    // As-of date formatted in the user's locale, for the read-only banner (F5).
    asOfDateFormatted: formatIsoDate(asOfDate, i18n.language),
    // Date shown in the selector: the chosen date, or today when none is chosen.
    asOfDisplayDate: asOfDate || todayIso(),
    setAsOfDate,
    isAsOf,
    // Derived
    accountCurrency,
    hasNonCashActiveHoldings,
    hasClosedHoldings,
    dividendPayingAssets,
    priceableAssets,
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
    handleDividendRecorded,
    handleFreeSharesOpen,
    handleFreeSharesClose,
    handleFreeSharesSuccess,
    handleTogglePriceRefreshLock,
  };
}
