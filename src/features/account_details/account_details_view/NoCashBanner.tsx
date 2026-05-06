import { ArrowDownToLine } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/ui/components/button/Button";

interface NoCashBannerProps {
  onRecordDeposit: () => void;
}

/**
 * Inline banner shown above the active holdings table when no cash is recorded yet
 * for an account that already has positions (CSH-095). Hidden when the account has
 * no positions at all — the existing "no positions yet" empty state takes over there.
 */
export function NoCashBanner({ onRecordDeposit }: NoCashBannerProps) {
  const { t } = useTranslation();
  return (
    <div className="m-4 flex items-center justify-between rounded-2xl border border-m3-outline-variant bg-m3-surface-container-high px-4 py-3">
      <p className="text-sm text-m3-on-surface-variant">{t("cash.no_cash_banner_message")}</p>
      <Button
        variant="primary"
        size="sm"
        icon={<ArrowDownToLine size={14} />}
        onClick={onRecordDeposit}
      >
        {t("cash.no_cash_banner_cta")}
      </Button>
    </div>
  );
}
