import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { TransactionFormData } from "@/features/transactions/shared/types";
import { validateTransactionForm } from "@/features/transactions/shared/validateTransaction";
import { useTransactions } from "@/features/transactions/useTransactions";
import { computeTotalMicro, decimalToMicro, microToDecimal } from "@/lib/microUnits";
import { useSnackbar } from "@/lib/snackbarStore";
import { useAppStore } from "@/lib/store";

interface UseBuyTransactionProps {
  accountId: string;
  assetId: string;
  onSubmitSuccess?: () => void;
}

const today = () => new Date().toISOString().slice(0, 10);

export function useBuyTransaction({ accountId, assetId, onSubmitSuccess }: UseBuyTransactionProps) {
  const { t } = useTranslation();
  const showSnackbar = useSnackbar();
  const { addTransaction } = useTransactions();
  const assets = useAppStore((state) => state.assets);

  const [formData, setFormData] = useState<TransactionFormData>(() => ({
    accountId,
    assetId,
    date: today(),
    quantity: "",
    unitPrice: "",
    exchangeRate: "1.000000",
    fees: "0",
    note: "",
  }));
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [showArchivedConfirm, setShowArchivedConfirm] = useState(false);

  const microValues = useMemo(() => {
    const qtyMicro = decimalToMicro(formData.quantity);
    const priceMicro = decimalToMicro(formData.unitPrice);
    const rateMicro = decimalToMicro(formData.exchangeRate);
    const feesMicro = decimalToMicro(formData.fees);
    const totalMicro = computeTotalMicro(qtyMicro, priceMicro, rateMicro, feesMicro);
    return { qtyMicro, priceMicro, rateMicro, feesMicro, totalMicro };
  }, [formData.quantity, formData.unitPrice, formData.exchangeRate, formData.fees]);

  const isFormValid = useMemo(
    () => validateTransactionForm(formData, microValues.qtyMicro, microValues.totalMicro) === null,
    [formData, microValues.qtyMicro, microValues.totalMicro],
  );

  // TRX-029 — is the pre-determined asset archived?
  const isAssetArchived = useMemo(
    () => assets.find((a) => a.id === assetId)?.is_archived ?? false,
    [assets, assetId],
  );

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

    try {
      const result = await addTransaction({
        account_id: formData.accountId,
        asset_id: formData.assetId,
        transaction_type: "Purchase",
        date: formData.date,
        quantity: microValues.qtyMicro,
        unit_price: microValues.priceMicro,
        exchange_rate: microValues.rateMicro,
        fees: microValues.feesMicro,
        note: formData.note || null,
      });

      if (result.error) {
        setError(result.error);
        return;
      }

      showSnackbar(t("transaction.success_created"), "success");
      onSubmitSuccess?.();
    } finally {
      setIsSubmitting(false);
    }
  }, [formData, microValues, addTransaction, t, showSnackbar, onSubmitSuccess]);

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      if (isAssetArchived) {
        setShowArchivedConfirm(true);
        return;
      }
      await doSubmit();
    },
    [isAssetArchived, doSubmit],
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
