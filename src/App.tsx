import { Loader2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { AboutPage } from "@/features/about";
import { AccountManager } from "@/features/accounts";
import { AssetManager } from "@/features/assets";
import { CategoryManager } from "@/features/categories";
import { DesignSystemPage } from "@/features/design-system";
import { shellGateway } from "@/features/shell/gateway";
import { MainLayout } from "@/features/shell/MainLayout";
import { logger } from "@/lib/logger";
import { useAppStore } from "@/lib/store";

function App() {
  const [activeItem, setActiveItem] = useState("Assets");
  const [dbError, setDbError] = useState<string | null>(null);
  const init = useAppStore((state) => state.init);
  const isInitialized = useAppStore((state) => state.isInitialized);
  const { t } = useTranslation();

  useEffect(() => {
    logger.info("[App] mounted");
    // R18 — listen for critical migration failure from backend
    const unlistenPromise = shellGateway.onMigrationError((message) => {
      setDbError(message);
    });
    return () => {
      unlistenPromise.then((fn) => fn()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    return init();
  }, [init]);

  // R18 — critical migration error: app blocked with error message
  if (dbError) {
    return (
      <div className="h-screen flex items-center justify-center bg-m3-surface p-8">
        <div className="max-w-md text-center flex flex-col gap-4">
          <p className="text-m3-error font-medium text-lg">{t("app.migration_error")}</p>
          <p className="text-m3-on-surface-variant text-sm font-mono">{dbError}</p>
        </div>
      </div>
    );
  }

  // R17 — loading screen while migrations/init are running
  if (!isInitialized) {
    return (
      <div className="h-screen flex items-center justify-center bg-m3-surface">
        <div className="flex flex-col items-center gap-3">
          <Loader2 className="w-6 h-6 animate-spin text-m3-primary" />
          <p className="text-m3-on-surface-variant text-sm">{t("app.migration_in_progress")}</p>
        </div>
      </div>
    );
  }

  return (
    <MainLayout activeItem={activeItem} onNavItemClick={setActiveItem}>
      <div className="h-full py-2 px-2 overflow-auto">
        {activeItem === "Assets" && <AssetManager />}

        {activeItem === "Accounts" && <AccountManager />}

        {activeItem === "Categories" && <CategoryManager />}

        {/* R25 — About page with manual update check */}
        {activeItem === "About" && <AboutPage />}

        {import.meta.env.DEV && activeItem === "Design System" && <DesignSystemPage />}
      </div>
    </MainLayout>
  );
}

export default App;
