import { useEffect, useState } from "react";
import type { AssetClass } from "@/bindings";
import { useCategories } from "@/features/categories/useCategories";
import { useAssets } from "../useAssets";

interface UseAddAssetProps {
  initialData?: {
    name: string;
    reference: string;
    class: AssetClass;
    currency: string;
    risk_level: number;
    category_id: string;
  };
  onSubmitSuccess?: () => void;
}

export function useAddAsset({ initialData, onSubmitSuccess }: UseAddAssetProps = {}) {
  const { addAsset } = useAssets();
  const { categories } = useCategories();

  const [formData, setFormData] = useState({
    name: initialData?.name || "",
    reference: initialData?.reference || "",
    class: initialData?.class || ("Stocks" as AssetClass),
    currency: initialData?.currency || "USD",
    risk_level: initialData?.risk_level || 3,
    category_id: initialData?.category_id || "",
  });

  // Effect to set initial category once categories are loaded
  useEffect(() => {
    if (!formData.category_id && categories.length > 0) {
      const firstCat = categories[0];
      if (firstCat) {
        setFormData((prev) => ({ ...prev, category_id: firstCat.id }));
      }
    }
  }, [categories, formData.category_id]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value } = e.target;
    setFormData((prev) => ({
      ...prev,
      [name]: name === "risk_level" ? parseInt(value, 10) : value,
    }));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    const success = await addAsset({
      name: formData.name,
      reference: formData.reference || null,
      class: formData.class as AssetClass,
      currency: formData.currency,
      risk_level: formData.risk_level,
      category_id: formData.category_id,
    });

    if (success && onSubmitSuccess) {
      onSubmitSuccess();
    }

    if (!initialData && success) {
      const firstCat = categories[0];
      setFormData({
        name: "",
        reference: "",
        class: "Stocks",
        currency: "USD",
        risk_level: 3,
        category_id: firstCat ? firstCat.id : "",
      });
    }
  };

  return {
    formData,
    handleChange,
    handleSubmit,
    categories,
  };
}
