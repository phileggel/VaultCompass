import { useEffect, useState } from "react";
import type { Account, UpdateFrequency } from "@/bindings";
import { FREQUENCY_LABELS } from "../shared/constants";
import { useAccounts } from "../useAccounts";

interface UseEditAccountModalProps {
  account: Account | null;
  onClose: () => void;
}

export function useEditAccountModal({ account, onClose }: UseEditAccountModalProps) {
  const { updateAccount } = useAccounts();

  const [formData, setFormData] = useState({
    name: "",
    update_frequency: "ManualMonth" as UpdateFrequency,
  });

  // Sync form data when account changes
  useEffect(() => {
    if (account) {
      setFormData({
        name: account.name,
        update_frequency: account.update_frequency,
      });
    }
  }, [account]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value } = e.target;
    setFormData((prev) => ({
      ...prev,
      [name]: value,
    }));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (account) {
      const success = await updateAccount({
        id: account.id,
        name: formData.name,
        update_frequency: formData.update_frequency,
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
    frequencies: Object.keys(FREQUENCY_LABELS) as UpdateFrequency[],
  };
}
