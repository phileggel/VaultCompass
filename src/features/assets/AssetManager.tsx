import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { logger } from "@/lib/logger";
import { FAB } from "@/ui/components/fab/FAB";
import { ManagerLayout } from "@/ui/components/layout/ManagerLayout";
import { AddAssetModal } from "./add_asset/AddAsset";
import { AssetTable } from "./asset_table/AssetTable";
import { useAssets } from "./useAssets";

export function AssetManager() {
  const { t } = useTranslation();
  const { activeCount } = useAssets();

  const [query, setQuery] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);

  useEffect(() => {
    logger.info("[AssetManager] mounted");
  }, []);

  const tableWithToggle = (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-end px-4 py-2 bg-m3-surface-container-low">
        <label className="flex items-center gap-2 text-sm text-m3-on-surface-variant cursor-pointer select-none">
          <input
            type="checkbox"
            checked={showArchived}
            onChange={(e) => setShowArchived(e.target.checked)}
            className="accent-m3-primary"
          />
          {t("asset.toggle_show_archived")}
        </label>
      </div>
      <div className="flex-1 overflow-auto">
        <AssetTable searchTerm={query} showArchived={showArchived} />
      </div>
    </div>
  );

  return (
    <>
      <ManagerLayout
        searchId="asset-search"
        title={t("asset.title")}
        count={activeCount}
        searchTerm={query}
        onSearchChange={setQuery}
        searchPlaceholder={t("asset.search_placeholder")}
        table={tableWithToggle}
      />
      <FAB onClick={() => setIsAddModalOpen(true)} label={t("asset.fab_label")} />
      <AddAssetModal isOpen={isAddModalOpen} onClose={() => setIsAddModalOpen(false)} />
    </>
  );
}
