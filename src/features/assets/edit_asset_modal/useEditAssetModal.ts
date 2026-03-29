import { useEffect, useState } from "react";
import type { Asset, AssetClass } from "@/bindings";
import { useCategories } from "@/features/categories/useCategories";
import { useAssets } from "../useAssets";

interface UseEditAssetModalProps {
  asset: Asset | null;
  onClose: () => void;
}

export function useEditAssetModal({ asset, onClose }: UseEditAssetModalProps) {
  const { updateAsset } = useAssets();
  const { categories } = useCategories();

  const [formData, setFormData] = useState({
    name: "",
    reference: "",
    class: "Stocks" as AssetClass,
    currency: "USD",
    risk_level: 3,
    category_id: "",
  });

  // Sync form data when asset changes
  useEffect(() => {
    if (asset) {
      setFormData({
        name: asset.name,
        reference: asset.reference || "",
        class: asset.class,
        currency: asset.currency,
        risk_level: asset.risk_level,
        category_id: asset.category.id,
      });
    }
  }, [asset]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value } = e.target;
    setFormData((prev) => ({
      ...prev,
      [name]: name === "risk_level" ? parseInt(value, 10) : value,
    }));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (asset) {
      const success = await updateAsset({
        asset_id: asset.id,
        name: formData.name,
        reference: formData.reference || null,
        class: formData.class as AssetClass,
        currency: formData.currency,
        risk_level: formData.risk_level,
        category_id: formData.category_id,
      });
      if (success) {
        onClose();
      }
    }
  };

  return {
    formData,
    handleChange,
    handleSubmit,
    categories,
  };
}
