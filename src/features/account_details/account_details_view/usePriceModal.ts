import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import type { HoldingDetail } from "@/bindings";
import { logger } from "@/lib/logger";
import { useSnackbar } from "@/lib/snackbarStore";
import { accountDetailsGateway } from "../gateway";

export interface UsePriceModalProps {
  holding: HoldingDetail;
  onSubmitSuccess?: () => void;
}

export interface UsePriceModalResult {
  date: string;
  price: string;
  error: string | null;
  isSubmitting: boolean;
  isFormValid: boolean;
  handleChange: (field: "date" | "price", value: string) => void;
  handleSubmit: (e: React.FormEvent) => Promise<void>;
}

const today = () => new Date().toISOString().slice(0, 10);

function validatePrice(price: string): string | null {
  const n = parseFloat(price);
  if (Number.isNaN(n) || n <= 0) return "price_modal.error_price_not_positive";
  return null;
}

function validateDate(date: string): string | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return "price_modal.error_invalid_date";
  if (date > today()) return "price_modal.error_future_date";
  return null;
}

function initialPrice(holding: HoldingDetail): string {
  const t = today();
  if (holding.current_price_date === t && holding.current_price !== null) {
    return (holding.current_price / 1_000_000).toFixed(2);
  }
  return "";
}

export function usePriceModal({
  holding,
  onSubmitSuccess,
}: UsePriceModalProps): UsePriceModalResult {
  const { t } = useTranslation();
  const showSnackbar = useSnackbar();
  const [date, setDate] = useState(today);
  const [price, setPrice] = useState(() => initialPrice(holding));
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Inline validation — only applied to non-empty values (MKT-021, MKT-022)
  const priceValidationError = price.length > 0 ? validatePrice(price) : null;
  const dateValidationError = date.length > 0 ? validateDate(date) : null;

  const isFormValid =
    date.length > 0 &&
    price.length > 0 &&
    priceValidationError === null &&
    dateValidationError === null;

  // Display order: submit error then inline validation errors
  const error = submitError ?? priceValidationError ?? dateValidationError;

  const handleChange = useCallback((field: "date" | "price", value: string) => {
    setSubmitError(null);
    if (field === "date") setDate(value);
    else setPrice(value);
  }, []);

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      if (!isFormValid) return;
      setIsSubmitting(true);
      const result = await accountDetailsGateway.recordAssetPrice(
        holding.asset_id,
        date,
        parseFloat(price),
      );
      setIsSubmitting(false);
      if (result.status === "ok") {
        showSnackbar(t("price_modal.success"));
        onSubmitSuccess?.();
      } else {
        logger.error("[usePriceModal] recordAssetPrice failed", result.error);
        setSubmitError(result.error);
      }
    },
    [isFormValid, holding.asset_id, date, price, showSnackbar, t, onSubmitSuccess],
  );

  return { date, price, error, isSubmitting, isFormValid, handleChange, handleSubmit };
}
