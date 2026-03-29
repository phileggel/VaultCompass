import { Plus } from "lucide-react";
import { useState } from "react";
import { useAppStore } from "@/lib/store";
import { ManagerLayout } from "@/ui/components/layout/ManagerLayout";
import { AccountTable } from "./account_table/AccountTable";
import { AddAccount } from "./add_account/AddAccount";

export function AccountManager() {
  const accountCount = useAppStore((state) => state.accounts.length);
  const [query, setQuery] = useState("");

  return (
    <ManagerLayout
      searchId="account-search"
      title="Account"
      count={accountCount}
      searchTerm={query}
      onSearchChange={setQuery}
      searchPlaceholder="Search history..."
      table={<AccountTable searchTerm={query} />}
      sidePanelTitle="Add Account"
      sidePanelIcon={<Plus size={24} strokeWidth={2.5} />}
      sidePanelDescription="Add a new account."
      sidePanelContent={<AddAccount />}
    />
  );
}
