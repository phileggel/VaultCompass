import { useCallback, useMemo, useState } from "react";
import type { Account } from "@/bindings";
import { FREQUENCY_ORDER } from "../shared/presenter";

export type SortConfig = {
  key: "name" | "update_frequency";
  direction: "asc" | "desc";
};

export function useAccountTable(
  accounts: Account[],
  searchTerm: string,
  deleteAccount: (id: string) => Promise<{ error: string | null }>,
) {
  const [sortConfig, setSortConfig] = useState<SortConfig>({ key: "name", direction: "asc" });
  const [deleteData, setDeleteData] = useState<{ id: string; name: string } | null>(null);
  const [editData, setEditData] = useState<Account | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  const handleSort = (key: SortConfig["key"]) => {
    setSortConfig((prev) => ({
      key,
      direction: prev.key === key && prev.direction === "asc" ? "desc" : "asc",
    }));
  };

  const handleDeleteConfirm = useCallback(async () => {
    if (!deleteData) return;
    const result = await deleteAccount(deleteData.id);
    if (result.error) {
      // R13 — keep dialog open, show inline error
      setActionError(result.error);
    } else {
      setDeleteData(null);
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
    isEmpty,
    hasNoSearchResults,
    deleteData,
    setDeleteData,
    editData,
    setEditData,
    actionError,
    setActionError,
    handleDeleteConfirm,
  };
}
