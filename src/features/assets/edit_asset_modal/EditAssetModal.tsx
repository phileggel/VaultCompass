import type { Asset } from "@/bindings";
import { Dialog } from "@/ui/components/modal/Dialog";
import { AssetForm } from "../shared/AssetForm";
import { useEditAssetModal } from "./useEditAssetModal";

interface EditAssetModalProps {
  isOpen: boolean;
  onClose: () => void;
  asset: Asset | null;
}

export function EditAssetModal({ isOpen, onClose, asset }: EditAssetModalProps) {
  const { formData, handleChange, handleSubmit, categories } = useEditAssetModal({
    asset,
    onClose,
  });

  const actions = (
    <>
      <button
        type="button"
        onClick={onClose}
        className="px-6 py-2.5 rounded-full text-sm font-medium text-m3-primary hover:bg-m3-primary/5 transition-all"
      >
        Cancel
      </button>
      <button type="submit" form="edit-asset-form" className="m3-button-filled px-8">
        Save Changes
      </button>
    </>
  );

  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      title="Edit Asset"
      actions={actions}
      maxWidth="max-w-xl"
    >
      <form id="edit-asset-form" className="py-2" onSubmit={handleSubmit}>
        <AssetForm
          formData={formData}
          handleChange={handleChange}
          categories={categories}
          idPrefix="edit"
        />
      </form>
    </Dialog>
  );
}
