import { useEffect, useState } from "react";
import { UpdateBanner, useUpdateBanner } from "@/features/update";
import { logger } from "@/lib/logger";
import { Content } from "./Content";
import { Header } from "./Header";
import { Sidebar } from "./Sidebar";

interface MainLayoutProps {
  children: React.ReactNode;
  activeItem: string;
  onNavItemClick: (label: string) => void;
}

export function MainLayout({ children, activeItem, onNavItemClick }: MainLayoutProps) {
  const [isDrawerOpen, setDrawerOpen] = useState(() => {
    const saved = localStorage.getItem("drawer_open");
    return saved === null ? true : saved === "true";
  });

  useEffect(() => {
    logger.info("[MainLayout] mounted");
  }, []);

  useEffect(() => {
    localStorage.setItem("drawer_open", isDrawerOpen.toString());
  }, [isDrawerOpen]);

  // R4 — banner is part of permanent shell layout, visible on all pages
  const updateBannerData = useUpdateBanner();

  return (
    <div className="flex h-screen overflow-hidden bg-m3-surface">
      <Sidebar
        isOpen={isDrawerOpen}
        toggleDrawer={() => setDrawerOpen(!isDrawerOpen)}
        activeItem={activeItem}
        onNavItemClick={onNavItemClick}
      />

      {/* Main Container */}
      <div className="flex-1 flex flex-col min-w-0">
        <Header activeItem={activeItem} />

        {/* R3, R4 — update banner between header and content */}
        <UpdateBanner data={updateBannerData} />

        <Content>{children}</Content>
      </div>
    </div>
  );
}
