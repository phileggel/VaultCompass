import { useParams } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { Transaction } from "@/bindings";
import { accountMutationErrorToI18n } from "@/features/accounts/shared/presenter";
import { logger } from "@/lib/logger";
import { decimalToMicro } from "@/lib/microUnits";
import { useAppStore } from "@/lib/store";
import type { I18nMessage } from "@/ui/format/i18n";
import { transactionGateway } from "../gateway";
import { type TransactionRowViewModel, toTransactionRow } from "../shared/presenter";

const UNKNOWN_ERROR: I18nMessage = { key: "error.Unknown" };

/** All filters are AND-combined; empty values mean "no constraint". */
interface JournalFilters {
  assetId: string;
  type: string;
  amountMin: string;
  amountMax: string;
}

const EMPTY_FILTERS: JournalFilters = { assetId: "", type: "", amountMin: "", amountMax: "" };

export function useAccountJournal() {
  const { accountId } = useParams({ from: "/accounts/$accountId/journal" });
  const assets = useAppStore((s) => s.assets);
  const accounts = useAppStore((s) => s.accounts);

  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<I18nMessage | null>(null);
  // Chronological, latest first (the journal default).
  const [sortDirection, setSortDirection] = useState<"asc" | "desc">("desc");
  const [filters, setFilters] = useState<JournalFilters>(EMPTY_FILTERS);

  const fetchTransactions = useCallback(async (): Promise<void> => {
    setIsLoading(true);
    setError(null);
    try {
      const res = await transactionGateway.getAllTransactionsForAccount(accountId);
      if (res.status === "ok") {
        setTransactions(res.data);
      } else {
        setError(accountMutationErrorToI18n(res.error));
        setTransactions([]);
      }
    } catch (e) {
      logger.error("Failed to fetch account journal", { error: e });
      setError(UNKNOWN_ERROR);
      setTransactions([]);
    } finally {
      setIsLoading(false);
    }
  }, [accountId]);

  useEffect(() => {
    fetchTransactions();
  }, [fetchTransactions]);

  // Re-fetch on TransactionUpdated so an edit/delete reflects without navigating away.
  useEffect(() => {
    const unlistenPromise = transactionGateway.subscribeToEvents((type) => {
      if (type === "TransactionUpdated") {
        fetchTransactions();
      }
    });
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [fetchTransactions]);

  const setFilter = useCallback((field: keyof JournalFilters, value: string) => {
    setFilters((prev) => ({ ...prev, [field]: value }));
  }, []);

  const clearFilters = useCallback(() => setFilters(EMPTY_FILTERS), []);

  const toggleSortDirection = useCallback(() => {
    setSortDirection((d) => (d === "asc" ? "desc" : "asc"));
  }, []);

  const transactionById = useMemo(() => {
    const map = new Map<string, Transaction>();
    for (const tx of transactions) map.set(tx.id, tx);
    return map;
  }, [transactions]);

  // Asset options come from the transactions actually present (TXL-013 style).
  const assetFilterOptions = useMemo(() => {
    const ids = [...new Set(transactions.map((tx) => tx.asset_id))];
    return ids.map((id) => ({ value: id, label: assets.find((a) => a.id === id)?.name ?? id }));
  }, [transactions, assets]);

  const typeFilterOptions = useMemo(() => {
    return [...new Set(transactions.map((tx) => tx.transaction_type))].map((type) => ({
      value: type,
      label: type,
    }));
  }, [transactions]);

  const filteredSortedRows = useMemo<TransactionRowViewModel[]>(() => {
    const hasMin = filters.amountMin.trim() !== "";
    const hasMax = filters.amountMax.trim() !== "";
    const minMicro = hasMin ? decimalToMicro(filters.amountMin) : 0;
    const maxMicro = hasMax ? decimalToMicro(filters.amountMax) : 0;

    const filtered = transactions.filter((tx) => {
      if (filters.assetId && tx.asset_id !== filters.assetId) return false;
      if (filters.type && tx.transaction_type !== filters.type) return false;
      if (hasMin && tx.total_amount < minMicro) return false;
      if (hasMax && tx.total_amount > maxMicro) return false;
      return true;
    });

    const rows = filtered.map((tx) => {
      const asset = assets.find((a) => a.id === tx.asset_id);
      const account = accounts.find((a) => a.id === tx.account_id);
      return toTransactionRow(tx, asset?.name ?? tx.asset_id, account?.name ?? tx.account_id);
    });

    return rows.sort((a, b) => {
      const cmp = a.date.localeCompare(b.date);
      return sortDirection === "asc" ? cmp : -cmp;
    });
  }, [transactions, filters, assets, accounts, sortDirection]);

  return {
    accountId,
    isLoading,
    error,
    sortDirection,
    filters,
    setFilter,
    clearFilters,
    toggleSortDirection,
    assetFilterOptions,
    typeFilterOptions,
    filteredSortedRows,
    transactionById,
    hasTransactions: transactions.length > 0,
    refresh: fetchTransactions,
  };
}
