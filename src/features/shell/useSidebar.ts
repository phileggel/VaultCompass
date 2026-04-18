import { Info, Palette, PieChart, Tag, Wallet } from "lucide-react";
import type { ElementType } from "react";

export interface NavItem {
  label: string;
  path: string;
  icon: ElementType;
}

const BASE_NAV_ITEMS: NavItem[] = [
  { label: "Assets", path: "/assets", icon: PieChart },
  { label: "Accounts", path: "/accounts", icon: Wallet },
  { label: "Categories", path: "/categories", icon: Tag },
  { label: "About", path: "/about", icon: Info },
];

const DEV_NAV_ITEMS: NavItem[] = import.meta.env.DEV
  ? [{ label: "Design System", path: "/design-system", icon: Palette }]
  : [];

export const NAV_ITEMS: NavItem[] = [...BASE_NAV_ITEMS, ...DEV_NAV_ITEMS];
