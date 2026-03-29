import type { UpdateFrequency } from "@/bindings";
import { AccountForm } from "../shared/AccountForm";
import { useAddAccount } from "./useAddAccount";

interface AddAccountProps {
  initialData?: {
    name: string;
    update_frequency: UpdateFrequency;
  };
  onSubmitSuccess?: () => void;
  submitLabel?: string;
}

export function AddAccount({
  initialData,
  onSubmitSuccess,
  submitLabel = "Create Account",
}: AddAccountProps) {
  const { formData, handleChange, handleSubmit, frequencies } = useAddAccount({
    initialData,
    onSubmitSuccess,
  });

  return (
    <form className="space-y-6 pt-2" onSubmit={handleSubmit}>
      <AccountForm formData={formData} handleChange={handleChange} frequencies={frequencies} />

      <div className="pt-4">
        <button type="submit" className="m3-button-filled w-full text-base py-3">
          {submitLabel}
        </button>
      </div>
    </form>
  );
}
