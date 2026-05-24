import { useCallback, useState } from "react";
import type { AssetLookupResult, LookupMode, WebLookupApplicationError } from "@/bindings";
import { assetGateway } from "../gateway";

export type WebLookupSearchState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "results"; results: AssetLookupResult[] }
  | { status: "empty" }
  | { status: "error"; code: WebLookupApplicationError["code"] };

export interface UseWebLookupSearchReturn {
  query: string;
  setQuery: (q: string) => void;
  state: WebLookupSearchState;
  submit: () => void;
  retry: () => void;
}

export function useWebLookupSearch(): UseWebLookupSearchReturn {
  const [query, setQuery] = useState("");
  const [state, setState] = useState<WebLookupSearchState>({ status: "idle" });

  const runSearch = useCallback(async (q: string, mode: LookupMode) => {
    setState({ status: "loading" });
    const result = await assetGateway.lookupAsset(q, mode);
    if (result.status === "error") {
      setState({ status: "error", code: result.error.code });
    } else if (result.data.length === 0) {
      setState({ status: "empty" });
    } else {
      setState({ status: "results", results: result.data });
    }
  }, []);

  const submit = useCallback(() => {
    if (!query.trim() || state.status === "loading") return;
    const mode: LookupMode =
      query.trim().length === 12 && /^[A-Za-z0-9]+$/.test(query.trim()) ? "Isin" : "Keyword";
    runSearch(query, mode);
  }, [query, state, runSearch]);

  const retry = useCallback(() => {
    if (!query.trim() || state.status === "loading") return;
    const mode: LookupMode =
      query.trim().length === 12 && /^[A-Za-z0-9]+$/.test(query.trim()) ? "Isin" : "Keyword";
    runSearch(query, mode);
  }, [query, state, runSearch]);

  return { query, setQuery, state, submit, retry };
}
