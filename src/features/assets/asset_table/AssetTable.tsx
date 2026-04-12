import { Archive, ArchiveRestore, ArrowDown, ArrowUp, Edit2, ShoppingCart, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Asset } from "@/bindings";
import { logger } from "@/lib/logger";
import { Button } from "@/ui/components/button/Button";
import { IconButton } from "@/ui/components/button/IconButton";
import { ConfirmationDialog } from "@/ui/components/modal/Dialog";
import { AddTransactionModal } from "../../transactions/add_transaction/AddTransactionModal";
import { EditAssetModal } from "../edit_asset_modal/EditAssetModal";
import { getRiskBadgeClasses } from "../shared/presenter";
import { useAssets } from "../useAssets";
import { type SortConfig, useAssetTable } from "./useAssetTable";

interface AssetTableProps {
  searchTerm: string;
  showArchived: boolean;
}

export function AssetTable({ searchTerm, showArchived }: AssetTableProps) {
  const { t } = useTranslation();
  const { archiveAsset, unarchiveAsset, assets, loading, fetchError, fetchAssets } = useAssets();
  const [selectedAssetId, setSelectedAssetId] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  useEffect(() => {
    logger.info("[AssetTable] mounted");
  }, []);

  const { sortedAndFilteredAssets, sortConfig, handleSort } = useAssetTable(
    assets,
    searchTerm,
    showArchived,
  );

  // Archive state
  const [isArchiveDialogOpen, setIsArchiveDialogOpen] = useState(false);
  const [assetToArchive, setAssetToArchive] = useState<{ id: string; name: string } | null>(null);

  // Unarchive state
  const [isUnarchiveDialogOpen, setIsUnarchiveDialogOpen] = useState(false);
  const [assetToUnarchive, setAssetToUnarchive] = useState<{ id: string; name: string } | null>(
    null,
  );

  // Edit state
  const [isEditModalOpen, setIsEditModalOpen] = useState(false);
  const [assetToEdit, setAssetToEdit] = useState<Asset | null>(null);

  // Buy transaction state (TRX-010)
  const [isBuyModalOpen, setIsBuyModalOpen] = useState(false);
  const [buyPrefillAssetId, setBuyPrefillAssetId] = useState<string | undefined>(undefined);

  const SortIcon = ({ column }: { column: SortConfig["key"] }) => {
    if (sortConfig.key !== column) return null;
    return sortConfig.direction === "asc" ? (
      <ArrowUp size={14} className="ml-1 text-m3-primary" />
    ) : (
      <ArrowDown size={14} className="ml-1 text-m3-primary" />
    );
  };

  const handleArchiveConfirm = async () => {
    if (!assetToArchive) return;
    const result = await archiveAsset(assetToArchive.id);
    setIsArchiveDialogOpen(false);
    setAssetToArchive(null);
    if (result.error) setActionError(result.error);
  };

  const handleUnarchiveConfirm = async () => {
    if (!assetToUnarchive) return;
    const result = await unarchiveAsset(assetToUnarchive.id);
    setIsUnarchiveDialogOpen(false);
    setAssetToUnarchive(null);
    if (result.error) setActionError(result.error);
  };

  const isSearching = searchTerm.trim().length > 0;

  return (
    <div className="m3-table-container flex-1">
      {actionError && (
        <div className="mb-3 flex items-center justify-between gap-2 text-sm text-m3-error px-2">
          <span>{actionError}</span>
          <IconButton
            icon={<X size={14} />}
            size="sm"
            aria-label={t("action.close")}
            onClick={() => setActionError(null)}
          />
        </div>
      )}
      <table className="w-full border-collapse">
        <thead className="sticky top-0 bg-m3-surface-container z-10">
          <tr>
            <th className="m3-th cursor-pointer" onClick={() => handleSort("name")}>
              <div className="flex items-center">
                {t("asset.column_name")} <SortIcon column="name" />
              </div>
            </th>
            <th className="m3-th cursor-pointer" onClick={() => handleSort("reference")}>
              <div className="flex items-center">
                {t("asset.column_reference")} <SortIcon column="reference" />
              </div>
            </th>
            <th className="m3-th cursor-pointer" onClick={() => handleSort("class")}>
              <div className="flex items-center">
                {t("asset.column_class")} <SortIcon column="class" />
              </div>
            </th>
            <th className="m3-th cursor-pointer" onClick={() => handleSort("category")}>
              <div className="flex items-center">
                {t("asset.column_category")} <SortIcon column="category" />
              </div>
            </th>
            <th className="m3-th text-center cursor-pointer" onClick={() => handleSort("currency")}>
              <div className="flex items-center justify-center">
                {t("asset.column_currency")} <SortIcon column="currency" />
              </div>
            </th>
            <th
              className="m3-th text-center cursor-pointer"
              onClick={() => handleSort("risk_level")}
            >
              <div className="flex items-center justify-center">
                {t("asset.column_risk")} <SortIcon column="risk_level" />
              </div>
            </th>
            <th className="m3-th">{t("asset.column_status")}</th>
            <th className="m3-th text-right">{t("asset.column_actions")}</th>
          </tr>
        </thead>
        <tbody>
          {loading ? (
            <tr>
              <td colSpan={8} className="m3-td text-center py-12">
                <span className="text-m3-on-surface-variant animate-pulse">
                  {t("asset.loading")}
                </span>
              </td>
            </tr>
          ) : fetchError ? (
            <tr>
              <td colSpan={8} className="m3-td text-center py-12">
                <div className="flex flex-col items-center gap-3">
                  <span className="text-m3-error text-sm">{t("asset.error_load")}</span>
                  <Button variant="outline" size="sm" onClick={fetchAssets}>
                    {t("action.retry")}
                  </Button>
                </div>
              </td>
            </tr>
          ) : sortedAndFilteredAssets.length === 0 ? (
            <tr>
              <td colSpan={8} className="m3-td text-center py-12 text-m3-on-surface-variant italic">
                {isSearching ? t("asset.no_search_results") : t("asset.empty")}
              </td>
            </tr>
          ) : (
            sortedAndFilteredAssets.map((asset) => (
              <tr
                key={asset.id}
                tabIndex={0}
                onClick={() => setSelectedAssetId(asset.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    setSelectedAssetId(asset.id);
                  }
                }}
                className={`m3-tr ${selectedAssetId === asset.id ? "m3-tr-selected" : ""} ${
                  asset.is_archived ? "opacity-50" : ""
                }`}
              >
                <td className="m3-td font-medium text-m3-on-surface">{asset.name}</td>
                <td className="m3-td font-mono text-m3-on-surface-variant">{asset.reference}</td>
                <td className="m3-td">
                  <span className="m3-chip-outline">{asset.class}</span>
                </td>
                <td className="m3-td text-m3-on-surface-variant">{asset.category.name}</td>
                <td className="m3-td text-center text-m3-on-surface font-bold text-xs">
                  {asset.currency}
                </td>
                <td className="m3-td text-center">
                  <span
                    className={`inline-flex items-center justify-center w-7 h-7 rounded-full text-[11px] font-bold ${getRiskBadgeClasses(asset.risk_level)}`}
                  >
                    {asset.risk_level}
                  </span>
                </td>
                <td className="m3-td">
                  {asset.is_archived && (
                    <span className="m3-chip-outline text-xs text-m3-on-surface-variant">
                      {t("asset.badge_archived")}
                    </span>
                  )}
                </td>
                <td className="m3-td text-right">
                  <div className="flex items-center justify-end gap-1">
                    {/* TRX-010 — Buy action entry point */}
                    <IconButton
                      icon={<ShoppingCart size={16} />}
                      size="sm"
                      aria-label={t("transaction.action_buy")}
                      onClick={(e) => {
                        e.stopPropagation();
                        setBuyPrefillAssetId(asset.id);
                        setIsBuyModalOpen(true);
                      }}
                    />
                    <IconButton
                      icon={<Edit2 size={16} />}
                      size="sm"
                      disabled={asset.is_archived}
                      aria-label={t("asset.action_edit")}
                      onClick={(e) => {
                        e.stopPropagation();
                        setAssetToEdit(asset);
                        setIsEditModalOpen(true);
                      }}
                    />
                    {asset.is_archived ? (
                      <IconButton
                        icon={<ArchiveRestore size={16} />}
                        size="sm"
                        aria-label={t("asset.action_unarchive")}
                        onClick={(e) => {
                          e.stopPropagation();
                          setAssetToUnarchive({ id: asset.id, name: asset.name });
                          setIsUnarchiveDialogOpen(true);
                        }}
                      />
                    ) : (
                      <IconButton
                        icon={<Archive size={16} />}
                        size="sm"
                        aria-label={t("asset.action_archive")}
                        onClick={(e) => {
                          e.stopPropagation();
                          setAssetToArchive({ id: asset.id, name: asset.name });
                          setIsArchiveDialogOpen(true);
                        }}
                      />
                    )}
                  </div>
                </td>
              </tr>
            ))
          )}
        </tbody>
      </table>

      {/* Edit Asset Modal */}
      <EditAssetModal
        isOpen={isEditModalOpen}
        onClose={() => {
          setIsEditModalOpen(false);
          setAssetToEdit(null);
        }}
        asset={assetToEdit}
      />

      {/* Buy Transaction Modal — TRX-010, TRX-011 */}
      <AddTransactionModal
        isOpen={isBuyModalOpen}
        onClose={() => {
          setIsBuyModalOpen(false);
          setBuyPrefillAssetId(undefined);
        }}
        prefillAssetId={buyPrefillAssetId}
      />

      {/* Archive Confirmation Dialog — R13 */}
      <ConfirmationDialog
        isOpen={isArchiveDialogOpen}
        onCancel={() => {
          setIsArchiveDialogOpen(false);
          setAssetToArchive(null);
        }}
        onConfirm={handleArchiveConfirm}
        title={t("asset.archive_confirm_title")}
        message={t("asset.archive_confirm_message")}
        confirmLabel={t("asset.action_archive")}
        cancelLabel={t("action.cancel")}
        variant="default"
      />

      {/* Unarchive Confirmation Dialog — R20 */}
      <ConfirmationDialog
        isOpen={isUnarchiveDialogOpen}
        onCancel={() => {
          setIsUnarchiveDialogOpen(false);
          setAssetToUnarchive(null);
        }}
        onConfirm={handleUnarchiveConfirm}
        title={t("asset.unarchive_confirm_title")}
        message={t("asset.unarchive_confirm_message")}
        confirmLabel={t("asset.action_unarchive")}
        cancelLabel={t("action.cancel")}
        variant="default"
      />
    </div>
  );
}
