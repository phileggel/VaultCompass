import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { logger } from "@/lib/logger";
import { Button } from "@/ui/components/button/Button";
import { ComboboxField } from "@/ui/components/field/ComboboxField";
import { DateField } from "@/ui/components/field/DateField";
import { TextareaField } from "@/ui/components/field/TextareaField";
import { TextField } from "@/ui/components/field/TextField";
import { FormModal } from "@/ui/components/modal/FormModal";
import type { DividendPayingAsset } from "./useDividendTransaction";
import { useDividendTransaction } from "./useDividendTransaction";

interface DividendTransactionModalProps {
  isOpen: boolean;
  onClose: () => void;
  accountId: string;
  accountCurrency: string;
  /** Active non-cash holdings the dividend can be attributed to (DIV-011/020). */
  heldAssets: DividendPayingAsset[];
  onSubmitSuccess: () => void;
  /** Refresh-only callback for "Record & add another" — keeps the modal open (DIV-010). */
  onRecorded: () => void;
}

export function DividendTransactionModal({
  isOpen,
  onClose,
  accountId,
  accountCurrency,
  heldAssets,
  onSubmitSuccess,
  onRecorded,
}: DividendTransactionModalProps) {
  const { t } = useTranslation();

  useEffect(() => {
    logger.info("[DividendTransactionModal] mounted");
  }, []);

  const {
    formData,
    error,
    isSubmitting,
    isFormValid,
    showExchangeRate,
    handleChange,
    handleSubmit,
    handleAddAnother,
  } = useDividendTransaction({
    accountId,
    accountCurrency,
    heldAssets,
    onSubmitSuccess,
    onRecorded,
  });

  const selectedCurrency =
    heldAssets.find((a) => a.assetId === formData.assetId)?.assetCurrency ?? accountCurrency;

  const footer = useMemo(
    () => (
      <div className="flex items-center justify-end gap-2">
        <Button variant="secondary" onClick={onClose} disabled={isSubmitting}>
          {t("action.cancel")}
        </Button>
        <Button
          id="dividend-trx-add-another"
          type="button"
          variant="secondary"
          onClick={handleAddAnother}
          loading={isSubmitting}
          disabled={isSubmitting || !isFormValid}
        >
          {t("dividend.action_record_and_add_another")}
        </Button>
        <Button
          id="dividend-trx-record"
          type="submit"
          form="dividend-transaction-form"
          variant="primary"
          loading={isSubmitting}
          disabled={isSubmitting || !isFormValid}
        >
          {t("dividend.action_record")}
        </Button>
      </div>
    ),
    [isSubmitting, isFormValid, t, onClose, handleAddAnother],
  );

  return (
    <FormModal
      isOpen={isOpen}
      onClose={onClose}
      title={t("dividend.modal_title")}
      footer={footer}
      maxWidth="max-w-2xl"
    >
      <form id="dividend-transaction-form" onSubmit={handleSubmit} className="flex flex-col gap-4">
        {/* DIV-020 — paying asset chosen inside the modal via fuzzy-search combobox */}
        <ComboboxField
          id="dividend-trx-asset"
          label={t("dividend.form_asset_label")}
          items={heldAssets}
          displayKey="assetName"
          idKey="assetId"
          value={formData.assetId}
          onChange={(id) => handleChange("assetId", id)}
          searchKeys={["assetName"]}
          placeholder={t("dividend.form_select_asset")}
        />

        <DateField
          id="dividend-trx-date"
          data-testid="dividend-trx-date"
          label={t("transaction.form_date_label")}
          value={formData.date}
          onChange={(e) => handleChange("date", e.target.value)}
          required
        />

        {/* DIV-021 — net amount in the paying asset's native currency */}
        <TextField
          id="dividend-trx-amount"
          data-testid="dividend-trx-amount"
          label={`${t("dividend.form_amount_label")} (${selectedCurrency})`}
          type="number"
          min="0"
          step="any"
          value={formData.amount}
          onChange={(e) => handleChange("amount", e.target.value)}
          placeholder={t("dividend.form_amount_placeholder")}
          required
        />

        {/* DIV-022 — exchange rate only when asset currency differs from account currency */}
        {showExchangeRate && (
          <TextField
            id="dividend-trx-exchange-rate"
            data-testid="dividend-trx-exchange-rate"
            label={t("transaction.form_exchange_rate_label")}
            type="number"
            min="0"
            step="any"
            value={formData.exchangeRate}
            onChange={(e) => handleChange("exchangeRate", e.target.value)}
            placeholder={t("transaction.form_exchange_rate_placeholder")}
          />
        )}

        <TextareaField
          id="dividend-trx-note"
          data-testid="dividend-trx-note"
          label={t("transaction.form_note_label")}
          rows={2}
          value={formData.note}
          onChange={(e) => handleChange("note", e.target.value)}
          placeholder={t("transaction.form_note_placeholder")}
        />

        {error && (
          <p role="alert" className="text-sm text-m3-error">
            {t(error.key, error.vars)}
          </p>
        )}
      </form>
    </FormModal>
  );
}
