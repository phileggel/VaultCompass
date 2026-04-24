import type { ChangeEvent } from "react";
import { useTranslation } from "react-i18next";
import type { UpdateFrequency } from "@/bindings";
import { SelectField } from "@/ui/components/field/SelectField";
import { TextField } from "@/ui/components/field/TextField";
import { FREQUENCY_I18N_KEYS } from "./presenter";

export interface AccountFormData {
  name: string;
  currency: string;
  update_frequency: UpdateFrequency;
}

interface AccountFormProps {
  formData: AccountFormData;
  handleChange: (e: ChangeEvent<HTMLInputElement | HTMLSelectElement>) => void;
  frequencies: UpdateFrequency[];
  idPrefix?: string;
}

export function AccountForm({
  formData,
  handleChange,
  frequencies,
  idPrefix = "account",
}: AccountFormProps) {
  const { t } = useTranslation();

  const frequencyOptions = frequencies.map((freq) => ({
    label: t(FREQUENCY_I18N_KEYS[freq]),
    value: freq,
  }));

  return (
    <div className="space-y-6">
      <TextField
        label={t("account.form_name_label")}
        id={`${idPrefix}-name`}
        name="name"
        required
        placeholder={t("account.form_name_placeholder")}
        value={formData.name}
        onChange={handleChange}
      />

      <TextField
        label={t("account.form_currency_label")}
        id={`${idPrefix}-currency`}
        name="currency"
        required
        maxLength={3}
        placeholder={t("account.form_currency_placeholder")}
        value={formData.currency}
        onChange={handleChange}
      />

      <SelectField
        label={t("account.form_frequency_label")}
        id={`${idPrefix}-update-frequency`}
        name="update_frequency"
        value={formData.update_frequency}
        onChange={handleChange}
        options={frequencyOptions}
      />
    </div>
  );
}
