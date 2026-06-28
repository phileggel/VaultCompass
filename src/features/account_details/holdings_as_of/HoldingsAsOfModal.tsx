import { useTranslation } from "react-i18next";
import { formatIsoDate } from "@/features/account_details/shared/formatDate";
import { Button } from "@/ui/components/button/Button";
import { DateField } from "@/ui/components/field/DateField";
import { FormModal } from "@/ui/components/modal/FormModal";
import { useHoldingsAsOf } from "./useHoldingsAsOf";

interface HoldingsAsOfModalProps {
  isOpen: boolean;
  onClose: () => void;
  accountId: string;
}

/**
 * Read-only modal: shows the account's holdings reconstructed as they stood on a
 * user-picked past date (quantity, average cost, price + value at the date).
 */
export function HoldingsAsOfModal({ isOpen, onClose, accountId }: HoldingsAsOfModalProps) {
  const { t, i18n } = useTranslation();
  const view = useHoldingsAsOf(accountId);

  const footer = (
    <div className="flex items-center justify-end">
      <Button id="holdings-as-of-close" variant="secondary" onClick={onClose}>
        {t("action.close")}
      </Button>
    </div>
  );

  return (
    <FormModal
      isOpen={isOpen}
      onClose={onClose}
      title={t("holdings_as_of.title")}
      footer={footer}
      maxWidth="max-w-3xl"
    >
      <div className="flex flex-col gap-4">
        <DateField
          id="holdings-as-of-date"
          data-testid="holdings-as-of-date"
          label={t("holdings_as_of.as_of_label")}
          value={view.date}
          onChange={(e) => view.setDate(e.target.value)}
        />

        {view.error ? (
          <p role="alert" className="text-sm text-m3-error">
            {t(view.error.key, view.error.vars)}
          </p>
        ) : view.isLoading ? (
          <div className="animate-pulse space-y-2" data-testid="holdings-as-of-loading">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-8 bg-m3-surface-variant rounded-lg" />
            ))}
          </div>
        ) : view.rows.length === 0 ? (
          <p className="text-sm text-m3-on-surface-variant italic py-6 text-center">
            {t("holdings_as_of.empty")}
          </p>
        ) : (
          <>
            <table className="w-full border-collapse" id="holdings-as-of-table">
              <thead>
                <tr>
                  <th className="m3-th">{t("holdings_as_of.column_asset")}</th>
                  <th className="m3-th text-right">{t("holdings_as_of.column_quantity")}</th>
                  <th className="m3-th text-right">{t("holdings_as_of.column_avg_cost")}</th>
                  <th className="m3-th text-right">{t("holdings_as_of.column_price")}</th>
                  <th className="m3-th text-right">{t("holdings_as_of.column_market_value")}</th>
                  <th className="m3-th text-right">{t("holdings_as_of.column_unrealized_pnl")}</th>
                </tr>
              </thead>
              <tbody>
                {view.rows.map((row) => (
                  <tr key={row.assetId} id={`holdings-as-of-row-${row.assetId}`}>
                    <td className="m3-td">{row.assetName}</td>
                    <td className="m3-td text-right">{row.quantity}</td>
                    <td className="m3-td text-right">{row.averageCost}</td>
                    <td className="m3-td text-right">
                      {row.price}
                      {row.priceDate && (
                        <span className="block text-xs text-m3-on-surface-variant">
                          {formatIsoDate(row.priceDate, i18n.language)}
                        </span>
                      )}
                    </td>
                    <td className="m3-td text-right">{row.marketValue}</td>
                    <td
                      className={`m3-td text-right ${
                        row.unrealizedPnlRaw !== null && row.unrealizedPnlRaw !== 0
                          ? row.unrealizedPnlRaw < 0
                            ? "text-m3-error"
                            : "text-m3-success"
                          : ""
                      }`}
                    >
                      {row.unrealizedPnl}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>

            <div className="flex justify-end gap-6 text-sm text-m3-on-surface-variant">
              <p>
                {t("holdings_as_of.total_cost_basis")}:{" "}
                <span className="font-semibold text-m3-on-surface">{view.totalCostBasis}</span>
              </p>
              <p>
                {t("holdings_as_of.total_market_value")}:{" "}
                <span className="font-semibold text-m3-on-surface">{view.totalMarketValue}</span>
              </p>
            </div>
          </>
        )}
      </div>
    </FormModal>
  );
}
