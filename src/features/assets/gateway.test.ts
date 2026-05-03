import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AssetLookupResult, WebLookupCommandError } from "@/bindings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockInvoke = vi.mocked(invoke);

// Import after mock is registered so bindings.ts picks up the mock
const { assetGateway } = await import("./gateway");

describe("asset gateway — lookupAsset", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // WEB-020 — success path returns AssetLookupResult[]
  it("lookupAsset returns result list on success", async () => {
    const results: AssetLookupResult[] = [
      {
        name: "Apple Inc.",
        reference: "AAPL",
        currency: "USD",
        asset_class: "Stocks",
        exchange: null,
      },
      {
        name: "iShares Core S&P 500",
        reference: "IVV",
        currency: "USD",
        asset_class: "ETF",
        exchange: null,
      },
    ];
    // bindings.ts wraps the TAURI_INVOKE result in { status: "ok", data: ... }
    mockInvoke.mockResolvedValue(results);

    const res = await assetGateway.lookupAsset("AAPL");

    expect(res).toEqual({ status: "ok", data: results });
    expect(mockInvoke).toHaveBeenCalledWith("lookup_asset", { query: "AAPL" });
  });

  // WEB-020 — ISIN query (12 alphanumeric chars) is forwarded as-is
  it("lookupAsset forwards 12-char ISIN query verbatim", async () => {
    const results: AssetLookupResult[] = [
      {
        name: "Apple Inc.",
        reference: "US0378331005",
        currency: "USD",
        asset_class: "Stocks",
        exchange: null,
      },
    ];
    mockInvoke.mockResolvedValue(results);

    const res = await assetGateway.lookupAsset("US0378331005");

    expect(res).toEqual({ status: "ok", data: results });
    expect(mockInvoke).toHaveBeenCalledWith("lookup_asset", { query: "US0378331005" });
  });

  // WEB-020 — empty list is a valid success (WEB-032 handled by UI layer)
  it("lookupAsset returns empty list when no instruments found", async () => {
    mockInvoke.mockResolvedValue([]);

    const res = await assetGateway.lookupAsset("xyzzy-not-a-real-ticker");

    expect(res).toEqual({ status: "ok", data: [] });
    expect(mockInvoke).toHaveBeenCalledWith("lookup_asset", {
      query: "xyzzy-not-a-real-ticker",
    });
  });

  // WEB-025 — NetworkError is surfaced as { status: "error", error: { code: "NetworkError" } }
  it("lookupAsset returns NetworkError on network failure", async () => {
    const err: WebLookupCommandError = { code: "NetworkError" };
    // bindings.ts catches the rejection and returns { status: "error", error: e }
    mockInvoke.mockRejectedValue(err);

    const res = await assetGateway.lookupAsset("AAPL");

    expect(res).toEqual({ status: "error", error: err });
    expect(mockInvoke).toHaveBeenCalledWith("lookup_asset", { query: "AAPL" });
  });

  // WEB-023/WEB-024/WEB-046 — optional fields may be null
  it("lookupAsset preserves null optional fields from result", async () => {
    const results: AssetLookupResult[] = [
      { name: "Obscure Fund", reference: null, currency: null, asset_class: null, exchange: null },
    ];
    mockInvoke.mockResolvedValue(results);

    const res = await assetGateway.lookupAsset("obscure fund");

    expect(res).toEqual({ status: "ok", data: results });
  });
});
