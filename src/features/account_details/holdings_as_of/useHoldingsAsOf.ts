import { useEffect, useMemo, useState } from "react";
import type { I18nMessage } from "@/ui/format/i18n";
import { accountDetailsGateway } from "../gateway";
import {
  type HoldingAsOfRowViewModel,
  holdingsAsOfErrorToI18n,
  toHoldingAsOfRow,
  toHoldingsAsOfTotals,
} from "./presenter";

/** Local calendar date as ISO `YYYY-MM-DD` — the default as-of date. */
function todayIso(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

interface UseHoldingsAsOfResult {
  /** Current as-of date ("YYYY-MM-DD"). */
  date: string;
  /** Commits a new as-of date and triggers a re-fetch. */
  setDate: (date: string) => void;
  /** Formatted holding rows on the date; empty until the first load resolves. */
  rows: HoldingAsOfRowViewModel[];
  /** Formatted total cost basis in account currency. */
  totalCostBasis: string;
  /** Formatted total market value in account currency. */
  totalMarketValue: string;
  /** ISO 4217 currency code of the account; "" until loaded. */
  accountCurrency: string;
  isLoading: boolean;
  /** Mapped error message, or null. */
  error: I18nMessage | null;
}

/**
 * Fetches the account's holdings as of `date` (defaulting to today), re-fetching
 * whenever the committed date changes. Returns display-ready rows + totals and a
 * typed error mapped through the F27 presenter.
 */
export function useHoldingsAsOf(accountId: string): UseHoldingsAsOfResult {
  const [date, setDate] = useState<string>(todayIso);
  const [rows, setRows] = useState<HoldingAsOfRowViewModel[]>([]);
  const [totalCostBasis, setTotalCostBasis] = useState("");
  const [totalMarketValue, setTotalMarketValue] = useState("");
  const [accountCurrency, setAccountCurrency] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<I18nMessage | null>(null);

  const effectiveDate = useMemo(() => date || todayIso(), [date]);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    // Clear any prior error so the loading state shows on every re-fetch.
    setError(null);
    accountDetailsGateway.getAccountHoldingsAsOf(accountId, effectiveDate).then((res) => {
      if (cancelled) return;
      if (res.status === "ok") {
        const totals = toHoldingsAsOfTotals(res.data);
        setRows(res.data.holdings.map(toHoldingAsOfRow));
        setTotalCostBasis(totals.totalCostBasis);
        setTotalMarketValue(totals.totalMarketValue);
        setAccountCurrency(res.data.account_currency);
      } else {
        setRows([]);
        setError(holdingsAsOfErrorToI18n(res.error));
      }
      setIsLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [accountId, effectiveDate]);

  return {
    date,
    setDate,
    rows,
    totalCostBasis,
    totalMarketValue,
    accountCurrency,
    isLoading,
    error,
  };
}
