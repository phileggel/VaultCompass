import { Plus } from "lucide-react";
import { useState } from "react";
import { useAppStore } from "@/lib/store";
import { ManagerLayout } from "@/ui/components/layout/ManagerLayout";
import { AddAsset } from "./add_asset/AddAsset";
import { AssetTable } from "./asset_table/AssetTable";

export function AssetManager() {
  const assetCount = useAppStore((state) => state.assets.length);
  const [query, setQuery] = useState("");

  return (
    <ManagerLayout
      searchId="asset-search"
      title="Asset"
      count={assetCount}
      searchTerm={query}
      onSearchChange={setQuery}
      searchPlaceholder="Search assets..."
      table={<AssetTable searchTerm={query} />}
      sidePanelTitle="Add Asset"
      sidePanelIcon={<Plus size={24} strokeWidth={2.5} />}
      sidePanelDescription="Add a new asset."
      sidePanelContent={<AddAsset />}
    />
  );
}
