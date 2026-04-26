import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { logger } from "@/lib/logger";
import { Button } from "@/ui/components/button/Button";
import { DateField } from "@/ui/components/field/DateField";
import { TextareaField } from "@/ui/components/field/TextareaField";
import { TextField } from "@/ui/components/field/TextField";
import { FormModal } from "@/ui/components/modal/FormModal";
import { useSellTransaction } from "./useSellTransaction";

interface SellTransactionModalProps {
  isOpen: boolean;
  onClose: () => void;
  accountId: string;
  accountName: string;
  assetId: string;
  assetName: string;
  assetCurrency: string;
  /** Holding quantity in micro-units — used for max hint and oversell guard (SEL-022). */
  holdingQuantityMicro: number;
  /** When true, asset currency differs from account currency — show exchange rate field (SEL-036). */
  showExchangeRate?: boolean;
  /** Called after a successful sell submission (SEL-045). Required — caller must refresh data. */
  onSubmitSuccess: () => void;
}

export function SellTransactionModal({
  isOpen,
  onClose,
  accountId,
  accountName,
  assetId,
  assetName,
  assetCurrency,
  holdingQuantityMicro,
  showExchangeRate = false,
  onSubmitSuccess,
}: SellTransactionModalProps) {
  const { t } = useTranslation();

  useEffect(() => {
    logger.info("[SellTransactionModal] mounted");
  }, []);

  const {
    formData,
    totalAmountDisplay,
    maxQuantityDisplay,
    error,
    isSubmitting,
    isFormValid,
    handleChange,
    handleSubmit,
  } = useSellTransaction({ accountId, assetId, holdingQuantityMicro, onSubmitSuccess });

  const footer = useMemo(
    () => (
      <div className="flex items-center justify-end gap-2">
        <Button variant="secondary" onClick={onClose} disabled={isSubmitting}>
          {t("action.cancel")}
        </Button>
        <Button
          type="submit"
          form="sell-transaction-form"
          variant="primary"
          loading={isSubmitting}
          disabled={isSubmitting || !isFormValid}
        >
          {t("transaction.action_sell")}
        </Button>
      </div>
    ),
    [isSubmitting, isFormValid, t, onClose],
  );

  return (
    <FormModal
      isOpen={isOpen}
      onClose={onClose}
      title={t("transaction.sell_modal_title")}
      footer={footer}
      maxWidth="max-w-2xl"
    >
      <form id="sell-transaction-form" onSubmit={handleSubmit} className="flex flex-col gap-4">
        {/* Account + Asset (read-only, SEL-011) */}
        <div className="grid grid-cols-2 gap-4">
          <TextField
            id="sell-trx-account"
            label={t("transaction.form_account_label")}
            type="text"
            value={accountName}
            readOnly
            aria-readonly="true"
          />
          <TextField
            id="sell-trx-asset"
            label={t("transaction.form_asset_label")}
            type="text"
            value={assetName}
            readOnly
            aria-readonly="true"
          />
        </div>

        {/* Date */}
        <DateField
          id="sell-trx-date"
          label={t("transaction.form_date_label")}
          value={formData.date}
          onChange={(e) => handleChange("date", e.target.value)}
          required
        />

        {/* Quantity with max hint (SEL-022) */}
        <div className="flex flex-col gap-1">
          <TextField
            id="sell-trx-quantity"
            label={t("transaction.form_quantity_label")}
            type="number"
            min="0"
            step="0.000001"
            value={formData.quantity}
            onChange={(e) => handleChange("quantity", e.target.value)}
            placeholder="0.000000"
            required
          />
          <span className="text-xs text-m3-on-surface-variant">
            {t("transaction.form_max_quantity_hint", { max: maxQuantityDisplay })}
          </span>
        </div>

        {/* Unit Price */}
        <TextField
          id="sell-trx-unit-price"
          label={`${t("transaction.form_unit_price_label")} (${assetCurrency})`}
          type="number"
          min="0"
          step="0.000001"
          value={formData.unitPrice}
          onChange={(e) => handleChange("unitPrice", e.target.value)}
          placeholder="0.000"
          required
        />

        {/* Exchange Rate (SEL-036) */}
        {showExchangeRate && (
          <TextField
            id="sell-trx-exchange-rate"
            label={t("transaction.form_exchange_rate_label")}
            type="number"
            min="0"
            step="0.000001"
            value={formData.exchangeRate}
            onChange={(e) => handleChange("exchangeRate", e.target.value)}
            placeholder="1.000000"
          />
        )}

        {/* Fees + Total Proceeds */}
        <div className="grid grid-cols-2 gap-4">
          <TextField
            id="sell-trx-fees"
            label={t("transaction.form_fees_label")}
            type="number"
            min="0"
            step="0.000001"
            value={formData.fees}
            onChange={(e) => handleChange("fees", e.target.value)}
            placeholder="0.000"
          />
          <TextField
            id="sell-trx-total"
            label={t("transaction.form_total_amount_label")}
            type="text"
            value={totalAmountDisplay}
            readOnly
            aria-readonly="true"
          />
        </div>

        {/* Note */}
        <TextareaField
          id="sell-trx-note"
          label={t("transaction.form_note_label")}
          rows={2}
          value={formData.note}
          onChange={(e) => handleChange("note", e.target.value)}
          placeholder={t("transaction.form_note_placeholder")}
        />

        {/* Inline error */}
        {error && (
          <p role="alert" className="text-sm text-m3-error">
            {t(error, { defaultValue: error })}
          </p>
        )}
      </form>
    </FormModal>
  );
}
