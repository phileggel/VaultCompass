import { ChevronDown, TrendingDown, TrendingUp } from "lucide-react";
import { useMemo, useState } from "react";
import type { Account } from "@/bindings";
import { useAccounts } from "../accounts";
import { useAccountAssetDetails } from "./useAccountAssetDetails";

type TimePeriod = "week" | "month" | "year";

export function AccountAssetDetailsView() {
  const { accounts } = useAccounts();
  const [selectedAccountId, setSelectedAccountId] = useState<string | null>(null);
  const [timePeriod, setTimePeriod] = useState<TimePeriod>("month");

  // Generate dates based on time period
  const dates = useMemo(() => {
    const today = new Date();
    const datesList: string[] = [];
    const maxDays = timePeriod === "week" ? 7 : timePeriod === "month" ? 30 : 365;

    for (let i = 0; i < maxDays; i += timePeriod === "week" ? 1 : timePeriod === "month" ? 7 : 30) {
      const date = new Date(today);
      date.setDate(today.getDate() - i);
      const isoDate = date.toISOString().split("T")[0];
      if (isoDate) {
        datesList.push(isoDate);
      }
    }

    return datesList;
  }, [timePeriod]);

  const { data: snapshots, loading, error } = useAccountAssetDetails(selectedAccountId, dates);

  interface AssetRow {
    id: string;
    asset_name: string;
    ticker: string | null;
    quantity: number;
    purchase_price: number;
    dataPoints: Record<string, { price: number; value_eur: number; performance_eur: number }>;
  }

  // Transform data for table display
  const tableData = useMemo(() => {
    if (snapshots.length === 0) return [];

    // Get unique assets from all snapshots
    const assetMap = new Map<string, AssetRow>();
    snapshots.forEach((snapshot) => {
      snapshot.snapshots.forEach((asset) => {
        if (!assetMap.has(asset.id)) {
          assetMap.set(asset.id, {
            id: asset.id,
            asset_name: asset.asset_name,
            ticker: asset.ticker,
            quantity: asset.quantity,
            purchase_price: asset.purchase_price,
            dataPoints: {},
          });
        }
      });
    });

    // Populate data points for each date
    snapshots.forEach((snapshot) => {
      snapshot.snapshots.forEach((asset) => {
        const row = assetMap.get(asset.id);
        if (row) {
          row.dataPoints[snapshot.date] = {
            price: asset.price,
            value_eur: asset.value_eur,
            performance_eur: asset.performance_eur,
          };
        }
      });
    });

    return Array.from(assetMap.values());
  }, [snapshots]);

  const columnDates = useMemo(() => {
    if (snapshots.length === 0) return [];
    return snapshots.map((s) => s.date);
  }, [snapshots]);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h2 className="text-2xl font-bold text-gray-900 mb-2">Account Asset Details</h2>
        <p className="text-gray-500">Track your asset performance over time</p>
      </div>

      {/* Controls */}
      <div className="flex flex-col sm:flex-row gap-4 bg-white p-4 rounded-lg border border-gray-200">
        {/* Account Selector */}
        <div className="flex-1">
          <label htmlFor="account-select" className="block text-sm font-medium text-gray-700 mb-2">
            Account
          </label>
          <div className="relative">
            <select
              id="account-select"
              value={selectedAccountId || ""}
              onChange={(e) => setSelectedAccountId(e.target.value || null)}
              className="w-full pl-3 pr-10 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent appearance-none bg-white"
            >
              <option value="">Select an account...</option>
              {accounts.map((account: Account) => (
                <option key={account.id} value={account.id}>
                  {account.name}
                </option>
              ))}
            </select>
            <ChevronDown className="absolute right-3 top-2.5 h-5 w-5 text-gray-400 pointer-events-none" />
          </div>
        </div>

        {/* Time Period Selector */}
        <div className="flex-1">
          <label htmlFor="period-select" className="block text-sm font-medium text-gray-700 mb-2">
            Time Period
          </label>
          <div className="relative">
            <select
              id="period-select"
              value={timePeriod}
              onChange={(e) => setTimePeriod(e.target.value as TimePeriod)}
              className="w-full pl-3 pr-10 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent appearance-none bg-white"
            >
              <option value="week">Weekly</option>
              <option value="month">Monthly</option>
              <option value="year">Yearly</option>
            </select>
            <ChevronDown className="absolute right-3 top-2.5 h-5 w-5 text-gray-400 pointer-events-none" />
          </div>
        </div>
      </div>

      {/* Error State */}
      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700">{error}</div>
      )}

      {/* Loading State */}
      {loading && (
        <div className="bg-white rounded-lg border border-gray-200 p-8 text-center text-gray-500">
          Loading...
        </div>
      )}

      {/* Data Table */}
      {!loading && tableData.length > 0 && (
        <div className="overflow-x-auto bg-white rounded-lg border border-gray-200 shadow-sm">
          <table className="w-full">
            <thead>
              <tr className="border-b border-gray-200 bg-gray-50">
                <th className="px-4 py-3 text-left text-xs font-medium text-gray-700 uppercase tracking-wider sticky left-0 bg-gray-50 z-10">
                  Asset
                </th>
                <th className="px-4 py-3 text-left text-xs font-medium text-gray-700 uppercase tracking-wider sticky left-50 bg-gray-50 z-10">
                  Qty / PRU
                </th>
                {columnDates.map((date) => (
                  <th
                    key={date}
                    className="px-4 py-3 text-right text-xs font-medium text-gray-700 uppercase tracking-wider whitespace-nowrap"
                  >
                    <div className="font-semibold">
                      {new Date(date).toLocaleDateString("en-US", {
                        month: "short",
                        day: "numeric",
                      })}
                    </div>
                    <div className="text-xs text-gray-500">EUR</div>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {tableData.map((row, idx) => (
                <tr key={row.id} className={idx % 2 === 0 ? "bg-white" : "bg-gray-50"}>
                  <td className="px-4 py-3 text-sm font-medium text-gray-900 sticky left-0 z-10 bg-inherit">
                    <div>{row.asset_name}</div>
                    {row.ticker && <div className="text-xs text-gray-500">{row.ticker}</div>}
                  </td>
                  <td className="px-4 py-3 text-sm text-gray-700 sticky left-50 z-10 bg-inherit whitespace-nowrap">
                    <div>{row.quantity.toFixed(2)}</div>
                    <div className="text-xs text-gray-500">{row.purchase_price.toFixed(2)}</div>
                  </td>
                  {columnDates.map((date) => {
                    const point = row.dataPoints[date];
                    if (!point) {
                      return (
                        <td key={date} className="px-4 py-3 text-sm text-gray-400 text-right">
                          —
                        </td>
                      );
                    }
                    const isPositive = point.performance_eur >= 0;
                    return (
                      <td
                        key={date}
                        className="px-4 py-3 text-sm text-right border-l border-gray-200 bg-yellow-50"
                        title={`Value: €${point.value_eur.toFixed(2)}`}
                      >
                        <div className="font-semibold text-gray-900">
                          {point.value_eur.toFixed(2)}
                        </div>
                        <div
                          className={`text-xs font-medium flex items-center justify-end gap-1 ${isPositive ? "text-green-600" : "text-red-600"}`}
                        >
                          {isPositive ? (
                            <TrendingUp className="h-3 w-3" />
                          ) : (
                            <TrendingDown className="h-3 w-3" />
                          )}
                          {isPositive ? "+" : ""}
                          {point.performance_eur.toFixed(2)}
                        </div>
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Empty State */}
      {!loading && tableData.length === 0 && !error && (
        <div className="bg-white rounded-lg border border-gray-200 p-12 text-center">
          <p className="text-gray-500 text-lg mb-2">
            {selectedAccountId
              ? "No data available for this account"
              : "Select an account to view asset details"}
          </p>
        </div>
      )}
    </div>
  );
}
