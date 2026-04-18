import { useRouterState } from "@tanstack/react-router";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { logger } from "@/lib/logger";
import { ThemeToggle } from "./theme_toggle/ThemeToggle";
import { NAV_ITEMS } from "./useSidebar";

function usePageTitle(pathname: string): string {
  const { t } = useTranslation("common");
  const exact = NAV_ITEMS.find((item) => item.path === pathname);
  if (exact) return t(exact.labelKey);
  const parent = NAV_ITEMS.find(
    (item) => item.path !== "/" && pathname.startsWith(`${item.path}/`),
  );
  return parent ? t(parent.labelKey) : "";
}

export function Header() {
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const title = usePageTitle(pathname);

  useEffect(() => {
    logger.info("[Header] mounted");
  }, []);

  // text-white is intentional: lives exclusively on the fixed-brand indigo gradient
  // (--color-header-from/to). White is always accessible on rich indigo (WCAG AA).
  return (
    <header
      className="
        bg-linear-to-br from-header-from to-header-to
        text-white px-6
        flex items-center gap-4
        h-app-bar shrink-0
        relative z-50
        shadow-elevation-1
      "
    >
      <div className="flex-1">
        <h1 className="text-lg font-semibold leading-tight">{title}</h1>
      </div>
      <ThemeToggle />
    </header>
  );
}
