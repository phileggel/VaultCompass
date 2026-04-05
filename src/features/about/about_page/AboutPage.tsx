import { RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "@/lib/store";
import { Button } from "@/ui/components";
import { useAboutPage } from "./useAboutPage";

// R25 — About page: shows current version and manual check button
export function AboutPage() {
  const { t } = useTranslation();
  const { checkStatus, handleCheckForUpdate } = useAboutPage();

  const currentVersion = useAppStore((state) => state.appVersion);

  return (
    <div className="flex flex-col gap-6 p-6 max-w-md">
      <h2 className="text-2xl font-medium text-m3-on-surface">{t("about.title")}</h2>
      {/* Current version */}
      <div className="flex items-baseline gap-3">
        <span className="text-m3-on-surface-variant text-sm">{t("about.version_label")}</span>
        <span className="text-m3-on-surface font-medium font-mono">{currentVersion}</span>
      </div>

      {/* R25 — manual check button; R26 — disabled + spinner during check */}
      <div className="flex flex-col gap-3">
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

        {/* R27 — "up to date" feedback after manual check finds nothing */}
        {checkStatus === "up_to_date" && (
          <p className="text-sm text-m3-on-surface-variant" role="status">
            {t("about.up_to_date")}
          </p>
        )}

        {/* Error feedback when check fails */}
        {checkStatus === "error" && (
          <p className="text-sm text-m3-error" role="alert">
            {t("about.check_failed")}
          </p>
        )}
      </div>
    </div>
  );
}
