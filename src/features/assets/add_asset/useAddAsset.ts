import { useMemo, useState } from "react";
import type { AssetClass } from "@/bindings";
import { useCategories } from "@/features/categories/useCategories";
import { DEFAULT_RISK_BY_CLASS, SYSTEM_CATEGORY_ID } from "../shared/constants";
import { hasDuplicateReference } from "../shared/validateAsset";
import { useAssets } from "../useAssets";

interface UseAddAssetProps {
  onSubmitSuccess?: () => void;
}

export function useAddAsset({ onSubmitSuccess }: UseAddAssetProps = {}) {
  const { addAsset, assets } = useAssets();
  const { categories } = useCategories();

  const [formData, setFormData] = useState({
    name: "",
    reference: "",
    class: "Cash" as AssetClass,
    currency: "EUR",
    risk_level: DEFAULT_RISK_BY_CLASS.Cash,
    category_id: SYSTEM_CATEGORY_ID,
  });
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Duplicate reference warning — R9 (includes archived assets)
  const duplicateWarning = useMemo(
    () => hasDuplicateReference(formData.reference, assets),
    [formData.reference, assets],
  );

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value } = e.target;
    setFormData((prev) => ({
      ...prev,
      [name]: name === "risk_level" ? parseInt(value, 10) : value,
    }));
  };

  // Auto-fill risk_level when class changes — R10 (creation only)
  const handleClassChange = (assetClass: AssetClass) => {
    setFormData((prev) => ({
      ...prev,
      class: assetClass,
      risk_level: DEFAULT_RISK_BY_CLASS[assetClass],
    }));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setIsSubmitting(true);

    const result = await addAsset({
      name: formData.name,
      reference: formData.reference,
      class: formData.class,
      currency: formData.currency,
      risk_level: formData.risk_level,
      category_id: formData.category_id || SYSTEM_CATEGORY_ID,
    });

    setIsSubmitting(false);

    if (result.error) {
      setError(result.error);
      return;
    }

    if (onSubmitSuccess) {
      onSubmitSuccess();
    }

    setFormData({
      name: "",
      reference: "",
      class: "Cash",
      currency: "EUR",
      risk_level: DEFAULT_RISK_BY_CLASS.Cash,
      category_id: SYSTEM_CATEGORY_ID,
    });
  };

  return {
    formData,
    error,
    isSubmitting,
    duplicateWarning,
    handleChange,
    handleClassChange,
    handleSubmit,
    categories,
  };
}
