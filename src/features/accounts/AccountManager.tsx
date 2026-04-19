import { useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
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
  const navigate = useNavigate();
  const handleAccountClick = useCallback(
    (id: string) =>
      navigate({
        to: "/accounts/$accountId",
        params: { accountId: id },
        search: { pendingTransactionAssetId: undefined },
      }),
    [navigate],
  );

  useEffect(() => {
    logger.info("[AccountManager] mounted");
  }, []);

  return (
    <>
      <ManagerLayout
        searchId="account-search"
        title={t("account.title")}
        count={accountCount}
        searchTerm={query}
        onSearchChange={setQuery}
        searchPlaceholder={t("account.search_placeholder")}
        table={<AccountTable searchTerm={query} onAccountClick={handleAccountClick} />}
      />
      {/* R14 — FAB opens add modal */}
      <FAB onClick={() => setIsAddModalOpen(true)} label={t("account.fab_label")} />
      <AddAccountModal isOpen={isAddModalOpen} onClose={() => setIsAddModalOpen(false)} />
    </>
  );
}
