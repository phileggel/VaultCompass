import { useNavigate } from "@tanstack/react-router";
import { Minus, Plus, Search } from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "@/lib/store";
import { IconButton } from "@/ui/components/button/IconButton";
import type { HoldingRowViewModel } from "../shared/presenter";
import type { ModalTarget, SellTarget } from "../shared/types";

type HoldingRowProps = {
  row: HoldingRowViewModel;
  accountId: string;
  onBuy: (target: ModalTarget) => void;
  onSell: (target: SellTarget) => void;
};

function PnlCell({ value, raw }: { value: string; raw: number }) {
  const { t } = useTranslation();
  const colorClass =
    raw > 0 ? "text-m3-success" : raw < 0 ? "text-m3-error" : "text-m3-on-surface-variant";
  return (
    <span className={`tabular-nums ${colorClass}`}>
      {raw === 0 ? t("account_details.pnl_placeholder") : value}
    </span>
  );
}

export function HoldingRow({ row, accountId, onBuy, onSell }: HoldingRowProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const assets = useAppStore((state) => state.assets);
  const accounts = useAppStore((state) => state.accounts);

  const buildTarget = useCallback((): ModalTarget => {
    const asset = assets.find((a) => a.id === row.assetId);
    const account = accounts.find((a) => a.id === accountId);
    return {
      accountName: account?.name ?? accountId,
      assetId: row.assetId,
      assetName: row.assetName,
      assetCurrency: asset?.currency ?? "",
      showExchangeRate: asset && account ? asset.currency !== account.currency : false,
    };
  }, [assets, accounts, accountId, row]);

  const handleBuy = useCallback(() => {
    onBuy(buildTarget());
  }, [onBuy, buildTarget]);

  const handleSell = useCallback(() => {
    onSell({ ...buildTarget(), holdingQuantityMicro: row.quantityMicro });
  }, [onSell, buildTarget, row.quantityMicro]);

  const handleViewTransactions = useCallback(() => {
    navigate({
      to: "/accounts/$accountId/transactions/$assetId",
      params: { accountId, assetId: row.assetId },
      search: { pendingTransactionAssetId: undefined },
    });
  }, [navigate, accountId, row.assetId]);

  const asset = assets.find((a) => a.id === row.assetId);
  const isArchived = asset?.is_archived ?? false;

  return (
    <tr className="m3-tr">
      <td className="m3-td">
        <div className="flex flex-col">
          <span className="font-medium text-m3-on-surface">{row.assetName}</span>
          <span className="text-xs text-m3-on-surface-variant">{row.assetReference}</span>
        </div>
      </td>
      <td className="m3-td text-right tabular-nums">{row.quantity}</td>
      <td className="m3-td text-right tabular-nums">{row.averagePrice}</td>
      <td className="m3-td text-right tabular-nums font-medium">{row.costBasis}</td>
      <td className="m3-td text-right">
        <PnlCell value={row.realizedPnl} raw={row.realizedPnlRaw} />
      </td>
      <td className="m3-td">
        <div className="flex items-center gap-1">
          {/* TRX-041 — Buy modal from holding row */}
          <IconButton
            icon={<Plus size={16} />}
            variant="success"
            size="sm"
            aria-label={t("transaction.action_buy")}
            onClick={handleBuy}
          />
          {/* SEL-010 — Sell button; disabled when asset is archived (SEL-037) */}
          <IconButton
            icon={<Minus size={16} />}
            variant="error"
            size="sm"
            aria-label={t("transaction.action_sell")}
            onClick={handleSell}
            disabled={isArchived}
          />
          <IconButton
            icon={<Search size={16} />}
            size="sm"
            aria-label={t("transaction.list_title")}
            onClick={handleViewTransactions}
          />
        </div>
      </td>
    </tr>
  );
}
