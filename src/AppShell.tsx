import { Outlet, useRouterState } from "@tanstack/react-router";
import { useEffect } from "react";
import { AddTransactionModalMount } from "@/features/shell/AddTransactionModalMount";
import { AssetEditModalMount } from "@/features/shell/AssetEditModalMount";
import { CashTransactionEditMount } from "@/features/shell/CashTransactionEditMount";
import { CurrencyRateEditMount } from "@/features/shell/CurrencyRateEditMount";
import { FreeSharesEditModalMount } from "@/features/shell/FreeSharesEditModalMount";
import { MainLayout } from "@/features/shell/MainLayout";
import { UnpricedPricesModalMount } from "@/features/shell/UnpricedPricesModalMount";
import { saveLastPath } from "@/lib/lastPath";

export function AppShell() {
  const pathname = useRouterState({ select: (s) => s.location.pathname });

  useEffect(() => {
    saveLastPath(pathname);
  }, [pathname]);

  return (
    <MainLayout>
      <Outlet />
      <AssetEditModalMount />
      <CashTransactionEditMount />
      <CurrencyRateEditMount />
      <FreeSharesEditModalMount />
      <UnpricedPricesModalMount />
      <AddTransactionModalMount />
    </MainLayout>
  );
}
