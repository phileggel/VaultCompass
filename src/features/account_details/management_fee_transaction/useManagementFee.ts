import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { getLastOperationDate, setLastOperationDate } from "@/lib/lastOperationDateStorage";
import { logger } from "@/lib/logger";
import { decimalToMicro } from "@/lib/microUnits";
import { useSnackbar } from "@/ui/components/snackbar/snackbarStore";
import type { I18nMessage } from "@/ui/format/i18n";
import { accountDetailsGateway } from "../gateway";
import { managementFeeErrorToI18n } from "../shared/presenter";
import { validateDate } from "../shared/validateCashForm";
import { validatePercentage } from "../shared/validateFeeForm";

/** A holding a one-off management fee can be charged against — active, non-cash (FEE-011/012). */
export interface ManagementFeeHolding {
  assetId: string;
  assetName: string;
  assetCurrency: string;
}

interface UseManagementFeeProps {
  accountId: string;
  onSubmitSuccess?: () => void;
}

interface ManagementFeeFormData {
  assetId: string;
  date: string;
  percent: string;
  note: string;
}

export function useManagementFee({ accountId, onSubmitSuccess }: UseManagementFeeProps) {
  const { t } = useTranslation();
  const showSnackbar = useSnackbar();

  const [formData, setFormData] = useState<ManagementFeeFormData>(() => ({
    assetId: "",
    date: getLastOperationDate(accountId),
    percent: "",
    note: "",
  }));
  const [error, setError] = useState<I18nMessage | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  // FEE-021 — asset selected, percentage in (0, 100], date valid.
  const isFormValid = useMemo(
    () =>
      formData.assetId !== "" &&
      validatePercentage(formData.percent) === null &&
      validateDate(formData.date) === null,
    [formData.assetId, formData.percent, formData.date],
  );

  const handleChange = useCallback((field: keyof ManagementFeeFormData, value: string) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
  }, []);

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      const percentErr = validatePercentage(formData.percent);
      const dateErr = validateDate(formData.date);
      const validationError =
        formData.assetId === "" ? { key: "error.AssetNotHeld" } : (percentErr ?? dateErr);
      if (validationError) {
        setError(validationError);
        return;
      }

      setError(null);
      setIsSubmitting(true);
      try {
        // 1% = 1_000_000 micro-percent — the same micro scaling as decimals (FEE-021).
        const percentMicros = decimalToMicro(formData.percent);
        const note = formData.note || null;

        const result = await accountDetailsGateway.recordManagementFee({
          account_id: accountId,
          asset_id: formData.assetId,
          date: formData.date,
          percent_micros: percentMicros,
          note,
        });

        if (result.status === "error") {
          logger.error("[useManagementFee] recordManagementFee failed", { error: result.error });
          setError(managementFeeErrorToI18n(result.error));
          return;
        }
        setLastOperationDate(accountId, formData.date);
        showSnackbar(t("management_fee.recorded"), "success");
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
    handleChange,
    handleSubmit,
  };
}
