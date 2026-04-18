import { useRouterState } from "@tanstack/react-router";
import { useEffect } from "react";
import { logger } from "@/lib/logger";
import { ThemeToggle } from "./theme_toggle/ThemeToggle";
import { NAV_ITEMS } from "./useSidebar";

function resolveTitle(pathname: string): string {
  const exact = NAV_ITEMS.find((item) => item.path === pathname);
  if (exact) return exact.label;
  const parent = NAV_ITEMS.find(
    (item) => item.path !== "/" && pathname.startsWith(`${item.path}/`),
  );
  return parent?.label ?? "";
}

export function Header() {
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const title = resolveTitle(pathname);

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
