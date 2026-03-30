import { BarChart3, Info, Palette, PieChart, Tag, Wallet } from "lucide-react";
import type { ElementType } from "react";

export interface NavItem {
  label: string;
  icon: ElementType;
}

const BASE_NAV_ITEMS: NavItem[] = [
  { label: "Assets", icon: PieChart },
  { label: "Accounts", icon: Wallet },
  { label: "Account Details", icon: BarChart3 },
  { label: "Categories", icon: Tag },
  { label: "About", icon: Info },
];

const DEV_NAV_ITEMS: NavItem[] = import.meta.env.DEV
  ? [{ label: "Design System", icon: Palette }]
  : [];

export const NAV_ITEMS: NavItem[] = [...BASE_NAV_ITEMS, ...DEV_NAV_ITEMS];
