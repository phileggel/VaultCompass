import { useCallback, useEffect, useState } from "react";
import type { CurrencyPairSummary, CurrencyRate } from "@/bindings";
import { logger } from "@/lib/logger";
import type { I18nMessage } from "@/ui/format/i18n";
import { getCurrencyPairs, getCurrencyRates, subscribeToEvents } from "../gateway";
import { currencyErrorToI18n } from "../shared/presenter";

interface SelectedPair {
  fromCurrency: string;
  toCurrency: string;
}

interface UseCurrencyRatesViewResult {
  isLoading: boolean;
  error: I18nMessage | null;
  pairs: CurrencyPairSummary[];
  selectedPair: SelectedPair | null;
  rates: CurrencyRate[];
  ratesError: I18nMessage | null;
  selectPair: (fromCurrency: string, toCurrency: string) => void;
  clearSelection: () => void;
  refetch: () => void;
}

/** FXR-050/051 — loads the followed pairs and, on drill-in, one pair's rate history. */
export function useCurrencyRatesView(): UseCurrencyRatesViewResult {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<I18nMessage | null>(null);
  const [pairs, setPairs] = useState<CurrencyPairSummary[]>([]);
  const [selectedPair, setSelectedPair] = useState<SelectedPair | null>(null);
  const [rates, setRates] = useState<CurrencyRate[]>([]);
  const [ratesError, setRatesError] = useState<I18nMessage | null>(null);

  const fetchPairs = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    const result = await getCurrencyPairs();
    if (result.status === "ok") {
      setPairs(result.data);
    } else {
      logger.error("[useCurrencyRatesView] fetchPairs failed", result.error);
      setError(currencyErrorToI18n(result.error));
    }
    setIsLoading(false);
  }, []);

  const fetchRates = useCallback(async (fromCurrency: string, toCurrency: string) => {
    setRatesError(null);
    const result = await getCurrencyRates(fromCurrency, toCurrency);
    if (result.status === "ok") {
      setRates(result.data);
    } else {
      logger.error("[useCurrencyRatesView] fetchRates failed", result.error);
      setRatesError(currencyErrorToI18n(result.error));
    }
  }, []);

  useEffect(() => {
    void fetchPairs();
  }, [fetchPairs]);

  // FXR-026/037 — re-fetch when a rate is recorded/updated/deleted elsewhere.
  useEffect(() => {
    const unlistenPromise = subscribeToEvents((type) => {
      if (type === "CurrencyRateUpdated") {
        void fetchPairs();
        setSelectedPair((current) => {
          if (current) void fetchRates(current.fromCurrency, current.toCurrency);
          return current;
        });
      }
    });
    return () => {
      void Promise.resolve(unlistenPromise).then((unlisten) => unlisten?.());
    };
  }, [fetchPairs, fetchRates]);

  const selectPair = useCallback(
    (fromCurrency: string, toCurrency: string) => {
      setSelectedPair({ fromCurrency, toCurrency });
      void fetchRates(fromCurrency, toCurrency);
    },
    [fetchRates],
  );

  const clearSelection = useCallback(() => {
    setSelectedPair(null);
    setRates([]);
    setRatesError(null);
  }, []);

  return {
    isLoading,
    error,
    pairs,
    selectedPair,
    rates,
    ratesError,
    selectPair,
    clearSelection,
    refetch: () => void fetchPairs(),
  };
}
