import type { Account } from "@/bindings";
import { Dialog } from "@/ui/components/modal/Dialog";
import { AccountForm } from "../shared/AccountForm";
import { useEditAccountModal } from "./useEditAccountModal";

interface EditAccountModalProps {
  isOpen: boolean;
  onClose: () => void;
  account: Account | null;
}

export function EditAccountModal({ isOpen, onClose, account }: EditAccountModalProps) {
  const { formData, handleChange, handleSubmit, frequencies } = useEditAccountModal({
    account,
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
      <button type="submit" form="edit-account-form" className="m3-button-filled px-8">
        Save Changes
      </button>
    </>
  );

  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      title="Edit Account"
      actions={actions}
      maxWidth="max-w-xl"
    >
      <form id="edit-account-form" className="py-2" onSubmit={handleSubmit}>
        <AccountForm
          formData={formData}
          handleChange={handleChange}
          frequencies={frequencies}
          idPrefix="edit"
        />
      </form>
    </Dialog>
  );
}
