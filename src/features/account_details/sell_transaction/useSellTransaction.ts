import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { TransactionFormData } from "@/features/transactions/shared/types";
import { validateSellForm } from "@/features/transactions/shared/validateTransaction";
import { useTransactions } from "@/features/transactions/useTransactions";
import { getAutoRecordPrice } from "@/lib/autoRecordPriceStorage";
import { logger } from "@/lib/logger";
import {
  computeSellTotalMicro,
  decimalToMicro,
  microToDecimal,
  microToFormatted,
} from "@/lib/microUnits";
import { useSnackbar } from "@/lib/snackbarStore";
import { accountDetailsGateway } from "../gateway";

interface UseSellTransactionProps {
  accountId: string;
  assetId: string;
  /** Holding quantity in micro-units — used for oversell guard (SEL-022). */
  holdingQuantityMicro: number;
  onSubmitSuccess?: () => void;
}

const today = () => new Date().toISOString().slice(0, 10);

export function useSellTransaction({
  accountId,
  assetId,
  holdingQuantityMicro,
  onSubmitSuccess,
}: UseSellTransactionProps) {
  const { t } = useTranslation();
  const showSnackbar = useSnackbar();
  const { sellHolding } = useTransactions();

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
  // MKT-052/053 — snapshot of the global auto-record toggle at hook mount
  const [recordPrice, setRecordPrice] = useState<boolean>(() => getAutoRecordPrice());

  const microValues = useMemo(() => {
    const qtyMicro = decimalToMicro(formData.quantity);
    const priceMicro = decimalToMicro(formData.unitPrice);
    const rateMicro = decimalToMicro(formData.exchangeRate);
    const feesMicro = decimalToMicro(formData.fees);
    const totalMicro = computeSellTotalMicro(qtyMicro, priceMicro, rateMicro, feesMicro);
    return { qtyMicro, priceMicro, rateMicro, feesMicro, totalMicro };
  }, [formData.quantity, formData.unitPrice, formData.exchangeRate, formData.fees]);

  const isFormValid = useMemo(
    () =>
      validateSellForm(
        formData,
        microValues.qtyMicro,
        microValues.totalMicro,
        holdingQuantityMicro,
      ) === null,
    [formData, microValues.qtyMicro, microValues.totalMicro, holdingQuantityMicro],
  );

  const handleChange = useCallback((field: keyof TransactionFormData, value: string) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
  }, []);

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();

      const validationError = validateSellForm(
        formData,
        microValues.qtyMicro,
        microValues.totalMicro,
        holdingQuantityMicro,
      );
      if (validationError) {
        setError(
          t(validationError, {
            defaultValue: validationError,
            max: microToFormatted(holdingQuantityMicro, 6),
          }),
        );
        return;
      }

      setError(null);
      setIsSubmitting(true);

      try {
        const result = await sellHolding({
          account_id: formData.accountId,
          asset_id: formData.assetId,
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

        // MKT-055/061 — record price separately when auto-record is on and price is non-zero (best-effort)
        if (recordPrice && microValues.priceMicro > 0) {
          accountDetailsGateway
            .recordAssetPrice(
              formData.assetId,
              formData.date,
              parseFloat(microToDecimal(microValues.priceMicro)),
            )
            .catch((e) =>
              logger.warn("Failed to record asset price after sell", {
                error: e,
              }),
            );
        }

        showSnackbar(t("transaction.success_sell_created"), "success");
        onSubmitSuccess?.();
      } finally {
        setIsSubmitting(false);
      }
    },
    [
      formData,
      microValues,
      holdingQuantityMicro,
      recordPrice,
      sellHolding,
      t,
      showSnackbar,
      onSubmitSuccess,
    ],
  );

  return {
    formData,
    /** Sell total proceeds in micro-units formatted for display (SEL-023, read-only). */
    totalAmountDisplay: microToFormatted(microValues.totalMicro),
    /** Maximum sellable quantity formatted for display (SEL-022). */
    maxQuantityDisplay: microToFormatted(holdingQuantityMicro, 6),
    error,
    isSubmitting,
    isFormValid,
    recordPrice,
    setRecordPrice,
    handleChange,
    handleSubmit,
  };
}
