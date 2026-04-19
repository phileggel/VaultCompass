import { ArrowLeft } from "lucide-react";
import { useEffect } from "react";
import { logger } from "@/lib/logger";
import { IconButton } from "@/ui/components/button/IconButton";
import { ThemeToggle } from "./theme_toggle/ThemeToggle";
import { useHeaderConfig } from "./useHeaderConfig";

export function Header() {
  const { title, onBack } = useHeaderConfig();

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
      {onBack && (
        <IconButton
          icon={<ArrowLeft size={20} />}
          onClick={onBack}
          aria-label="Back"
          className="text-white hover:enabled:bg-white/20"
        />
      )}
      <div className="flex-1">
        <h1 className="text-lg font-semibold leading-tight">{title}</h1>
      </div>
      <ThemeToggle />
    </header>
  );
}
