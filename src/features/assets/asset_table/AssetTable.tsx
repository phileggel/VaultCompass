import { ArrowDown, ArrowUp, Edit2, Trash2 } from "lucide-react";
import { useState } from "react";
import { ConfirmationDialog } from "@/ui/components/modal/Dialog";
import { EditAssetModal } from "../edit_asset_modal/EditAssetModal";
import { useAssets } from "../useAssets";
import { type SortConfig, useAssetTable } from "./useAssetTable";

interface AssetTableProps {
  searchTerm: string;
}

export function AssetTable({ searchTerm }: AssetTableProps) {
  const { deleteAsset, assets, loading } = useAssets();
  const [selectedAssetId, setSelectedAssetId] = useState<string | null>(null);

  const { sortedAndFilteredAssets, sortConfig, handleSort } = useAssetTable(assets, searchTerm);

  // Deletion state
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [assetToDelete, setAssetToDelete] = useState<{ id: string; name: string } | null>(null);

  // Edition state
  const [isEditModalOpen, setIsEditModalOpen] = useState(false);
  const [assetToEdit, setAssetToEdit] = useState<(typeof assets)[0] | null>(null);

  const SortIcon = ({ column }: { column: SortConfig["key"] }) => {
    if (sortConfig.key !== column) return null;
    return sortConfig.direction === "asc" ? (
      <ArrowUp size={14} className="ml-1 text-m3-primary" />
    ) : (
      <ArrowDown size={14} className="ml-1 text-m3-primary" />
    );
  };

  return (
    <div className="m3-table-container flex-1">
      <table className="w-full border-collapse">
        <thead className="sticky top-0 bg-m3-surface-container z-10">
          <tr>
            <th className="m3-th cursor-pointer" onClick={() => handleSort("name")}>
              <div className="flex items-center">
                Name <SortIcon column="name" />
              </div>
            </th>
            <th className="m3-th cursor-pointer" onClick={() => handleSort("reference")}>
              <div className="flex items-center">
                Reference <SortIcon column="reference" />
              </div>
            </th>
            <th className="m3-th cursor-pointer" onClick={() => handleSort("class")}>
              <div className="flex items-center">
                Class <SortIcon column="class" />
              </div>
            </th>
            <th className="m3-th cursor-pointer" onClick={() => handleSort("category")}>
              <div className="flex items-center">
                Category <SortIcon column="category" />
              </div>
            </th>
            <th className="m3-th text-center" onClick={() => handleSort("currency")}>
              <div className="flex items-center justify-center">
                CCY <SortIcon column="currency" />
              </div>
            </th>
            <th className="m3-th text-center" onClick={() => handleSort("risk_level")}>
              <div className="flex items-center justify-center">
                Risk <SortIcon column="risk_level" />
              </div>
            </th>
            <th className="m3-th text-right">Actions</th>
          </tr>
        </thead>
        <tbody>
          {loading ? (
            <tr>
              <td colSpan={7} className="m3-td text-center py-12">
                <span className="text-m3-on-surface-variant animate-pulse">Loading assets...</span>
              </td>
            </tr>
          ) : sortedAndFilteredAssets.length === 0 ? (
            <tr>
              <td colSpan={7} className="m3-td text-center py-12 text-m3-on-surface-variant italic">
                No assets found.
              </td>
            </tr>
          ) : (
            sortedAndFilteredAssets.map((asset) => (
              <tr
                key={asset.id}
                onClick={() => setSelectedAssetId(asset.id)}
                className={`m3-tr ${selectedAssetId === asset.id ? "m3-tr-selected" : ""}`}
              >
                <td className="m3-td font-medium text-m3-on-surface">{asset.name}</td>
                <td className="m3-td font-mono text-m3-on-surface-variant">
                  {asset.reference || "—"}
                </td>
                <td className="m3-td">
                  <span className="m3-chip-outline">{asset.class}</span>
                </td>
                <td className="m3-td text-m3-on-surface-variant">{asset.category.name}</td>
                <td className="m3-td text-center text-m3-on-surface font-bold text-xs">
                  {asset.currency}
                </td>
                <td className="m3-td text-center">
                  <span
                    className={`inline-flex items-center justify-center w-7 h-7 rounded-full text-[11px] font-bold ${
                      asset.risk_level >= 4
                        ? "bg-red-500/10 text-red-600"
                        : asset.risk_level >= 3
                          ? "bg-orange-500/10 text-orange-600"
                          : "bg-green-500/10 text-green-600"
                    }`}
                  >
                    {asset.risk_level}
                  </span>
                </td>
                <td className="m3-td text-right">
                  <div className="flex items-center justify-end gap-1">
                    <button
                      type="button"
                      className="m3-icon-button-primary"
                      onClick={(e) => {
                        e.stopPropagation();
                        setAssetToEdit(asset);
                        setIsEditModalOpen(true);
                      }}
                    >
                      <Edit2 size={16} />
                    </button>
                    <button
                      type="button"
                      className="m3-icon-button-error"
                      onClick={(e) => {
                        e.stopPropagation();
                        setAssetToDelete({ id: asset.id, name: asset.name });
                        setIsDeleteDialogOpen(true);
                      }}
                    >
                      <Trash2 size={16} />
                    </button>
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

      {/* Delete Confirmation Dialog */}
      <ConfirmationDialog
        isOpen={isDeleteDialogOpen}
        onCancel={() => {
          setIsDeleteDialogOpen(false);
          setAssetToDelete(null);
        }}
        onConfirm={() => {
          if (assetToDelete) {
            deleteAsset(assetToDelete.id);
          }
        }}
        title="Delete Asset"
        message={`Are you sure you want to delete ${assetToDelete?.name}? This action cannot be undone.`}
        confirmLabel="Delete"
        cancelLabel="Cancel"
        variant="danger"
      />
    </div>
  );
}
