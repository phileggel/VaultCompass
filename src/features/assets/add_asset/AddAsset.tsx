import type { AssetClass } from "@/bindings";
import { AssetForm } from "../shared/AssetForm";
import { useAddAsset } from "./useAddAsset";

interface AddAssetProps {
  initialData?: {
    name: string;
    reference: string;
    class: AssetClass;
    currency: string;
    risk_level: number;
    category_id: string;
  };
  onSubmitSuccess?: () => void;
  submitLabel?: string;
}

export function AddAsset({
  initialData,
  onSubmitSuccess,
  submitLabel = "Create Asset",
}: AddAssetProps) {
  const { formData, handleChange, handleSubmit, categories } = useAddAsset({
    initialData,
    onSubmitSuccess,
  });

  return (
    <form className="space-y-6 pt-2" onSubmit={handleSubmit}>
      <AssetForm formData={formData} handleChange={handleChange} categories={categories} />

      <div className="pt-4">
        <button type="submit" className="m3-button-filled w-full text-base py-3">
          {submitLabel}
        </button>
      </div>
    </form>
  );
}
