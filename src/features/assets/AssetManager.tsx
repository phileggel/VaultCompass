import { useNavigate, useSearch } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { logger } from "@/lib/logger";
import { FAB } from "@/ui/components/fab/FAB";
import { ManagerLayout } from "@/ui/components/layout/ManagerLayout";
import { AddAssetModal } from "./add_asset/AddAsset";
import { AssetTable } from "./asset_table/AssetTable";
import { useAssets } from "./useAssets";

type ReturnNavTarget =
  | { type: "txList"; accountId: string; assetId: string }
  | { type: "account"; accountId: string }
  | { type: "assets" };

function resolveReturnNav(returnPath: string | undefined): ReturnNavTarget {
  if (!returnPath) return { type: "assets" };
  const txMatch = returnPath.match(/^\/accounts\/([^/]+)\/transactions\/([^/]+)$/);
  // biome-ignore lint/style/noNonNullAssertion: groups are present when regex matches
  if (txMatch) return { type: "txList", accountId: txMatch[1]!, assetId: txMatch[2]! };
  const accMatch = returnPath.match(/^\/accounts\/([^/]+)$/);
  // biome-ignore lint/style/noNonNullAssertion: group is present when regex matches
  if (accMatch) return { type: "account", accountId: accMatch[1]! };
  return { type: "assets" };
}

export function AssetManager() {
  const { t } = useTranslation();
  const { activeCount } = useAssets();
  const navigate = useNavigate();
  const { createNew, returnPath, pendingTransactionAssetId } = useSearch({ from: "/assets" });

  const [query, setQuery] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  const [isAddModalOpen, setIsAddModalOpen] = useState(!!createNew);

  useEffect(() => {
    logger.info("[AssetManager] mounted");
  }, []);

  const handleAddAssetSuccess = useCallback(
    (newAssetId: string) => {
      setIsAddModalOpen(false);
      const target = resolveReturnNav(returnPath);
      const search = { pendingTransactionAssetId: newAssetId };
      if (target.type === "txList") {
        navigate({
          to: "/accounts/$accountId/transactions/$assetId",
          params: { accountId: target.accountId, assetId: target.assetId },
          search,
        });
      } else if (target.type === "account") {
        navigate({ to: "/accounts/$accountId", params: { accountId: target.accountId }, search });
      } else {
        navigate({
          to: "/assets",
          search: { ...search, createNew: undefined, returnPath: undefined },
        });
      }
    },
    [navigate, returnPath],
  );

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
        <AssetTable
          searchTerm={query}
          showArchived={showArchived}
          pendingBuyAssetId={pendingTransactionAssetId}
        />
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
      <AddAssetModal
        isOpen={isAddModalOpen}
        prefillName={createNew}
        onSuccess={handleAddAssetSuccess}
        onClose={() => setIsAddModalOpen(false)}
      />
    </>
  );
}
