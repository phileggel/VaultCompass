import { useCallback, useState } from "react";
import type { AssetLookupResult } from "@/bindings";
import type { WebLookupSearchState } from "./useWebLookupSearch";
import { useWebLookupSearch } from "./useWebLookupSearch";

export type ModalStep =
  | { step: "search" }
  | { step: "form-prefilled"; selection: AssetLookupResult }
  | { step: "form-manual" };

export interface UseWebLookupModalReturn {
  modalStep: ModalStep;
  searchState: WebLookupSearchState;
  query: string;
  setQuery: (q: string) => void;
  submitSearch: () => void;
  retrySearch: () => void;
  selectResult: (result: AssetLookupResult) => void;
  fillManually: () => void;
  back: () => void;
  canGoBack: boolean;
}

export function useWebLookupModal(): UseWebLookupModalReturn {
  const search = useWebLookupSearch();
  const [modalStep, setModalStep] = useState<ModalStep>({ step: "search" });

  const selectResult = useCallback((result: AssetLookupResult) => {
    setModalStep({ step: "form-prefilled", selection: result });
  }, []);

  const fillManually = useCallback(() => {
    setModalStep({ step: "form-manual" });
  }, []);

  const back = useCallback(() => {
    setModalStep({ step: "search" });
  }, []);

  const canGoBack = modalStep.step === "form-prefilled";

  return {
    modalStep,
    searchState: search.state,
    query: search.query,
    setQuery: search.setQuery,
    submitSearch: search.submit,
    retrySearch: search.retry,
    selectResult,
    fillManually,
    back,
    canGoBack,
  };
}
