import { useCallback } from "react";
import type { CreateAccountDTO, UpdateAccountDTO } from "@/bindings";
import { logger } from "@/lib/logger";
import { useAppStore } from "../../lib/store";
import { accountGateway } from "./gateway";

function extractErrorCode(error: unknown): string {
  if (typeof error === "object" && error !== null && "code" in error) {
    return String((error as { code: unknown }).code);
  }
  logger.error("[useAccounts] unexpected error shape from gateway", { error });
  return "Unknown";
}

export function useAccounts() {
  const accounts = useAppStore((state) => state.accounts);
  const loading = useAppStore((state) => state.isLoadingAccounts);
  const fetchError = useAppStore((state) => state.accountsError);
  const fetchAccounts = useAppStore((state) => state.fetchAccounts);

  const addAccount = useCallback(async (dto: CreateAccountDTO) => {
    try {
      const res = await accountGateway.addAccount(dto);
      if (res.status === "ok") {
        return { data: res.data, error: null };
      }
      return { data: null, error: `error.${extractErrorCode(res.error)}` };
    } catch (e) {
      logger.error("Failed to add account", { error: e });
      return { data: null, error: String(e) };
    }
  }, []);

  const updateAccount = useCallback(async (dto: UpdateAccountDTO) => {
    try {
      const res = await accountGateway.updateAccount(dto);
      if (res.status === "ok") {
        return { data: res.data, error: null };
      }
      return { data: null, error: `error.${extractErrorCode(res.error)}` };
    } catch (e) {
      logger.error("Failed to update account", { error: e });
      return { data: null, error: String(e) };
    }
  }, []);

  const deleteAccount = useCallback(async (id: string) => {
    try {
      const res = await accountGateway.deleteAccount(id);
      if (res.status === "ok") {
        return { error: null };
      }
      return { error: `error.${extractErrorCode(res.error)}` };
    } catch (e) {
      logger.error("Failed to delete account", { error: e });
      return { error: String(e) };
    }
  }, []);

  const getAccountDeletionSummary = useCallback(async (id: string) => {
    try {
      const res = await accountGateway.getAccountDeletionSummary(id);
      if (res.status === "ok") {
        return { data: res.data, error: null };
      }
      return { data: null, error: `error.${extractErrorCode(res.error)}` };
    } catch (e) {
      logger.error("Failed to fetch account deletion summary", { error: e });
      return { data: null, error: String(e) };
    }
  }, []);

  return {
    accounts,
    loading,
    fetchError,
    fetchAccounts,
    addAccount,
    updateAccount,
    deleteAccount,
    getAccountDeletionSummary,
  };
}
