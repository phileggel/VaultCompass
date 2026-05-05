import { useCallback, useState } from "react";
import type { AssetPrice } from "@/bindings";
import { logger } from "@/lib/logger";
import { microToDecimal } from "@/lib/microUnits";
import { accountDetailsGateway } from "../gateway";
import { isDateValid, isPriceValid } from "../shared/validatePriceForm";

interface UseEditPriceProps {
  assetId: string;
  target: AssetPrice;
  onSuccess: () => void;
}

export interface UseEditPriceResult {
  date: string;
  price: string;
  setDate: (v: string) => void;
  setPrice: (v: string) => void;
  isValid: boolean;
  isSubmitting: boolean;
  error: string | null;
  handleSubmit: () => Promise<void>;
}

export function useEditPrice({
  assetId,
  target,
  onSuccess,
}: UseEditPriceProps): UseEditPriceResult {
  const [date, setDate] = useState(target.date);
  const [price, setPrice] = useState(() => microToDecimal(target.price, 6));
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isValid = isPriceValid(price) && isDateValid(date);

  const handleSubmit = useCallback(async () => {
    if (!isValid) return;
    setIsSubmitting(true);
    try {
      const result = await accountDetailsGateway.updateAssetPrice(
        assetId,
        target.date,
        date,
        parseFloat(price),
      );
      if (result.status === "ok") {
        setError(null);
        onSuccess();
      } else {
        logger.error("[useEditPrice] updateAssetPrice failed", result.error);
        setError(result.error.code);
      }
    } finally {
      setIsSubmitting(false);
    }
  }, [isValid, assetId, target.date, date, price, onSuccess]);

  return {
    date,
    price,
    setDate,
    setPrice,
    isValid,
    isSubmitting,
    error,
    handleSubmit,
  };
}
