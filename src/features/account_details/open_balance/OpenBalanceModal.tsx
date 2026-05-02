import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { logger } from "@/lib/logger";
import { useAppStore } from "@/lib/store";
import { Button } from "@/ui/components/button/Button";
import { ComboboxField } from "@/ui/components/field/ComboboxField";
import { DateField } from "@/ui/components/field/DateField";
import { TextField } from "@/ui/components/field/TextField";
import { FormModal } from "@/ui/components/modal/FormModal";
import { useOpenBalance } from "./useOpenBalance";

interface OpenBalanceModalProps {
  isOpen: boolean;
  onClose: () => void;
  accountId: string;
  accountName: string;
  /** Pre-selected asset ID. When empty, a combobox is shown so the user can pick an asset (TRX-055). */
  assetId: string;
  /** Display name for the pre-selected asset. Only used when assetId is non-empty. */
  assetName: string;
  onSubmitSuccess: () => void;
}

export function OpenBalanceModal({
  isOpen,
  onClose,
  accountId,
  accountName,
  assetId,
  assetName,
  onSubmitSuccess,
}: OpenBalanceModalProps) {
  const { t } = useTranslation();
  const assets = useAppStore((state) => state.assets);

  useEffect(() => {
    logger.info("[OpenBalanceModal] mounted");
  }, []);

  const { formData, error, isSubmitting, isFormValid, handleChange, handleSubmit } = useOpenBalance(
    { accountId, assetId, onSubmitSuccess },
  );

  return (
    <FormModal
      isOpen={isOpen}
      onClose={onClose}
      title={t("open_balance.modal_title")}
      maxWidth="max-w-md"
    >
      <form id="ob-form" onSubmit={handleSubmit} className="flex flex-col gap-4">
        {/* Account (read-only, TRX-011) */}
        <TextField
          id="ob-account"
          label={t("transaction.form_account_label")}
          type="text"
          value={accountName}
          readOnly
          aria-readonly="true"
        />

        {/* Asset: read-only when pre-selected, combobox when opened from page header (TRX-055) */}
        {assetId ? (
          <TextField
            id="ob-asset-display"
            label={t("transaction.form_asset_label")}
            type="text"
            value={assetName}
            readOnly
            aria-readonly="true"
          />
        ) : (
          <ComboboxField
            id="ob-asset-select"
            label={t("transaction.form_asset_label")}
            items={assets.filter((a) => !a.is_archived)}
            displayKey="name"
            idKey="id"
            value={formData.assetId}
            onChange={(id) => handleChange("assetId", id)}
            searchKeys={["name", "reference"]}
            placeholder={t("transaction.form_asset_placeholder")}
          />
        )}

        {/* Date — max=today enforces TRX-046 */}
        <DateField
          id="ob-date"
          label={t("transaction.form_date_label")}
          value={formData.date}
          onChange={(e) => handleChange("date", e.target.value)}
          max={new Date().toISOString().slice(0, 10)}
          required
        />

        {/* Quantity */}
        <TextField
          id="ob-quantity"
          label={t("transaction.form_quantity_label")}
          type="number"
          min="0"
          step="0.000001"
          value={formData.quantity}
          onChange={(e) => handleChange("quantity", e.target.value)}
          placeholder="0.000000"
          required
        />

        {/* Total Cost (TRX-043: no fees, no exchange_rate, no unit_price) */}
        <TextField
          id="ob-total-cost"
          label={t("open_balance.form_total_cost_label")}
          type="number"
          min="0"
          step="0.000001"
          value={formData.totalCost}
          onChange={(e) => handleChange("totalCost", e.target.value)}
          placeholder="0.000"
          required
        />

        {/* Inline error */}
        {error && (
          <p role="alert" className="text-sm text-m3-error">
            {t(error, { defaultValue: error })}
          </p>
        )}

        <div className="flex items-center justify-end gap-2 pt-2">
          <Button variant="secondary" type="button" onClick={onClose} disabled={isSubmitting}>
            {t("action.cancel")}
          </Button>
          <Button
            type="submit"
            form="ob-form"
            variant="primary"
            loading={isSubmitting}
            disabled={isSubmitting || !isFormValid}
          >
            {t("open_balance.action_submit")}
          </Button>
        </div>
      </form>
    </FormModal>
  );
}
