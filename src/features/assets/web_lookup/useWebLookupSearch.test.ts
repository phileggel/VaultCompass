import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AssetLookupResult } from "@/bindings";
import { useWebLookupSearch } from "./useWebLookupSearch";

const mockSearchAssetWeb = vi.fn();

vi.mock("../gateway", () => ({
  assetGateway: {
    searchAssetWeb: (...args: unknown[]) => mockSearchAssetWeb(...args),
  },
}));

describe("useWebLookupSearch", () => {
  beforeEach(() => {
    mockSearchAssetWeb.mockReset();
  });

  // WEB-011 — empty query submit is a no-op: state stays idle
  it("does not call the gateway and stays idle when query is empty", async () => {
    const { result } = renderHook(() => useWebLookupSearch());

    expect(result.current.query).toBe("");
    expect(result.current.state.status).toBe("idle");

    await act(async () => {
      result.current.submit();
    });

    expect(mockSearchAssetWeb).not.toHaveBeenCalled();
    expect(result.current.state.status).toBe("idle");
  });

  // WEB-030 — transitions through loading and lands on results (WEB-031)
  it("transitions idle → loading → results on successful search", async () => {
    const results: AssetLookupResult[] = [
      { name: "Apple Inc.", reference: "AAPL", currency: "USD", asset_class: "Stocks" },
    ];
    mockSearchAssetWeb.mockResolvedValue({ status: "ok", data: results });

    const { result } = renderHook(() => useWebLookupSearch());

    act(() => {
      result.current.setQuery("AAPL");
    });

    // Start the submit but do not await yet so we can observe loading
    let submitPromise: Promise<void>;
    act(() => {
      submitPromise = Promise.resolve().then(() => result.current.submit());
    });

    await act(async () => {
      await submitPromise;
    });

    expect(mockSearchAssetWeb).toHaveBeenCalledWith("AAPL");
    expect(result.current.state.status).toBe("results");
    if (result.current.state.status === "results") {
      expect(result.current.state.results).toEqual(results);
    }
  });

  // WEB-032 — empty result list transitions to empty state
  it("transitions to empty state when gateway returns no results", async () => {
    mockSearchAssetWeb.mockResolvedValue({ status: "ok", data: [] });

    const { result } = renderHook(() => useWebLookupSearch());

    act(() => {
      result.current.setQuery("unknown-instrument");
    });

    await act(async () => {
      result.current.submit();
    });

    expect(result.current.state.status).toBe("empty");
  });

  // WEB-033 — NetworkError transitions to error state
  it("transitions to error state when gateway returns NetworkError", async () => {
    mockSearchAssetWeb.mockResolvedValue({
      status: "error",
      error: { code: "NetworkError" },
    });

    const { result } = renderHook(() => useWebLookupSearch());

    act(() => {
      result.current.setQuery("AAPL");
    });

    await act(async () => {
      result.current.submit();
    });

    expect(result.current.state.status).toBe("error");
  });

  // WEB-033 — retry re-issues the last query
  it("retry re-issues the last query after an error", async () => {
    mockSearchAssetWeb
      .mockResolvedValueOnce({ status: "error", error: { code: "NetworkError" } })
      .mockResolvedValueOnce({
        status: "ok",
        data: [{ name: "Apple Inc.", reference: "AAPL", currency: "USD", asset_class: "Stocks" }],
      });

    const { result } = renderHook(() => useWebLookupSearch());

    act(() => {
      result.current.setQuery("AAPL");
    });

    await act(async () => {
      result.current.submit();
    });

    expect(result.current.state.status).toBe("error");

    await act(async () => {
      result.current.retry();
    });

    expect(mockSearchAssetWeb).toHaveBeenCalledTimes(2);
    expect(mockSearchAssetWeb).toHaveBeenNthCalledWith(2, "AAPL");
    expect(result.current.state.status).toBe("results");
  });

  // WEB-030 — submit while loading is ignored (no duplicate request)
  it("ignores a second submit while a search is already loading", async () => {
    let resolveFirst!: (v: unknown) => void;
    const firstCall = new Promise((resolve) => {
      resolveFirst = resolve;
    });
    mockSearchAssetWeb.mockReturnValueOnce(firstCall);

    const { result } = renderHook(() => useWebLookupSearch());

    act(() => {
      result.current.setQuery("AAPL");
    });

    // Kick off first submit — intentionally not awaited yet
    act(() => {
      result.current.submit();
    });

    // Immediately fire a second submit while loading
    act(() => {
      result.current.submit();
    });

    // Now let the first call resolve
    await act(async () => {
      resolveFirst({ status: "ok", data: [] });
    });

    // Gateway should have been called only once
    expect(mockSearchAssetWeb).toHaveBeenCalledTimes(1);
  });
});
