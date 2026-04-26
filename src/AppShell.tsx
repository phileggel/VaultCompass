import { Outlet, useRouterState } from "@tanstack/react-router";
import { useEffect } from "react";
import { MainLayout } from "@/features/shell/MainLayout";
import { saveLastPath } from "@/lib/lastPath";

export function AppShell() {
  const pathname = useRouterState({ select: (s) => s.location.pathname });

  useEffect(() => {
    saveLastPath(pathname);
  }, [pathname]);

  return (
    <MainLayout>
      <Outlet />
    </MainLayout>
  );
}
