import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useCategories } from "../useCategories";

interface UseAddCategoryProps {
  onSubmitSuccess?: () => void;
}

export function useAddCategory({ onSubmitSuccess }: UseAddCategoryProps = {}) {
  const { t } = useTranslation();
  const { addCategory } = useCategories();
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setName(e.target.value);
    setError(null);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    setIsSubmitting(true);
    const result = await addCategory(name.trim());
    if (result.error) {
      if (result.error === "error.DuplicateName") {
        setError(t("category.error_duplicate"));
      } else {
        setError(t("category.error_generic"));
      }
    } else {
      setName("");
      setError(null);
      onSubmitSuccess?.();
    }
    setIsSubmitting(false);
  };

  return { name, error, isSubmitting, handleChange, handleSubmit };
}
