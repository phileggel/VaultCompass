import { useNavigate } from "@tanstack/react-router";
import { useCallback, useState } from "react";
import type { HoldingDetail } from "@/bindings";
import { useAppStore } from "@/lib/store";
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
  const accountCurrency = accounts.find((a) => a.id === accountId)?.currency ?? "";

  // ---------------------------------------------------------------------------
  // Modal targets / open flags
  // ---------------------------------------------------------------------------
  const [buyTarget, setBuyTarget] = useState<ModalTarget | null>(null);
  const [sellTarget, setSellTarget] = useState<SellTarget | null>(null);
  const [priceTarget, setPriceTarget] = useState<HoldingDetail | null>(null);
  const [historyTarget, setHistoryTarget] = useState<HoldingDetail | null>(null);
  const [openBalanceOpen, setOpenBalanceOpen] = useState(false);
  const [depositOpen, setDepositOpen] = useState(false);
  const [withdrawalOpen, setWithdrawalOpen] = useState(false);

  // ---------------------------------------------------------------------------
  // Handlers
  // ---------------------------------------------------------------------------
  const handleAddTransaction = useCallback(() => {
    navigate({
      to: "/transactions/new",
      search: { prefillAccountId: accountId, prefillAssetId: undefined },
    });
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

  // MKT-010/013 — find the raw HoldingDetail (no extra fetch) for the price modal
  const handleEnterPrice = useCallback(
    (assetId: string) => {
      const holding = data.holdingDetails.find((h) => h.asset_id === assetId);
      if (holding) setPriceTarget(holding);
    },
    [data.holdingDetails],
  );
  const handlePriceClose = useCallback(() => setPriceTarget(null), []);
  // MKT-028 — close modal on success; re-fetch happens via AssetPriceUpdated event (MKT-036)
  const handlePriceSuccess = useCallback(() => setPriceTarget(null), []);

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

  // ---------------------------------------------------------------------------
  // Derived flags
  // ---------------------------------------------------------------------------
  const hasActiveHoldings = data.holdings.length > 0;
  const hasClosedHoldings = data.summary?.hasClosedHoldings ?? false;
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
    // Modal targets / flags
    buyTarget,
    sellTarget,
    priceTarget,
    historyTarget,
    openBalanceOpen,
    depositOpen,
    withdrawalOpen,
    // Handlers
    handleAddTransaction,
    handleBuyOpen,
    handleBuyClose,
    handleBuySuccess,
    handleSellOpen,
    handleSellClose,
    handleSellSuccess,
    handleEnterPrice,
    handlePriceClose,
    handlePriceSuccess,
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
  };
}
