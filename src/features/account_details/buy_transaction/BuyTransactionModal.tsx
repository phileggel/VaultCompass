import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { logger } from "@/lib/logger";
import { Button } from "@/ui/components/button/Button";
import { DateField } from "@/ui/components/field/DateField";
import { TextField } from "@/ui/components/field/TextField";
import { ConfirmationDialog } from "@/ui/components/modal/Dialog";
import { FormModal } from "@/ui/components/modal/FormModal";
import { useBuyTransaction } from "./useBuyTransaction";

interface BuyTransactionModalProps {
  isOpen: boolean;
  onClose: () => void;
  accountId: string;
  accountName: string;
  assetId: string;
  assetName: string;
  assetCurrency: string;
  /** When true, asset currency differs from account currency — show exchange rate field (TRX-041). */
  showExchangeRate?: boolean;
  /** Called after a successful purchase submission. Caller must refresh data. */
  onSubmitSuccess: () => void;
}

export function BuyTransactionModal({
  isOpen,
  onClose,
  accountId,
  accountName,
  assetId,
  assetName,
  assetCurrency,
  showExchangeRate = false,
  onSubmitSuccess,
}: BuyTransactionModalProps) {
  const { t } = useTranslation();

  useEffect(() => {
    logger.info("[BuyTransactionModal] mounted");
  }, []);

  const {
    formData,
    totalAmountDisplay,
    error,
    isSubmitting,
    isFormValid,
    showArchivedConfirm,
    handleChange,
    handleSubmit,
    handleConfirmArchived,
    handleCancelArchived,
  } = useBuyTransaction({ accountId, assetId, onSubmitSuccess });

  const footer = useMemo(
    () => (
      <div className="flex items-center justify-end gap-2">
        <Button
          variant="secondary"
          onClick={onClose}
          disabled={isSubmitting || showArchivedConfirm}
        >
          {t("action.cancel")}
        </Button>
        <Button
          type="submit"
          form="buy-transaction-form"
          variant="primary"
          loading={isSubmitting}
          disabled={isSubmitting || showArchivedConfirm || !isFormValid}
        >
          {t("transaction.action_buy")}
        </Button>
      </div>
    ),
    [isSubmitting, showArchivedConfirm, isFormValid, t, onClose],
  );

  return (
    <>
      <FormModal
        isOpen={isOpen}
        onClose={onClose}
        title={t("transaction.buy_modal_title")}
        footer={footer}
        maxWidth="max-w-2xl"
      >
        <form id="buy-transaction-form" onSubmit={handleSubmit} className="flex flex-col gap-4">
          {/* Account + Asset (read-only, TRX-011) */}
          <div className="grid grid-cols-2 gap-4">
            <TextField
              id="buy-trx-account"
              label={t("transaction.form_account_label")}
              type="text"
              value={accountName}
              readOnly
              aria-readonly="true"
            />
            <TextField
              id="buy-trx-asset"
              label={t("transaction.form_asset_label")}
              type="text"
              value={assetName}
              readOnly
              aria-readonly="true"
            />
          </div>

          {/* Date */}
          <DateField
            id="buy-trx-date"
            label={t("transaction.form_date_label")}
            value={formData.date}
            onChange={(e) => handleChange("date", e.target.value)}
            required
          />

          {/* Quantity */}
          <TextField
            id="buy-trx-quantity"
            label={t("transaction.form_quantity_label")}
            type="number"
            min="0"
            step="0.000001"
            value={formData.quantity}
            onChange={(e) => handleChange("quantity", e.target.value)}
            placeholder="0.000000"
            required
          />

          {/* Unit Price */}
          <TextField
            id="buy-trx-unit-price"
            label={`${t("transaction.form_unit_price_label")} (${assetCurrency})`}
            type="number"
            min="0"
            step="0.000001"
            value={formData.unitPrice}
            onChange={(e) => handleChange("unitPrice", e.target.value)}
            placeholder="0.000"
            required
          />

          {/* Exchange Rate (TRX-041) */}
          {showExchangeRate && (
            <TextField
              id="buy-trx-exchange-rate"
              label={t("transaction.form_exchange_rate_label")}
              type="number"
              min="0"
              step="0.000001"
              value={formData.exchangeRate}
              onChange={(e) => handleChange("exchangeRate", e.target.value)}
              placeholder="1.000000"
            />
          )}

          {/* Fees + Total */}
          <div className="grid grid-cols-2 gap-4">
            <TextField
              id="buy-trx-fees"
              label={t("transaction.form_fees_label")}
              type="number"
              min="0"
              step="0.000001"
              value={formData.fees}
              onChange={(e) => handleChange("fees", e.target.value)}
              placeholder="0.000"
            />
            <TextField
              id="buy-trx-total"
              label={t("transaction.form_total_amount_label")}
              type="text"
              value={totalAmountDisplay}
              readOnly
              aria-readonly="true"
            />
          </div>

          {/* Note */}
          <div className="flex flex-col gap-1">
            <label htmlFor="buy-trx-note" className="m3-input-label">
              {t("transaction.form_note_label")}
            </label>
            <textarea
              id="buy-trx-note"
              className="m3-input w-full resize-none"
              rows={2}
              value={formData.note}
              onChange={(e) => handleChange("note", e.target.value)}
              placeholder={t("transaction.form_note_placeholder")}
            />
          </div>

          {/* Inline error */}
          {error && (
            <p role="alert" className="text-sm text-m3-error">
              {t(error, { defaultValue: error })}
            </p>
          )}
        </form>
      </FormModal>

      {/* TRX-029 — archived asset confirmation */}
      <ConfirmationDialog
        isOpen={showArchivedConfirm}
        onCancel={handleCancelArchived}
        onConfirm={handleConfirmArchived}
        title={t("transaction.archived_asset_confirm_title")}
        message={t("transaction.archived_asset_confirm_message")}
        confirmLabel={t("action.confirm")}
        cancelLabel={t("action.cancel")}
      />
    </>
  );
}
