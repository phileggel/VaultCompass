import type { UpdateFrequency } from "@/bindings";
import { SelectField } from "@/ui/components/field/SelectField";
import { TextField } from "@/ui/components/field/TextField";
import { FREQUENCY_LABELS } from "./constants";

interface AccountFormProps {
  formData: {
    name: string;
    update_frequency: UpdateFrequency;
  };
  handleChange: (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => void;
  frequencies: UpdateFrequency[];
  idPrefix?: string;
}

export function AccountForm({
  formData,
  handleChange,
  frequencies,
  idPrefix = "account",
}: AccountFormProps) {
  const frequencyOptions = frequencies.map((freq) => ({
    label: FREQUENCY_LABELS[freq],
    value: freq,
  }));

  return (
    <div className="space-y-6">
      <TextField
        label="Account Name"
        id={`${idPrefix}-name`}
        name="name"
        required
        placeholder="e.g. My Savings, Brokerage"
        value={formData.name}
        onChange={handleChange}
      />

      <SelectField
        label="Update Frequency"
        id={`${idPrefix}-update-frequency`}
        name="update_frequency"
        value={formData.update_frequency}
        onChange={handleChange}
        options={frequencyOptions}
      />
    </div>
  );
}
