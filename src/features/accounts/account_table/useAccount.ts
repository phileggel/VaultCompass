import { useMemo, useState } from "react";
import type { Account, UpdateFrequency } from "@/bindings";
import { FREQUENCY_LABELS } from "../shared/constants";

export type SortConfig = {
  key: "name" | "update_frequency";
  direction: "asc" | "desc";
};

export function useAccountTable(accounts: Account[], searchTerm: string) {
  const [sortConfig, setSortConfig] = useState<SortConfig>({ key: "name", direction: "asc" });

  const handleSort = (key: SortConfig["key"]) => {
    setSortConfig((prev) => ({
      key,
      direction: prev.key === key && prev.direction === "asc" ? "desc" : "asc",
    }));
  };

  const sortedAndFilteredAccounts = useMemo(() => {
    const filtered = accounts.filter(
      (a) =>
        a.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        a.update_frequency.toLowerCase().includes(searchTerm.toLowerCase()),
    );

    return [...filtered].sort((a, b) => {
      const aRaw = a[sortConfig.key];
      const bRaw = b[sortConfig.key];

      let aValue: string;
      let bValue: string;
      if (sortConfig.key === "update_frequency") {
        aValue = FREQUENCY_LABELS[a.update_frequency as UpdateFrequency] || aRaw;
        bValue = FREQUENCY_LABELS[b.update_frequency as UpdateFrequency] || bRaw;
      } else {
        aValue = aRaw;
        bValue = bRaw;
      }

      const aLower = aValue.toLowerCase();
      const bLower = bValue.toLowerCase();

      if (aLower < bLower) return sortConfig.direction === "asc" ? -1 : 1;
      if (aLower > bLower) return sortConfig.direction === "asc" ? 1 : -1;
      return 0;
    });
  }, [accounts, searchTerm, sortConfig]);

  return {
    sortedAndFilteredAccounts,
    sortConfig,
    handleSort,
  };
}
