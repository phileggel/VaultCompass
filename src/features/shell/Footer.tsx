export function Footer() {
  return (
    <footer className="bg-m3-surface-container-high border-t border-m3-outline/10 p-4 flex flex-col sm:flex-row items-center justify-between gap-4 text-xs font-medium text-m3-on-surface-variant">
      <div className="flex items-center gap-6">
        <span>© 2026 Vault Manager</span>
        <div className="flex gap-4">
          <button type="button" className="hover:text-m3-primary cursor-pointer">
            Privacy
          </button>
          <button type="button" className="hover:text-m3-primary cursor-pointer">
            Terms
          </button>
        </div>
      </div>

      <div className="flex items-center gap-3">
        <span className="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-emerald-100 text-emerald-700">
          <div className="w-1.5 h-1.5 rounded-full bg-emerald-500"></div>
          Live Market Data
        </span>
        <span className="text-m3-outline">v0.1.0</span>
      </div>
    </footer>
  );
}
