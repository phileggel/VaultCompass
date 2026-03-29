import { useEffect, useState } from "react";
import { Content } from "./Content";
import { Footer } from "./Footer";
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
    localStorage.setItem("drawer_open", isDrawerOpen.toString());
  }, [isDrawerOpen]);

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

        <Content>{children}</Content>

        <Footer />
      </div>
    </div>
  );
}
