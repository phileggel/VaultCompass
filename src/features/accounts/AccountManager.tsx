import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { AccountDetailsView } from "@/features/account_details";
import { logger } from "@/lib/logger";
import { useAppStore } from "@/lib/store";
import { FAB } from "@/ui/components/fab/FAB";
import { ManagerLayout } from "@/ui/components/layout/ManagerLayout";
import { AccountTable } from "./account_table/AccountTable";
import { AddAccountModal } from "./add_account/AddAccountModal";

export function AccountManager() {
  const { t } = useTranslation();
  const accountCount = useAppStore((state) => state.accounts.length);
  const [query, setQuery] = useState("");
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  // ACD-011 — selected account drives navigation to the details view
  const [selectedAccountId, setSelectedAccountId] = useState<string | null>(null);

  useEffect(() => {
    logger.info("[AccountManager] mounted");
  }, []);

  // ACD-011 — render details view when an account is selected
  if (selectedAccountId) {
    return (
      <AccountDetailsView accountId={selectedAccountId} onBack={() => setSelectedAccountId(null)} />
    );
  }

  return (
    <>
      <ManagerLayout
        searchId="account-search"
        title={t("account.title")}
        count={accountCount}
        searchTerm={query}
        onSearchChange={setQuery}
        searchPlaceholder={t("account.search_placeholder")}
        table={<AccountTable searchTerm={query} onAccountClick={setSelectedAccountId} />}
      />
      {/* R14 — FAB opens add modal */}
      <FAB onClick={() => setIsAddModalOpen(true)} label={t("account.fab_label")} />
      <AddAccountModal isOpen={isAddModalOpen} onClose={() => setIsAddModalOpen(false)} />
    </>
  );
}
