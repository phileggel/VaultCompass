import { useState } from "react";
import type { UpdateFrequency } from "@/bindings";
import { FREQUENCY_LABELS } from "../shared/constants";
import { useAccounts } from "../useAccounts";

interface UseAccountFormProps {
  initialData?: {
    name: string;
    update_frequency: UpdateFrequency;
  };
  onSubmitSuccess?: () => void;
}

export function useAddAccount({ initialData, onSubmitSuccess }: UseAccountFormProps = {}) {
  const { addAccount } = useAccounts();

  const [formData, setFormData] = useState({
    name: initialData?.name || "",
    update_frequency: initialData?.update_frequency || ("ManualMonth" as UpdateFrequency),
  });

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value } = e.target;
    setFormData((prev) => ({
      ...prev,
      [name]: value,
    }));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!initialData) {
      const success = await addAccount({
        name: formData.name,
        update_frequency: formData.update_frequency as UpdateFrequency,
      });
      if (success && onSubmitSuccess) {
        onSubmitSuccess();
      }
    }

    if (!initialData) {
      setFormData({
        name: "",
        update_frequency: "ManualMonth",
      });
    }
  };

  return {
    formData,
    handleChange,
    handleSubmit,
    frequencies: Object.keys(FREQUENCY_LABELS) as UpdateFrequency[],
  };
}
