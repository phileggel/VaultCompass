interface StatCardProps {
  label: string;
  value: string;
  change: string;
  positive: boolean;
}

export function StatCard({ label, value, change, positive }: StatCardProps) {
  return (
    <div className="m3-card-elevated">
      <p className="text-sm font-medium text-m3-on-surface-variant uppercase tracking-wider">
        {label}
      </p>
      <div className="flex items-end justify-between mt-2">
        <span className="text-2xl font-bold text-m3-on-surface">{value}</span>
        <span
          className={`text-xs font-bold px-2 py-0.5 rounded-full ${positive ? "bg-emerald-100 text-emerald-700" : "bg-rose-100 text-rose-700"}`}
        >
          {change}
        </span>
      </div>
    </div>
  );
}
