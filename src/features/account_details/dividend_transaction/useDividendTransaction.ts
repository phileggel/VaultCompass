import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { logger } from "@/lib/logger";
import { decimalToMicro } from "@/lib/microUnits";
import { useSnackbar } from "@/ui/components/snackbar/snackbarStore";
import type { I18nMessage } from "@/ui/format/i18n";
import { accountDetailsGateway } from "../gateway";
import { dividendErrorToI18n } from "../shared/presenter";
import { validateAmount, validateDate } from "../shared/validateCashForm";

/** A holding the dividend can be attributed to — active, non-cash (DIV-011/020). */
export interface DividendPayingAsset {
  assetId: string;
  assetName: string;
  assetCurrency: string;
}

interface UseDividendTransactionProps {
  accountId: string;
  accountCurrency: string;
  heldAssets: DividendPayingAsset[];
  onSubmitSuccess?: () => void;
}

interface DividendFormData {
  assetId: string;
  date: string;
  amount: string;
  exchangeRate: string;
  note: string;
}

const today = () => new Date().toISOString().slice(0, 10);

export function useDividendTransaction({
  accountId,
  accountCurrency,
  heldAssets,
  onSubmitSuccess,
}: UseDividendTransactionProps) {
  const { t } = useTranslation();
  const showSnackbar = useSnackbar();

  const [formData, setFormData] = useState<DividendFormData>(() => ({
    assetId: "",
    date: today(),
    amount: "",
    exchangeRate: "1.000000",
    note: "",
  }));
  const [error, setError] = useState<I18nMessage | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const selectedAsset = useMemo(
    () => heldAssets.find((a) => a.assetId === formData.assetId) ?? null,
    [heldAssets, formData.assetId],
  );

  // DIV-022 — exchange rate only relevant when the paying asset's currency
  // differs from the account currency.
  const showExchangeRate = useMemo(
    () => selectedAsset !== null && selectedAsset.assetCurrency !== accountCurrency,
    [selectedAsset, accountCurrency],
  );

  // DIV-021 — asset selected, amount strictly positive, date valid.
  const isFormValid = useMemo(
    () =>
      formData.assetId !== "" &&
      validateAmount(formData.amount) === null &&
      validateDate(formData.date) === null,
    [formData.assetId, formData.amount, formData.date],
  );

  const handleChange = useCallback((field: keyof DividendFormData, value: string) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
  }, []);

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      const amountErr = validateAmount(formData.amount);
      const dateErr = validateDate(formData.date);
      const validationError =
        formData.assetId === "" ? { key: "error.AssetNotHeld" } : (amountErr ?? dateErr);
      if (validationError) {
        setError(validationError);
        return;
      }

      setError(null);
      setIsSubmitting(true);
      try {
        const result = await accountDetailsGateway.recordDividend({
          account_id: accountId,
          asset_id: formData.assetId,
          date: formData.date,
          amount_micros: decimalToMicro(formData.amount),
          exchange_rate: decimalToMicro(formData.exchangeRate),
          note: formData.note || null,
        });
        if (result.status === "error") {
          logger.error("[useDividendTransaction] recordDividend failed", { error: result.error });
          setError(dividendErrorToI18n(result.error));
          return;
        }
        showSnackbar(t("dividend.recorded"), "success");
        onSubmitSuccess?.();
      } finally {
        setIsSubmitting(false);
      }
    },
    [accountId, formData, t, showSnackbar, onSubmitSuccess],
  );

  return {
    formData,
    error,
    isSubmitting,
    isFormValid,
    showExchangeRate,
    handleChange,
    handleSubmit,
  };
}
