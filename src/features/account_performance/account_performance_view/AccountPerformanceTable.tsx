import { useTranslation } from "react-i18next";
import type { PeriodRowViewModel } from "../shared/presenter";

interface AccountPerformanceTableProps {
  rows: PeriodRowViewModel[];
  showYtd: boolean;
}

export function AccountPerformanceTable({ rows, showYtd }: AccountPerformanceTableProps) {
  const { t } = useTranslation();

  return (
    <div className="m3-table-container">
      <table data-testid="account-performance-table" className="w-full border-collapse">
        <thead className="sticky top-0 bg-m3-surface-container z-10">
          <tr>
            <th className="m3-th">{t("account_performance.column_period")}</th>
            <th className="m3-th text-right">{t("account_performance.column_end_value")}</th>
            <th className="m3-th text-right">
              {t("account_performance.column_period_over_period")}
            </th>
            {/* PRF-037 — YTD column present only in month view */}
            {showYtd && (
              <th data-testid="account-performance-col-ytd" className="m3-th text-right">
                {t("account_performance.column_year_to_date")}
              </th>
            )}
            <th className="m3-th text-right">{t("account_performance.column_since_inception")}</th>
          </tr>
        </thead>
        <tbody>
          {/* PRF-041 — rows rendered in backend order (most-recent first) */}
          {rows.map((row) => (
            <tr key={row.rowKey} data-testid={`account-performance-row-${row.rowKey}`}>
              <td className="m3-td">{row.month !== null ? t(row.periodLabel) : row.periodLabel}</td>
              <td className="m3-td text-right">{row.endValueFormatted}</td>
              <td className={`m3-td text-right ${row.periodOverPeriod.colorClass}`}>
                {row.periodOverPeriod.gainFormatted}{" "}
                <span className="text-xs">({row.periodOverPeriod.pctFormatted})</span>
              </td>
              {showYtd && (
                <td className={`m3-td text-right ${row.yearToDate?.colorClass ?? ""}`}>
                  {row.yearToDate?.gainFormatted ?? "—"}{" "}
                  <span className="text-xs">({row.yearToDate?.pctFormatted ?? "—"})</span>
                </td>
              )}
              <td className={`m3-td text-right ${row.sinceInception.colorClass}`}>
                {row.sinceInception.gainFormatted}{" "}
                <span className="text-xs">({row.sinceInception.pctFormatted})</span>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
