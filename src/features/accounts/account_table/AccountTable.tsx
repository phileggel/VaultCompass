import { ArrowDown, ArrowUp, Calendar, Edit2, Trash2 } from "lucide-react";
import { useState } from "react";
import type { Account, UpdateFrequency } from "@/bindings";
import { ConfirmationDialog } from "@/ui/components/modal/Dialog";
import { EditAccountModal } from "../edit_account_modal/EditAccountModal";
import { FREQUENCY_LABELS } from "../shared/constants";
import { useAccounts } from "../useAccounts";
import { type SortConfig, useAccountTable } from "./useAccount";

interface AccountTableProps {
  searchTerm: string;
}

export function AccountTable({ searchTerm }: AccountTableProps) {
  const { accounts, loading, deleteAccount } = useAccounts();
  const [selectedAccountId, setSelectedAccountId] = useState<string | null>(null);

  const { sortedAndFilteredAccounts, sortConfig, handleSort } = useAccountTable(
    accounts,
    searchTerm,
  );

  // Modals
  const [deleteData, setDeleteData] = useState<{ id: string; name: string } | null>(null);
  const [editData, setEditData] = useState<Account | null>(null);

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
            <th className="m3-th" onClick={() => handleSort("name")}>
              <div className="flex items-center">
                Name <SortIcon column="name" />
              </div>
            </th>
            <th className="m3-th" onClick={() => handleSort("update_frequency")}>
              <div className="flex items-center">
                Update Frequency <SortIcon column="update_frequency" />
              </div>
            </th>
            <th className="m3-th text-right">Actions</th>
          </tr>
        </thead>
        <tbody>
          {loading ? (
            <tr>
              <td colSpan={3} className="m3-td text-center py-12">
                <span className="text-m3-on-surface-variant animate-pulse">
                  Loading accounts...
                </span>
              </td>
            </tr>
          ) : sortedAndFilteredAccounts.length === 0 ? (
            <tr>
              <td colSpan={3} className="m3-td text-center py-12 text-m3-on-surface-variant">
                No accounts found.
              </td>
            </tr>
          ) : (
            sortedAndFilteredAccounts.map((account: Account) => (
              <tr
                key={account.id}
                onClick={() => setSelectedAccountId(account.id)}
                className={`m3-tr ${selectedAccountId === account.id ? "m3-tr-selected" : ""}`}
              >
                <td className="m3-td font-medium text-m3-on-surface">{account.name}</td>
                <td className="m3-td">
                  <div className="flex items-center gap-2 text-m3-on-surface-variant">
                    <Calendar size={14} className="text-m3-primary" />
                    <span className="m3-chip-outline">
                      {FREQUENCY_LABELS[account.update_frequency as UpdateFrequency] ||
                        account.update_frequency}
                    </span>
                  </div>
                </td>
                <td className="m3-td text-right">
                  <div className="flex items-center justify-end gap-1">
                    <button
                      type="button"
                      className="m3-icon-button-primary"
                      onClick={(e) => {
                        e.stopPropagation();
                        setEditData(account);
                      }}
                    >
                      <Edit2 size={16} />
                    </button>
                    <button
                      type="button"
                      className="m3-icon-button-error"
                      onClick={(e) => {
                        e.stopPropagation();
                        setDeleteData({ id: account.id, name: account.name });
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

      {/* Edit Account Modal */}
      <EditAccountModal isOpen={!!editData} onClose={() => setEditData(null)} account={editData} />

      {/* Delete Confirmation Dialog */}
      <ConfirmationDialog
        isOpen={!!deleteData}
        onCancel={() => setDeleteData(null)}
        onConfirm={async () => {
          if (deleteData) {
            await deleteAccount(deleteData.id);
          }
        }}
        title="Delete Account"
        message={`Are you sure you want to delete ${deleteData?.name}? All asset links to this account will be removed.`}
        confirmLabel="Delete"
        cancelLabel="Cancel"
        variant="danger"
      />
    </div>
  );
}
