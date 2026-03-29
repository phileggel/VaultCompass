import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { logger } from "@/lib/logger";
import { Button } from "@/ui/components/button/Button";
import { Dialog } from "@/ui/components/modal/Dialog";
import { AssetForm } from "../shared/AssetForm";
import { useAddAsset } from "./useAddAsset";

interface AddAssetModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export function AddAssetModal({ isOpen, onClose }: AddAssetModalProps) {
  const { t } = useTranslation();
  const {
    formData,
    error,
    isSubmitting,
    duplicateWarning,
    handleChange,
    handleClassChange,
    handleSubmit,
    categories,
  } = useAddAsset({ onSubmitSuccess: onClose });

  useEffect(() => {
    logger.info("[AddAssetModal] mounted");
  }, []);

  const isSubmitDisabled =
    !formData.name.trim() || !formData.reference.trim() || !formData.currency.trim();

  const actions = (
    <>
      <Button variant="secondary" onClick={onClose}>
        {t("action.cancel")}
      </Button>
      <Button
        type="submit"
        form="add-asset-form"
        variant="primary"
        loading={isSubmitting}
        disabled={isSubmitDisabled || isSubmitting}
      >
        {t("action.add")}
      </Button>
    </>
  );

  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      title={t("asset.add_modal_title")}
      actions={actions}
      maxWidth="max-w-xl"
    >
      <form id="add-asset-form" className="py-2" onSubmit={handleSubmit}>
        <AssetForm
          formData={formData}
          handleChange={handleChange}
          onClassChange={handleClassChange}
          categories={categories}
          duplicateWarning={duplicateWarning}
          idPrefix="add-asset"
        />
        {error && <p className="mt-3 text-sm text-m3-error">{error}</p>}
      </form>
    </Dialog>
  );
}
