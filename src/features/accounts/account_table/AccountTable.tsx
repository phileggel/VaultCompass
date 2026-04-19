import { Calendar, ChevronRight, Edit2, Trash2, X } from "lucide-react";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { logger } from "@/lib/logger";
import { Button } from "@/ui/components/button/Button";
import { IconButton } from "@/ui/components/button/IconButton";
import { ConfirmationDialog } from "@/ui/components/modal/Dialog";
import { SortIcon } from "@/ui/components/SortIcon";
import { EditAccountModal } from "../edit_account_modal/EditAccountModal";
import { FREQUENCY_I18N_KEYS } from "../shared/presenter";
import { useAccounts } from "../useAccounts";
import { useAccountTable } from "./useAccountTable";

interface AccountTableProps {
  searchTerm: string;
  /** ACD-010 — callback when the user clicks an account row (excluding action buttons). */
  onAccountClick: (accountId: string) => void;
}

export function AccountTable({ searchTerm, onAccountClick }: AccountTableProps) {
  const { t } = useTranslation();
  const { accounts, loading, fetchError, fetchAccounts, deleteAccount } = useAccounts();

  useEffect(() => {
    logger.info("[AccountTable] mounted");
  }, []);

  const {
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
  } = useAccountTable(accounts, searchTerm, deleteAccount);

  return (
    <div className="m3-table-container flex-1">
      {/* R13 — inline action error with dismiss */}
      {actionError && (
        <div
          role="alert"
          className="mb-3 flex items-center justify-between gap-2 text-sm text-m3-error px-2"
        >
          <span>{t(actionError, { defaultValue: actionError })}</span>
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
            <th
              className="m3-th cursor-pointer"
              tabIndex={0}
              scope="col"
              aria-sort={
                sortConfig.key === "name"
                  ? sortConfig.direction === "asc"
                    ? "ascending"
                    : "descending"
                  : "none"
              }
              onClick={() => handleSort("name")}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  handleSort("name");
                }
              }}
            >
              <div className="flex items-center">
                {t("account.column_name")}
                <SortIcon
                  active={sortConfig.key === "name"}
                  direction={sortConfig.key === "name" ? sortConfig.direction : null}
                />
              </div>
            </th>
            <th
              className="m3-th cursor-pointer"
              tabIndex={0}
              scope="col"
              aria-sort={
                sortConfig.key === "update_frequency"
                  ? sortConfig.direction === "asc"
                    ? "ascending"
                    : "descending"
                  : "none"
              }
              onClick={() => handleSort("update_frequency")}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  handleSort("update_frequency");
                }
              }}
            >
              <div className="flex items-center">
                {t("account.column_frequency")}
                <SortIcon
                  active={sortConfig.key === "update_frequency"}
                  direction={sortConfig.key === "update_frequency" ? sortConfig.direction : null}
                />
              </div>
            </th>
            <th className="m3-th text-right">{t("account.column_actions")}</th>
          </tr>
        </thead>
        <tbody>
          {loading ? (
            <tr>
              <td colSpan={3} className="m3-td text-center py-12">
                <span className="text-m3-on-surface-variant animate-pulse">
                  {t("account.loading")}
                </span>
              </td>
            </tr>
          ) : fetchError ? (
            // R12 — error state with retry
            <tr>
              <td colSpan={3} className="m3-td text-center py-12">
                <div className="flex flex-col items-center gap-3">
                  <span className="text-m3-error text-sm">{t("account.error_load")}</span>
                  <Button variant="outline" size="sm" onClick={fetchAccounts}>
                    {t("action.retry")}
                  </Button>
                </div>
              </td>
            </tr>
          ) : isEmpty ? (
            // R11 — empty state distinct from no-search-results
            <tr>
              <td colSpan={3} className="m3-td text-center py-12 text-m3-on-surface-variant italic">
                {t("account.empty")}
              </td>
            </tr>
          ) : hasNoSearchResults ? (
            // R10 — no search results (filter active, no match)
            <tr>
              <td colSpan={3} className="m3-td text-center py-12 text-m3-on-surface-variant italic">
                {t("account.no_search_results")}
              </td>
            </tr>
          ) : (
            sortedAndFilteredAccounts.map((account) => (
              <tr
                key={account.id}
                className="m3-tr cursor-pointer group hover:bg-m3-primary/5"
                tabIndex={0}
                aria-label={t("account.open_account", { name: account.name })}
                onClick={() => onAccountClick(account.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    onAccountClick(account.id);
                  }
                }}
              >
                <td className="m3-td">
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-m3-on-surface">{account.name}</span>
                    <ChevronRight
                      size={14}
                      className="text-m3-primary opacity-0 group-hover:opacity-100 transition-opacity"
                    />
                  </div>
                </td>
                <td className="m3-td">
                  <div className="flex items-center gap-2 text-m3-on-surface-variant">
                    <Calendar size={14} className="text-m3-primary" />
                    <span className="m3-chip-outline">
                      {t(FREQUENCY_I18N_KEYS[account.update_frequency])}
                    </span>
                  </div>
                </td>
                <td className="m3-td text-right">
                  <div className="flex items-center justify-end gap-1">
                    <IconButton
                      icon={<Edit2 size={16} />}
                      variant="ghost"
                      aria-label={t("action.edit")}
                      onClick={(e) => {
                        e.stopPropagation();
                        setEditData(account);
                      }}
                    />
                    <IconButton
                      icon={<Trash2 size={16} />}
                      variant="danger"
                      aria-label={t("action.delete")}
                      onClick={(e) => {
                        e.stopPropagation();
                        setDeleteData({ id: account.id, name: account.name });
                      }}
                    />
                  </div>
                </td>
              </tr>
            ))
          )}
        </tbody>
      </table>

      <EditAccountModal isOpen={!!editData} onClose={() => setEditData(null)} account={editData} />

      {/* R16 — standard confirmation dialog for delete */}
      <ConfirmationDialog
        isOpen={!!deleteData}
        onCancel={() => setDeleteData(null)}
        onConfirm={handleDeleteConfirm}
        title={t("account.delete_confirm_title")}
        message={t("account.delete_confirm_message", { name: deleteData?.name ?? "" })}
        confirmLabel={t("action.delete")}
        cancelLabel={t("action.cancel")}
        variant="danger"
      />
    </div>
  );
}
