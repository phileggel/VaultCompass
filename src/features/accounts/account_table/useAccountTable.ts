import type { KeyboardEvent, MouseEvent } from "react";
import { useCallback, useMemo, useState } from "react";
import type { Account, AccountDeletionSummary } from "@/bindings";
import { FREQUENCY_ORDER } from "../shared/presenter";

export type SortConfig = {
  key: "name" | "update_frequency";
  direction: "asc" | "desc";
};

export function useAccountTable(
  accounts: Account[],
  searchTerm: string,
  deleteAccount: (id: string) => Promise<{ error: string | null }>,
  getAccountDeletionSummary: (
    id: string,
  ) => Promise<{ data: AccountDeletionSummary | null; error: string | null }>,
  onAccountClick: (accountId: string) => void,
) {
  const [sortConfig, setSortConfig] = useState<SortConfig>({ key: "name", direction: "asc" });
  const [deleteData, setDeleteData] = useState<{ id: string; name: string } | null>(null);
  const [deleteSummary, setDeleteSummary] = useState<AccountDeletionSummary | null>(null);
  const [fetchingSummaryFor, setFetchingSummaryFor] = useState<string | null>(null);
  const [editData, setEditData] = useState<Account | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  const handleSort = useCallback((key: SortConfig["key"]) => {
    setSortConfig((prev) => ({
      key,
      direction: prev.key === key && prev.direction === "asc" ? "desc" : "asc",
    }));
  }, []);

  const handleNameKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        handleSort("name");
      }
    },
    [handleSort],
  );

  const handleFrequencyKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        handleSort("update_frequency");
      }
    },
    [handleSort],
  );

  const handleRowKeyDown = useCallback(
    (e: KeyboardEvent, accountId: string) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onAccountClick(accountId);
      }
    },
    [onAccountClick],
  );

  const handleEditClick = useCallback((e: MouseEvent, account: Account) => {
    e.stopPropagation();
    setEditData(account);
  }, []);

  const handleEditClose = useCallback(() => setEditData(null), []);

  const handleDeleteClick = useCallback(
    async (e: MouseEvent, id: string, name: string) => {
      e.stopPropagation();
      setFetchingSummaryFor(id);
      setActionError(null);
      const result = await getAccountDeletionSummary(id);
      setFetchingSummaryFor(null);
      if (result.error) {
        setActionError(result.error);
        return;
      }
      if (!result.data) {
        setActionError("error.Unknown");
        return;
      }
      setDeleteSummary(result.data);
      setDeleteData({ id, name });
    },
    [getAccountDeletionSummary],
  );

  const handleDeleteCancel = useCallback(() => {
    setDeleteData(null);
    setDeleteSummary(null);
  }, []);

  const handleDeleteConfirm = useCallback(async () => {
    if (!deleteData) return;
    const result = await deleteAccount(deleteData.id);
    if (result.error) {
      // R13 — keep dialog open, show inline error
      setActionError(result.error);
    } else {
      setDeleteData(null);
      setDeleteSummary(null);
    }
  }, [deleteData, deleteAccount]);

  const sortedAndFilteredAccounts = useMemo(() => {
    const filtered = accounts.filter((a) =>
      a.name.toLowerCase().includes(searchTerm.toLowerCase()),
    );

    return [...filtered].sort((a, b) => {
      let cmp: number;
      if (sortConfig.key === "update_frequency") {
        // R9 — sort by logical enum order, not alphabetical label
        cmp = FREQUENCY_ORDER[a.update_frequency] - FREQUENCY_ORDER[b.update_frequency];
      } else {
        cmp = a.name.toLowerCase().localeCompare(b.name.toLowerCase());
      }
      return sortConfig.direction === "asc" ? cmp : -cmp;
    });
  }, [accounts, searchTerm, sortConfig]);

  // R11 — no accounts exist and no search is active
  const isEmpty = accounts.length === 0 && searchTerm.trim().length === 0;

  // R10 — search is active but no results match
  const hasNoSearchResults = searchTerm.trim().length > 0 && sortedAndFilteredAccounts.length === 0;

  return {
    sortedAndFilteredAccounts,
    sortConfig,
    handleSort,
    handleNameKeyDown,
    handleFrequencyKeyDown,
    handleRowKeyDown,
    handleEditClick,
    handleEditClose,
    handleDeleteClick,
    handleDeleteCancel,
    isEmpty,
    hasNoSearchResults,
    deleteData,
    deleteSummary,
    fetchingSummaryFor,
    editData,
    actionError,
    setActionError,
    handleDeleteConfirm,
  };
}
