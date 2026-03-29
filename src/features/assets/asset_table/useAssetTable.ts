import { useMemo, useState } from "react";
import type { Asset } from "@/bindings";

export type SortConfig = {
  key: "name" | "reference" | "class" | "category" | "currency" | "risk_level";
  direction: "asc" | "desc";
};

export function useAssetTable(assets: Asset[], searchTerm: string) {
  const [sortConfig, setSortConfig] = useState<SortConfig>({ key: "name", direction: "asc" });

  const handleSort = (key: SortConfig["key"]) => {
    setSortConfig((prev) => ({
      key,
      direction: prev.key === key && prev.direction === "asc" ? "desc" : "asc",
    }));
  };

  const sortedAndFilteredAssets = useMemo(() => {
    const filtered = assets.filter(
      (a) =>
        a.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        a.reference?.toLowerCase().includes(searchTerm.toLowerCase()) ||
        a.class.toLowerCase().includes(searchTerm.toLowerCase()) ||
        a.category.name.toLowerCase().includes(searchTerm.toLowerCase()),
    );

    return [...filtered].sort((a, b) => {
      let aValue: string | number = "";
      let bValue: string | number = "";

      if (sortConfig.key === "category") {
        aValue = a.category.name;
        bValue = b.category.name;
      } else {
        const val = a[sortConfig.key];
        const valB = b[sortConfig.key];
        aValue = (typeof val === "string" || typeof val === "number" ? val : "") ?? "";
        bValue = (typeof valB === "string" || typeof valB === "number" ? valB : "") ?? "";
      }

      if (aValue < bValue) return sortConfig.direction === "asc" ? -1 : 1;
      if (aValue > bValue) return sortConfig.direction === "asc" ? 1 : -1;
      return 0;
    });
  }, [assets, searchTerm, sortConfig]);

  return {
    sortedAndFilteredAssets,
    sortConfig,
    handleSort,
  };
}
