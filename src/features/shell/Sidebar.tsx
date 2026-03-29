import { Menu, Settings, TrendingUp, X } from "lucide-react";
import { IconButton } from "@/ui/components";
import { NAV_ITEMS } from "./useSidebar";

interface SidebarProps {
  isOpen: boolean;
  toggleDrawer: () => void;
  activeItem: string;
  onNavItemClick: (label: string) => void;
  onSettingsClick?: () => void;
}

export function Sidebar({
  isOpen,
  toggleDrawer,
  activeItem,
  onNavItemClick,
  onSettingsClick,
}: SidebarProps) {
  return (
    <aside
      className={`
        relative z-50 h-full bg-m3-surface-container-high drawer-transition
        ${isOpen ? "w-80" : "w-20"}
        overflow-hidden flex flex-col shrink-0
      `}
    >
      <div className="p-4 flex items-center h-16">
        <IconButton
          variant="ghost"
          shape="round"
          size="md"
          icon={isOpen ? <X size={24} /> : <Menu size={24} />}
          aria-label={isOpen ? "Collapse menu" : "Expand menu"}
          onClick={toggleDrawer}
          className={!isOpen ? "mx-auto" : undefined}
        />
        {isOpen && (
          <div className="flex items-center gap-3 ml-2">
            <div className="w-8 h-8 bg-m3-primary rounded-lg flex items-center justify-center text-m3-on-primary">
              <TrendingUp size={20} />
            </div>
            {/* "Vault M3" is the fixed product brand name — intentionally not i18n'd */}
            <span className="text-lg font-bold text-m3-on-surface tracking-tight">Vault M3</span>
          </div>
        )}
      </div>

      <nav className="flex-1 px-3 mt-2 space-y-1">
        {NAV_ITEMS.map((item) => (
          <button
            type="button"
            key={item.label}
            onClick={() => onNavItemClick(item.label)}
            aria-label={item.label}
            className={`
              w-full m3-navigation-item
              ${activeItem === item.label ? "m3-navigation-item-active" : ""}
              ${!isOpen && "justify-center px-0"}
            `}
          >
            <item.icon size={24} />
            {isOpen && <span>{item.label}</span>}
          </button>
        ))}
      </nav>

      {/* Footer — tonal surface shift instead of border (No-Line rule) */}
      <div className="px-3 py-4 bg-m3-surface-container-highest">
        <button
          type="button"
          onClick={onSettingsClick}
          aria-label="Settings"
          className={`w-full m3-navigation-item ${!isOpen && "justify-center px-0"}`}
        >
          <Settings size={24} />
          {isOpen && <span>Settings</span>}
        </button>
      </div>
    </aside>
  );
}
