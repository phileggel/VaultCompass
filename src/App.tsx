import { useEffect, useState } from "react";
import { AccountAssetDetailsView } from "@/features/account_asset_details";
import { AccountManager } from "@/features/accounts";
import { AssetManager } from "@/features/assets";
import { CategoryManager } from "@/features/categories";
import { DesignSystemPage } from "@/features/design-system";
import { MainLayout } from "@/features/shell/MainLayout";
import { useAppStore } from "@/lib/store";

function App() {
  const [activeItem, setActiveItem] = useState("Assets");
  const init = useAppStore((state) => state.init);

  useEffect(() => {
    return init();
  }, [init]);

  return (
    <MainLayout activeItem={activeItem} onNavItemClick={setActiveItem}>
      <div className="h-full py-2 px-2 overflow-auto">
        {activeItem === "Assets" && <AssetManager />}

        {activeItem === "Accounts" && <AccountManager />}

        {activeItem === "Categories" && <CategoryManager />}

        {activeItem === "Account Details" && <AccountAssetDetailsView />}

        {import.meta.env.DEV && activeItem === "Design System" && <DesignSystemPage />}
      </div>
    </MainLayout>
  );
}

export default App;
