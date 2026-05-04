import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { HoldingDetail } from "@/bindings";
import { logger } from "@/lib/logger";
import { Button } from "@/ui/components/button/Button";
import { DateField } from "@/ui/components/field/DateField";
import { TextField } from "@/ui/components/field/TextField";
import { FormModal } from "@/ui/components/modal/FormModal";
import { usePriceModal } from "./usePriceModal";

interface PriceModalProps {
  isOpen: boolean;
  onClose: () => void;
  holding: HoldingDetail;
  onSubmitSuccess: () => void;
}

export function PriceModal({ isOpen, onClose, holding, onSubmitSuccess }: PriceModalProps) {
  const { t } = useTranslation();
  const { date, price, error, isSubmitting, isFormValid, handleChange, handleSubmit } =
    usePriceModal({ holding, onSubmitSuccess });

  useEffect(() => {
    logger.info("[PriceModal] mounted");
  }, []);

  const footer = useMemo(
    () => (
      <div className="flex items-center justify-end gap-2">
        <Button variant="secondary" onClick={onClose} disabled={isSubmitting}>
          {t("action.cancel")}
        </Button>
        <Button
          type="submit"
          form="price-modal-form"
          variant="primary"
          loading={isSubmitting}
          disabled={isSubmitting || !isFormValid}
        >
          {t("price_modal.submit")}
        </Button>
      </div>
    ),
    [isSubmitting, isFormValid, t, onClose],
  );

  return (
    <FormModal
      isOpen={isOpen}
      onClose={onClose}
      title={t("price_modal.title")}
      footer={footer}
      maxWidth="max-w-md"
    >
      <form id="price-modal-form" onSubmit={handleSubmit} className="flex flex-col gap-4">
        {/* Asset name — read-only (MKT-011) */}
        <TextField
          id="price-modal-asset"
          label={t("price_modal.asset_label")}
          type="text"
          value={holding.asset_name}
          readOnly
          aria-readonly="true"
        />

        {/* Date — editable, pre-filled with today (MKT-011) */}
        <DateField
          id="price-modal-date"
          label={t("price_modal.date_label")}
          value={date}
          onChange={(e) => handleChange("date", e.target.value)}
          required
        />

        {/* Price with currency label (MKT-023) */}
        <TextField
          id="price-modal-price"
          label={`${t("price_modal.price_label")} (${holding.asset_currency})`}
          type="number"
          min="0.000001"
          step="0.000001"
          value={price}
          onChange={(e) => handleChange("price", e.target.value)}
          placeholder={t("price_modal.form_price_placeholder")}
          required
        />

        {/* Inline error (MKT-029) */}
        {error && (
          <p role="alert" className="text-sm text-m3-error">
            {t(error, { defaultValue: error })}
          </p>
        )}
      </form>
    </FormModal>
  );
}
