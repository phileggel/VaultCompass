import { useCallback } from "react";
import type { CreateAccountDTO, UpdateAccountDTO } from "../../bindings";
import { useAppStore } from "../../lib/store";
import { accountGateway } from "./gateway";

export function useAccounts() {
  const accounts = useAppStore((state) => state.accounts);
  const loading = useAppStore((state) => state.isLoadingAccounts);
  const fetchAccounts = useAppStore((state) => state.fetchAccounts);

  const addAccount = useCallback(async (dto: CreateAccountDTO) => {
    try {
      const res = await accountGateway.addAccount(dto);
      return res.status === "ok";
    } catch (e) {
      console.error("Failed to add account", e);
      return false;
    }
  }, []);

  const updateAccount = useCallback(async (dto: UpdateAccountDTO) => {
    try {
      const res = await accountGateway.updateAccount(dto);
      return res.status === "ok";
    } catch (e) {
      console.error("Failed to update account", e);
      return false;
    }
  }, []);

  const deleteAccount = useCallback(async (id: string) => {
    try {
      const res = await accountGateway.deleteAccount(id);
      return res.status === "ok";
    } catch (e) {
      console.error("Failed to delete account", e);
      return false;
    }
  }, []);

  const getAccountHoldings = useCallback(async (accountId: string) => {
    try {
      const res = await accountGateway.getAccountHoldings(accountId);
      if (res.status === "ok") return res.data;
    } catch (e) {
      console.error("Failed to get holdings", e);
    }
    return [];
  }, []);

  return {
    accounts,
    loading,
    addAccount,
    updateAccount,
    deleteAccount,
    fetchAccounts,
    getAccountHoldings,
  };
}
