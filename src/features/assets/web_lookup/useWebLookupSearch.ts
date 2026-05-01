import { useCallback, useState } from "react";
import type { AssetLookupResult } from "@/bindings";
import { assetGateway } from "../gateway";

export type WebLookupSearchState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "results"; results: AssetLookupResult[] }
  | { status: "empty" }
  | { status: "error" };

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

  const runSearch = useCallback(async (q: string) => {
    setState({ status: "loading" });
    const result = await assetGateway.searchAssetWeb(q);
    if (result.status === "error") {
      setState({ status: "error" });
    } else if (result.data.length === 0) {
      setState({ status: "empty" });
    } else {
      setState({ status: "results", results: result.data });
    }
  }, []);

  const submit = useCallback(() => {
    if (!query.trim() || state.status === "loading") return;
    runSearch(query);
  }, [query, state, runSearch]);

  const retry = useCallback(() => {
    if (!query.trim() || state.status === "loading") return;
    runSearch(query);
  }, [query, state, runSearch]);

  return { query, setQuery, state, submit, retry };
}
