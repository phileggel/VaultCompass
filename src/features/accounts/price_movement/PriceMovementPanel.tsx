import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { PriceMovementReport } from "@/bindings";
import {
  formatPriceMovementPct,
  formatPriceMovementValue,
  priceMovementDateLabel,
} from "../shared/presenter";

interface PriceMovementPanelProps {
  report: PriceMovementReport;
  onDismiss: () => void;
}

/**
 * PMV-024 — the project's financial polarity: gain and loss carry their own
 * intent tokens rather than overloading success/error (see `PnlCell`). A null
 * movement is neither, so it stays neutral.
 */
function movementColour(microPercent: number | null): string {
  if (microPercent === null || microPercent === 0) return "text-m3-on-surface-variant";
  return microPercent > 0 ? "text-m3-gain" : "text-m3-loss";
}

/**
 * PMV-013/017/030–034/043/050–052/060 — what the last Global refresh did to each
 * account's value, as a dismissible panel above the accounts list.
 *
 * Deliberately not a modal: it never blocks, so the unupdated-prices modal
 * (MKT-172) behaves exactly as it did before. Every judgement it displays was
 * made by the backend — whether a proportion exists, the row order, what counts
 * as incomplete — so this renders and never derives.
 */
export function PriceMovementPanel({ report, onDismiss }: PriceMovementPanelProps) {
  const { t } = useTranslation();
  const dateLabel = priceMovementDateLabel(report.observed_from, report.observed_to);
  // PMV-060 — decided by the values alone, never by the dates.
  const nothingMoved = report.rows.every((row) => row.before === row.after);

  return (
    <section
      id="price-movement-panel"
      className="flex flex-col gap-3 rounded-[20px] bg-m3-surface-container-lowest p-5 shadow-elevation-2"
    >
      <div className="flex items-start justify-between gap-4">
        <div>
          <h3 className="font-headline text-base font-medium text-m3-on-surface">
            {t("pmv.title")}
          </h3>
          <p className="mt-0.5 text-xs text-m3-on-surface-variant">{t("pmv.subtitle_frozen")}</p>
        </div>
        <button
          id="price-movement-dismiss"
          type="button"
          onClick={onDismiss}
          aria-label={t("pmv.dismiss")}
          className="-mr-2 -mt-2 rounded-full p-2 text-m3-on-surface-variant transition-colors hover:bg-m3-on-surface/5"
        >
          <X size={18} />
        </button>
      </div>

      {nothingMoved ? (
        <p className="text-sm text-m3-on-surface">
          {report.incomplete ? t("pmv.nothing_moved_incomplete") : t("pmv.nothing_moved")}
        </p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-m3-surface-variant text-xs text-m3-on-surface-variant">
                <th className="py-2 text-left font-medium">{t("pmv.column_account")}</th>
                <th className="py-2 text-right font-medium" colSpan={2}>
                  {dateLabel === null ? t("pmv.column_values") : t(dateLabel.key, dateLabel.vars)}
                </th>
                <th className="py-2 text-right font-medium">{t("pmv.column_change")}</th>
              </tr>
            </thead>
            <tbody>
              {report.rows.map((row) => (
                <tr
                  key={row.account_id}
                  id={`price-movement-row-${row.account_id}`}
                  className="border-b border-m3-surface-variant/40"
                >
                  <td className="py-2.5 text-left">
                    <span className="text-m3-on-surface">{row.name}</span>
                    {row.incomplete && (
                      <span className="ml-2 text-xs text-m3-on-surface-variant">
                        {t("pmv.incomplete_marker")}
                      </span>
                    )}
                  </td>
                  <td className="py-2.5 text-right tabular-nums text-m3-on-surface-variant">
                    {`${formatPriceMovementValue(row.before)} ${row.currency}`}
                  </td>
                  <td className="py-2.5 text-right font-medium tabular-nums text-m3-on-surface">
                    {`${formatPriceMovementValue(row.after)} ${row.currency}`}
                  </td>
                  <td
                    className={`py-2.5 text-right font-medium tabular-nums ${movementColour(row.movement_pct)}`}
                  >
                    {formatPriceMovementPct(row.movement_pct)}
                  </td>
                </tr>
              ))}
              <tr id="price-movement-total" className="border-t border-m3-surface-variant">
                <td className="py-2.5 text-left font-semibold text-m3-on-surface">
                  {t("pmv.total")}
                  {report.incomplete && (
                    <span className="ml-2 text-xs font-normal text-m3-on-surface-variant">
                      {t("pmv.incomplete_marker")}
                    </span>
                  )}
                </td>
                <td className="py-2.5 text-right tabular-nums text-m3-on-surface-variant">
                  {`${formatPriceMovementValue(report.total_before)} ${report.total_currency}`}
                </td>
                <td className="py-2.5 text-right font-semibold tabular-nums text-m3-on-surface">
                  {`${formatPriceMovementValue(report.total_after)} ${report.total_currency}`}
                </td>
                <td
                  className={`py-2.5 text-right font-semibold tabular-nums ${movementColour(report.total_movement_pct)}`}
                >
                  {formatPriceMovementPct(report.total_movement_pct)}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
