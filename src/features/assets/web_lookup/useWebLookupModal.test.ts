import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AssetLookupResult } from "@/bindings";
import { useWebLookupModal } from "./useWebLookupModal";

const mockSearchAssetWeb = vi.fn();

vi.mock("../gateway", () => ({
  assetGateway: {
    searchAssetWeb: (...args: unknown[]) => mockSearchAssetWeb(...args),
  },
}));

const appleResult: AssetLookupResult = {
  name: "Apple Inc.",
  reference: "AAPL",
  currency: "USD",
  asset_class: "Stocks",
};

const etfResult: AssetLookupResult = {
  name: "iShares Core S&P 500",
  reference: "IVV",
  currency: "USD",
  asset_class: "ETF",
};

describe("useWebLookupModal", () => {
  beforeEach(() => {
    mockSearchAssetWeb.mockReset();
  });

  // Initial state
  it("starts in the search step", () => {
    const { result } = renderHook(() => useWebLookupModal());
    expect(result.current.modalStep.step).toBe("search");
  });

  // WEB-040 — selecting a result transitions to form-prefilled
  it("selectResult transitions from search to form-prefilled with the selected result", async () => {
    mockSearchAssetWeb.mockResolvedValue({ status: "ok", data: [appleResult] });

    const { result } = renderHook(() => useWebLookupModal());

    act(() => {
      result.current.setQuery("AAPL");
    });

    await act(async () => {
      result.current.submitSearch();
    });

    act(() => {
      result.current.selectResult(appleResult);
    });

    expect(result.current.modalStep.step).toBe("form-prefilled");
    if (result.current.modalStep.step === "form-prefilled") {
      expect(result.current.modalStep.selection).toEqual(appleResult);
    }
  });

  // WEB-013 — fillManually transitions to form-manual (no gateway call needed)
  it("fillManually transitions from search to form-manual", () => {
    const { result } = renderHook(() => useWebLookupModal());

    act(() => {
      result.current.fillManually();
    });

    expect(result.current.modalStep.step).toBe("form-manual");
  });

  // WEB-047 — back from form-prefilled restores search state (query + results retained)
  it("back from form-prefilled returns to search step with previous results retained", async () => {
    mockSearchAssetWeb.mockResolvedValue({ status: "ok", data: [appleResult, etfResult] });

    const { result } = renderHook(() => useWebLookupModal());

    act(() => {
      result.current.setQuery("apple");
    });

    await act(async () => {
      result.current.submitSearch();
    });

    act(() => {
      result.current.selectResult(appleResult);
    });

    expect(result.current.modalStep.step).toBe("form-prefilled");

    act(() => {
      result.current.back();
    });

    expect(result.current.modalStep.step).toBe("search");
    // Previous query is retained — no need to retype (WEB-047)
    expect(result.current.query).toBe("apple");
    // Previous results are retained in state
    expect(result.current.searchState.status).toBe("results");
    if (result.current.searchState.status === "results") {
      expect(result.current.searchState.results).toEqual([appleResult, etfResult]);
    }
  });

  // WEB-013 — back is NOT available from form-manual
  it("canGoBack is false when in form-manual step", () => {
    const { result } = renderHook(() => useWebLookupModal());

    act(() => {
      result.current.fillManually();
    });

    expect(result.current.modalStep.step).toBe("form-manual");
    expect(result.current.canGoBack).toBe(false);
  });

  // WEB-047 — back IS available from form-prefilled
  it("canGoBack is true when in form-prefilled step", async () => {
    mockSearchAssetWeb.mockResolvedValue({ status: "ok", data: [appleResult] });

    const { result } = renderHook(() => useWebLookupModal());

    act(() => {
      result.current.setQuery("AAPL");
    });

    await act(async () => {
      result.current.submitSearch();
    });

    act(() => {
      result.current.selectResult(appleResult);
    });

    expect(result.current.canGoBack).toBe(true);
  });

  // WEB-040 — selecting a different result replaces the previous selection
  it("selecting a different result replaces all pre-filled values", async () => {
    mockSearchAssetWeb.mockResolvedValue({ status: "ok", data: [appleResult, etfResult] });

    const { result } = renderHook(() => useWebLookupModal());

    act(() => {
      result.current.setQuery("apple");
    });

    await act(async () => {
      result.current.submitSearch();
    });

    act(() => {
      result.current.selectResult(appleResult);
    });

    // Go back and select a different result
    act(() => {
      result.current.back();
    });

    act(() => {
      result.current.selectResult(etfResult);
    });

    expect(result.current.modalStep.step).toBe("form-prefilled");
    if (result.current.modalStep.step === "form-prefilled") {
      expect(result.current.modalStep.selection).toEqual(etfResult);
    }
  });
});
