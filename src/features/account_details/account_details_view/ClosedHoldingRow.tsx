import { useNavigate } from "@tanstack/react-router";
import { Search } from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { IconButton } from "@/ui/components/button/IconButton";
import { formatIsoDate } from "../shared/formatDate";
import { PnlCell } from "../shared/PnlCell";
import type { ClosedHoldingRowViewModel } from "../shared/presenter";

type ClosedHoldingRowProps = {
  row: ClosedHoldingRowViewModel;
  accountId: string;
};

export function ClosedHoldingRow({ row, accountId }: ClosedHoldingRowProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const handleViewTransactions = useCallback(() => {
    navigate({
      to: "/accounts/$accountId/transactions/$assetId",
      params: { accountId, assetId: row.assetId },
      search: { pendingTransactionAssetId: undefined },
    });
  }, [navigate, accountId, row.assetId]);

  return (
    <tr className="m3-tr opacity-70">
      <td className="m3-td">
        <div className="flex flex-col">
          <span className="font-medium text-m3-on-surface">{row.assetName}</span>
          <span className="text-xs text-m3-on-surface-variant">{row.assetReference}</span>
        </div>
      </td>
      <td className="m3-td text-right">
        <PnlCell value={row.realizedPnl} raw={row.realizedPnlRaw} />
      </td>
      <td className="m3-td text-right text-m3-on-surface-variant">
        {formatIsoDate(row.lastSoldDate)}
      </td>
      {/* ACD-049 — inspect action only; Buy/Sell omitted for closed positions */}
      <td className="m3-td">
        <IconButton
          icon={<Search size={16} />}
          size="sm"
          aria-label={t("transaction.list_title")}
          onClick={handleViewTransactions}
        />
      </td>
    </tr>
  );
}
