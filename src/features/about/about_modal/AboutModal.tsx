import { RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "@/lib/store";
import { Button } from "@/ui/components";
import { Dialog } from "@/ui/components/modal/Dialog";
import { useAboutPage } from "../about_page/useAboutPage";

interface AboutModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export function AboutModal({ isOpen, onClose }: AboutModalProps) {
  const { t } = useTranslation("common");
  const { checkStatus, handleCheckForUpdate } = useAboutPage();
  const currentVersion = useAppStore((state) => state.appVersion);

  return (
    <Dialog isOpen={isOpen} onClose={onClose} title={t("about.title")}>
      <div className="flex flex-col gap-6 py-2">
        <p className="text-sm text-m3-on-surface-variant leading-relaxed">
          {t("about.description")}
        </p>

        <div className="flex items-baseline gap-3">
          <span className="text-m3-on-surface-variant text-sm">{t("about.version_label")}</span>
          <span className="text-m3-on-surface font-medium font-mono">{currentVersion}</span>
        </div>

        <p className="text-xs text-m3-on-surface-variant">{t("about.license")}</p>

        <div className="flex flex-col gap-3 pb-2">
          <Button
            variant="outline"
            size="md"
            loading={checkStatus === "checking"}
            icon={<RefreshCw className="w-4 h-4" />}
            onClick={() => void handleCheckForUpdate()}
            className="w-fit"
          >
            {checkStatus === "checking" ? t("about.checking") : t("about.check_updates")}
          </Button>

          {checkStatus === "up_to_date" && (
            <p className="text-sm text-m3-on-surface-variant" role="status">
              {t("about.up_to_date")}
            </p>
          )}

          {checkStatus === "error" && (
            <p className="text-sm text-m3-error" role="alert">
              {t("about.check_failed")}
            </p>
          )}
        </div>
      </div>
    </Dialog>
  );
}
