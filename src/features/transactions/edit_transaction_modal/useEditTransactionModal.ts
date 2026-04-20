import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Transaction } from "@/bindings";
import { computeTotalMicro, decimalToMicro, microToDecimal } from "@/lib/microUnits";
import { useSnackbar } from "@/lib/snackbarStore";
import { useAppStore } from "@/lib/store";
import type { TransactionFormData } from "../shared/types";
import { validateTransactionForm } from "../shared/validateTransaction";
import { useTransactions } from "../useTransactions";

interface UseEditTransactionModalProps {
  transaction: Transaction;
  onSubmitSuccess?: () => void;
}

/**
 * Populates the form from an existing Transaction (micro-units → decimal strings)
 * and submits via updateTransaction (TRX-031, TRX-033).
 */
export function useEditTransactionModal({
  transaction,
  onSubmitSuccess,
}: UseEditTransactionModalProps) {
  const { t } = useTranslation();
  const showSnackbar = useSnackbar();
  const { updateTransaction } = useTransactions();
  const assets = useAppStore((state) => state.assets);

  const [formData, setFormData] = useState<TransactionFormData>(() => ({
    accountId: transaction.account_id,
    assetId: transaction.asset_id,
    date: transaction.date,
    quantity: microToDecimal(transaction.quantity),
    unitPrice: microToDecimal(transaction.unit_price),
    exchangeRate: microToDecimal(transaction.exchange_rate),
    fees: microToDecimal(transaction.fees),
    note: transaction.note ?? "",
  }));

  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [showArchivedConfirm, setShowArchivedConfirm] = useState(false);

  // Derive micro-unit values from form strings — single conversion at the input boundary (ADR-001).
  const microValues = useMemo(() => {
    const qtyMicro = decimalToMicro(formData.quantity);
    const priceMicro = decimalToMicro(formData.unitPrice);
    const rateMicro = decimalToMicro(formData.exchangeRate);
    const feesMicro = decimalToMicro(formData.fees);
    const totalMicro = computeTotalMicro(qtyMicro, priceMicro, rateMicro, feesMicro);
    return { qtyMicro, priceMicro, rateMicro, feesMicro, totalMicro };
  }, [formData.quantity, formData.unitPrice, formData.exchangeRate, formData.fees]);

  // Derived form validity
  const isFormValid = useMemo(
    () => validateTransactionForm(formData, microValues.qtyMicro, microValues.totalMicro) === null,
    [formData, microValues.qtyMicro, microValues.totalMicro],
  );

  // TRX-029 — derived flag: is the currently selected asset archived?
  const isSelectedAssetArchived = formData.assetId
    ? (assets.find((a) => a.id === formData.assetId)?.is_archived ?? false)
    : false;

  const handleChange = useCallback((field: keyof TransactionFormData, value: string) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
  }, []);

  const doSubmit = useCallback(async () => {
    const validationError = validateTransactionForm(
      formData,
      microValues.qtyMicro,
      microValues.totalMicro,
    );
    if (validationError) {
      setError(t(validationError, { defaultValue: validationError }));
      return;
    }

    setError(null);
    setIsSubmitting(true);

    const result = await updateTransaction(transaction.id, {
      account_id: formData.accountId,
      asset_id: formData.assetId,
      transaction_type: transaction.transaction_type,
      date: formData.date,
      quantity: microValues.qtyMicro,
      unit_price: microValues.priceMicro,
      exchange_rate: microValues.rateMicro,
      fees: microValues.feesMicro,
      note: formData.note || null,
    });

    setIsSubmitting(false);

    if (result.error) {
      setError(result.error);
      return;
    }

    showSnackbar(t("transaction.success_updated"), "success");
    onSubmitSuccess?.();
  }, [
    formData,
    microValues,
    updateTransaction,
    transaction.id,
    transaction.transaction_type,
    t,
    onSubmitSuccess,
    showSnackbar,
  ]);

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      if (isSelectedAssetArchived) {
        // TRX-029 — show confirmation before submitting with an archived asset
        setShowArchivedConfirm(true);
        return;
      }
      await doSubmit();
    },
    [isSelectedAssetArchived, doSubmit],
  );

  const handleConfirmArchived = useCallback(async () => {
    setShowArchivedConfirm(false);
    await doSubmit();
  }, [doSubmit]);

  const handleCancelArchived = useCallback(() => {
    setShowArchivedConfirm(false);
  }, []);

  return {
    formData,
    /** Total amount in micro-units formatted for display (read-only, derived). */
    totalAmountDisplay: microToDecimal(microValues.totalMicro),
    error,
    isSubmitting,
    isFormValid,
    showArchivedConfirm,
    handleChange,
    handleSubmit,
    handleConfirmArchived,
    handleCancelArchived,
  };
}
