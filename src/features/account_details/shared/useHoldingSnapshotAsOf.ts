import { useEffect, useMemo, useState } from "react";
import type { HoldingSnapshot } from "@/bindings";
import { accountDetailsGateway } from "../gateway";

/** Local calendar date as ISO `YYYY-MM-DD` — the trade-dialog default when no date is entered. */
function todayIso(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

/**
 * TDI-010 — fetches the (account, asset) holding snapshot (quantity + VWAP
 * average cost) as of `date`, defaulting to today when `date` is empty (TDI-020).
 * Returns null until the first load resolves or when the query fails (the caller
 * then shows nothing).
 */
export function useHoldingSnapshotAsOf(
  accountId: string,
  assetId: string,
  date: string,
): HoldingSnapshot | null {
  const [snapshot, setSnapshot] = useState<HoldingSnapshot | null>(null);
  const effectiveDate = useMemo(() => date || todayIso(), [date]);

  useEffect(() => {
    let cancelled = false;
    accountDetailsGateway.getHoldingSnapshotAsOf(accountId, assetId, effectiveDate).then((res) => {
      if (!cancelled) setSnapshot(res.status === "ok" ? res.data : null);
    });
    return () => {
      cancelled = true;
    };
  }, [accountId, assetId, effectiveDate]);

  return snapshot;
}
