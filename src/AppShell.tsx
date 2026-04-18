import { Outlet } from "@tanstack/react-router";
import { MainLayout } from "@/features/shell/MainLayout";

export function AppShell() {
  return (
    <MainLayout>
      <Outlet />
    </MainLayout>
  );
}
