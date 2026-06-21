import type { Transaction } from "@/bindings";
import { patchModalSearch } from "@/lib/modalSearch";

/**
 * Routes a transaction edit to the correct surface (TRX-036). Cash Deposit/Withdrawal
 * and FreeShares corrections open their dedicated modals via the URL-driven modal mount
 * (the generic modal is cash-excluded, CSH-018 / FSD-040); everything else opens the
 * generic edit modal through `openGenericModal`. Shared by the per-asset and account-wide
 * journals so the branching lives in one place.
 */
export function routeEditTransaction(
  navigate: Parameters<typeof patchModalSearch>[0],
  raw: Transaction,
  openGenericModal: (transaction: Transaction) => void,
): void {
  switch (raw.transaction_type) {
    case "Deposit":
      patchModalSearch(navigate, {
        modal: "edit-cash-deposit",
        editTxId: raw.id,
        editTxAccountId: raw.account_id,
        editTxAssetId: raw.asset_id,
      });
      break;
    case "Withdrawal":
      patchModalSearch(navigate, {
        modal: "edit-cash-withdrawal",
        editTxId: raw.id,
        editTxAccountId: raw.account_id,
        editTxAssetId: raw.asset_id,
      });
      break;
    case "FreeShares":
      patchModalSearch(navigate, {
        modal: "edit-free-shares",
        editTxId: raw.id,
        editTxAccountId: raw.account_id,
        editTxAssetId: raw.asset_id,
      });
      break;
    default:
      openGenericModal(raw);
  }
}
