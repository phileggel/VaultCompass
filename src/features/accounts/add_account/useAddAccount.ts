import { useState } from "react";
import type { AccountFormData } from "../shared/AccountForm";
import { FREQUENCIES } from "../shared/presenter";
import { validateAccountName } from "../shared/validateAccount";
import { useAccounts } from "../useAccounts";

interface UseAddAccountProps {
  onSubmitSuccess?: () => void;
}

export function useAddAccount({ onSubmitSuccess }: UseAddAccountProps = {}) {
  const { addAccount } = useAccounts();

  const [formData, setFormData] = useState<AccountFormData>({
    name: "",
    update_frequency: "ManualMonth",
  });
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value } = e.target;
    setFormData((prev) => ({ ...prev, [name]: value }));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    // R14 — block if name is empty or whitespace-only
    const validationError = validateAccountName(formData.name);
    if (validationError) {
      setError(validationError);
      return;
    }

    setError(null);
    setIsSubmitting(true);

    const result = await addAccount({
      name: formData.name,
      update_frequency: formData.update_frequency,
    });

    setIsSubmitting(false);

    // R13 — keep modal open on error
    if (result.error) {
      setError(result.error);
      return;
    }

    setFormData({ name: "", update_frequency: "ManualMonth" });

    if (onSubmitSuccess) {
      onSubmitSuccess();
    }
  };

  return {
    formData,
    error,
    isSubmitting,
    handleChange,
    handleSubmit,
    frequencies: FREQUENCIES,
  };
}
