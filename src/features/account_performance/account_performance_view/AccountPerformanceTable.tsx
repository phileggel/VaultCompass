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
      <table
        id="account-performance-table"
        data-testid="account-performance-table"
        className="w-full border-collapse"
      >
        <thead className="sticky top-0 bg-m3-surface-container z-10">
          {/* Each performance metric spans a Value + % pair (grouped header). */}
          <tr>
            <th scope="col" rowSpan={2} className="m3-th align-bottom">
              {t("account_performance.column_period")}
            </th>
            {/* PRF-070-074 — Global Value bridge, read left-to-right as a sum:
                Prev + Cash + Asset + Dividends + P&L = End Value. */}
            <th scope="col" rowSpan={2} className="m3-th text-right align-bottom">
              {t("account_performance.column_prev_value")}
            </th>
            <th scope="col" rowSpan={2} className="m3-th text-right align-bottom">
              {t("account_performance.column_cash_flow")}
            </th>
            <th scope="col" rowSpan={2} className="m3-th text-right align-bottom">
              {t("account_performance.column_asset_flow")}
            </th>
            <th scope="col" rowSpan={2} className="m3-th text-right align-bottom">
              {t("account_performance.column_dividends")}
            </th>
            <th scope="col" rowSpan={2} className="m3-th text-right align-bottom">
              {t("account_performance.column_pnl")}
            </th>
            <th scope="col" rowSpan={2} className="m3-th text-right align-bottom">
              {t("account_performance.column_end_value")}
            </th>
            <th scope="colgroup" colSpan={2} className="m3-th text-center">
              {t("account_performance.column_period_over_period")}
            </th>
            {/* PRF-037 — YTD column present only in month view */}
            {showYtd && (
              <th
                id="account-performance-col-ytd"
                data-testid="account-performance-col-ytd"
                scope="colgroup"
                colSpan={2}
                className="m3-th text-center"
              >
                {t("account_performance.column_year_to_date")}
              </th>
            )}
            <th scope="colgroup" colSpan={2} className="m3-th text-center">
              {t("account_performance.column_since_inception")}
            </th>
          </tr>
          <tr>
            <th scope="col" id="account-performance-subcol-pop-value" className="m3-th text-right">
              {t("account_performance.subcol_value")}
            </th>
            <th scope="col" id="account-performance-subcol-pop-pct" className="m3-th text-right">
              {t("account_performance.subcol_pct")}
            </th>
            {showYtd && (
              <>
                <th
                  scope="col"
                  id="account-performance-subcol-ytd-value"
                  className="m3-th text-right"
                >
                  {t("account_performance.subcol_value")}
                </th>
                <th
                  scope="col"
                  id="account-performance-subcol-ytd-pct"
                  className="m3-th text-right"
                >
                  {t("account_performance.subcol_pct")}
                </th>
              </>
            )}
            <th
              scope="col"
              id="account-performance-subcol-since-value"
              className="m3-th text-right"
            >
              {t("account_performance.subcol_value")}
            </th>
            <th scope="col" id="account-performance-subcol-since-pct" className="m3-th text-right">
              {t("account_performance.subcol_pct")}
            </th>
          </tr>
        </thead>
        <tbody>
          {/* PRF-041 — rows rendered in backend order (most-recent first) */}
          {rows.map((row) => (
            <tr
              key={row.rowKey}
              id={`account-performance-row-${row.rowKey}`}
              data-testid={`account-performance-row-${row.rowKey}`}
            >
              <td className="m3-td">{row.month !== null ? t(row.periodLabel) : row.periodLabel}</td>
              <td
                data-testid={`account-performance-prev-value-${row.rowKey}`}
                className="m3-td text-right"
              >
                {row.previousValueFormatted}
              </td>
              <td
                data-testid={`account-performance-cash-flow-${row.rowKey}`}
                className={`m3-td text-right ${row.cashFlow.colorClass}`}
              >
                {row.cashFlow.formatted}
              </td>
              <td
                data-testid={`account-performance-asset-flow-${row.rowKey}`}
                className={`m3-td text-right ${row.assetFlow.colorClass}`}
              >
                {row.assetFlow.formatted}
              </td>
              <td
                data-testid={`account-performance-dividends-${row.rowKey}`}
                className="m3-td text-right"
              >
                {row.dividendsFormatted}
              </td>
              <td
                data-testid={`account-performance-pnl-${row.rowKey}`}
                className={`m3-td text-right ${row.pnl.colorClass}`}
              >
                {row.pnl.formatted}
              </td>
              <td className="m3-td text-right font-medium">{row.endValueFormatted}</td>
              <td
                data-testid={`account-performance-pop-value-${row.rowKey}`}
                className={`m3-td text-right ${row.periodOverPeriod.colorClass}`}
              >
                {row.periodOverPeriod.gainFormatted}
              </td>
              <td
                data-testid={`account-performance-pop-pct-${row.rowKey}`}
                className={`m3-td text-right ${row.periodOverPeriod.colorClass}`}
              >
                {row.periodOverPeriod.pctFormatted}
              </td>
              {showYtd && (
                <>
                  <td
                    data-testid={`account-performance-ytd-value-${row.rowKey}`}
                    className={`m3-td text-right ${row.yearToDate?.colorClass ?? ""}`}
                  >
                    {row.yearToDate?.gainFormatted ?? "—"}
                  </td>
                  <td
                    data-testid={`account-performance-ytd-pct-${row.rowKey}`}
                    className={`m3-td text-right ${row.yearToDate?.colorClass ?? ""}`}
                  >
                    {row.yearToDate?.pctFormatted ?? "—"}
                  </td>
                </>
              )}
              <td
                data-testid={`account-performance-since-value-${row.rowKey}`}
                className={`m3-td text-right ${row.sinceInception.colorClass}`}
              >
                {row.sinceInception.gainFormatted}
              </td>
              <td
                data-testid={`account-performance-since-pct-${row.rowKey}`}
                className={`m3-td text-right ${row.sinceInception.colorClass}`}
              >
                {row.sinceInception.pctFormatted}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
