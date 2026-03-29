import type { AssetCategory, AssetClass } from "@/bindings";
import { SelectField } from "@/ui/components/field/SelectField";
import { TextField } from "@/ui/components/field/TextField";
import { ASSET_CLASSES, RISK_LEVELS } from "./constants";

interface AssetFormData {
  name: string;
  reference: string;
  class: AssetClass;
  currency: string;
  risk_level: number;
  category_id: string;
}

interface AssetFormProps {
  formData: AssetFormData;
  handleChange: (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => void;
  categories: AssetCategory[];
  idPrefix?: string;
}

export function AssetForm({
  formData,
  handleChange,
  categories,
  idPrefix = "asset",
}: AssetFormProps) {
  const categoryOptions = categories.map((cat) => ({
    label: cat.name,
    value: cat.id,
  }));

  const classOptions = ASSET_CLASSES.map((c) => ({
    label: c,
    value: c,
  }));

  return (
    <div className="space-y-6">
      <TextField
        label="Asset Name"
        id={`${idPrefix}-name`}
        name="name"
        required
        placeholder="e.g. Apple Inc."
        value={formData.name}
        onChange={handleChange}
      />

      <div className="grid grid-cols-2 gap-4">
        <TextField
          label="Reference"
          id={`${idPrefix}-reference`}
          name="reference"
          placeholder="Ticker, ISIN, etc."
          value={formData.reference}
          onChange={handleChange}
        />
        <TextField
          label="Currency (ISO)"
          id={`${idPrefix}-currency`}
          name="currency"
          required
          className="uppercase"
          placeholder="USD"
          value={formData.currency}
          onChange={handleChange}
        />
      </div>

      <SelectField
        label="Category"
        id={`${idPrefix}-category`}
        name="category_id"
        value={formData.category_id}
        onChange={handleChange}
        options={categoryOptions}
      />

      <SelectField
        label="Class"
        id={`${idPrefix}-class`}
        name="class"
        value={formData.class}
        onChange={handleChange}
        options={classOptions}
      />

      <div className="flex flex-col gap-1.5">
        <span className="m3-input-label">Risk Level</span>
        <div className="flex p-1 bg-m3-surface-variant rounded-2xl gap-1 overflow-hidden">
          {RISK_LEVELS.map((level) => {
            const isSelected = formData.risk_level === level;

            return (
              <label
                key={level}
                className={`
                  relative flex-1 flex items-center justify-center py-2 rounded-xl 
                  text-sm font-bold cursor-pointer transition-all duration-200
                  ${
                    isSelected
                      ? "bg-m3-primary text-m3-on-primary"
                      : "text-m3-on-surface-variant hover:bg-m3-primary/10"
                  }
                `}
              >
                <input
                  type="radio"
                  name="risk_level"
                  value={level}
                  checked={isSelected}
                  onChange={handleChange}
                  className="absolute opacity-0 w-0 h-0 appearance-none"
                />
                <span>{level}</span>
              </label>
            );
          })}
        </div>
      </div>
    </div>
  );
}
