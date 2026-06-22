import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/ui/components/button/Button";
import { DateField } from "@/ui/components/field/DateField";
import { FormModal } from "@/ui/components/modal/FormModal";
import { useFetchAccountPricesForDate } from "./useFetchAccountPricesForDate";

interface FetchPricesForDateModalProps {
  isOpen: boolean;
  onClose: () => void;
  accountId: string;
}

/**
 * Modal that fetches every fetchable holding's Yahoo close at a user-picked date
 * and stores it keyed to that date (carry-back over closed market days). Isolated
 * from the latest-price "Refresh prices" path. Closes itself on a successful fetch;
 * the account view re-reads via the `AssetPriceUpdated` event.
 */
export function FetchPricesForDateModal({
  isOpen,
  onClose,
  accountId,
}: FetchPricesForDateModalProps) {
  const { t } = useTranslation();
  const { date, setDate, isSubmitting, submit } = useFetchAccountPricesForDate(accountId, onClose);

  const handleSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    void submit();
  };

  const footer = useMemo(
    () => (
      <div className="flex items-center justify-end gap-2">
        <Button
          id="fetch-prices-for-date-cancel"
          variant="secondary"
          onClick={onClose}
          disabled={isSubmitting}
        >
          {t("action.cancel")}
        </Button>
        <Button
          id="fetch-prices-for-date-submit"
          type="submit"
          form="fetch-prices-for-date-form"
          variant="primary"
          loading={isSubmitting}
          disabled={isSubmitting || date === ""}
        >
          {t("account_details.action_fetch_prices_for_date_submit")}
        </Button>
      </div>
    ),
    [isSubmitting, date, t, onClose],
  );

  return (
    <FormModal
      isOpen={isOpen}
      onClose={onClose}
      title={t("account_details.fetch_prices_for_date_modal_title")}
      footer={footer}
      maxWidth="max-w-md"
    >
      <form id="fetch-prices-for-date-form" onSubmit={handleSubmit} className="flex flex-col gap-4">
        <p className="text-sm text-m3-on-surface-variant">
          {t("account_details.fetch_prices_for_date_description")}
        </p>
        <DateField
          id="fetch-prices-for-date-date"
          label={t("account_details.fetch_prices_for_date_label")}
          value={date}
          onChange={(e) => setDate(e.target.value)}
          required
        />
      </form>
    </FormModal>
  );
}
