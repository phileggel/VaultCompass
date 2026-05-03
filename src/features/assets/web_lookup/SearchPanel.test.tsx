import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { AssetLookupResult } from "@/bindings";
import { SearchPanel } from "./SearchPanel";
import type { WebLookupSearchState } from "./useWebLookupSearch";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string, opts?: Record<string, string>) => opts?.name ?? key }),
}));

const noop = () => {};

function renderPanel(state: WebLookupSearchState, onSelect = noop, query = "") {
  return render(
    <SearchPanel
      query={query}
      setQuery={noop}
      state={state}
      submit={noop}
      retry={noop}
      onSelect={onSelect}
      onFillManually={noop}
    />,
  );
}

// ---------------------------------------------------------------------------
// Status states
// ---------------------------------------------------------------------------

describe("SearchPanel — status states", () => {
  // WEB-011 — idle hint shown when no search has been made
  it("shows idle hint in idle state", () => {
    renderPanel({ status: "idle" });
    expect(screen.getByText("asset.web_lookup.idle_hint")).toBeTruthy();
  });

  // WEB-030 — loading indicator shown while search is in progress
  it("shows loading indicator in loading state", () => {
    renderPanel({ status: "loading" });
    expect(screen.getByText("asset.web_lookup.loading")).toBeTruthy();
  });

  // WEB-032 — empty state shown when no results returned
  it("shows no-results message in empty state", () => {
    renderPanel({ status: "empty" });
    expect(screen.getByText("asset.web_lookup.no_results")).toBeTruthy();
  });

  // WEB-033 — error state shows inline error message
  it("shows error message in error state", () => {
    renderPanel({ status: "error" });
    expect(screen.getByRole("alert")).toBeTruthy();
    expect(screen.getByText("asset.web_lookup.error_network")).toBeTruthy();
  });

  // WEB-011 — search button disabled when query is empty
  it("disables search button when query is empty", () => {
    renderPanel({ status: "idle" }, noop, "");
    const btn = screen.getByRole("button", { name: "asset.web_lookup.action_search" });
    expect(btn).toBeDisabled();
  });

  // WEB-011 — search button enabled when query is non-empty
  it("enables search button when query is non-empty", () => {
    renderPanel({ status: "idle" }, noop, "AAPL");
    const btn = screen.getByRole("button", { name: "asset.web_lookup.action_search" });
    expect(btn).not.toBeDisabled();
  });
});

// ---------------------------------------------------------------------------
// Results row layout (WEB-031)
// ---------------------------------------------------------------------------

describe("SearchPanel — result row layout (WEB-031)", () => {
  const stockResult: AssetLookupResult = {
    name: "Apple Inc.",
    reference: "AAPL",
    currency: "USD",
    asset_class: "Stocks",
    exchange: "NYSE",
  };

  // WEB-031 — first line shows reference code and instrument name
  it("shows reference code and name on the first line", () => {
    renderPanel({ status: "results", results: [stockResult] });
    expect(screen.getByText("AAPL")).toBeTruthy();
    expect(screen.getByText("Apple Inc.")).toBeTruthy();
  });

  // WEB-031 — second line shows formatted class label and exchange separated by ·
  it("shows class label · exchange on the second line when both present", () => {
    renderPanel({ status: "results", results: [stockResult] });
    expect(screen.getByText("Stocks · NYSE")).toBeTruthy();
  });

  // WEB-031 — multi-word class names are human-readable on the second line
  it("renders multi-word class names as readable labels", () => {
    const fund: AssetLookupResult = {
      name: "Vanguard 500",
      reference: "VFIAX",
      currency: "USD",
      asset_class: "MutualFunds",
      exchange: null,
    };
    renderPanel({ status: "results", results: [fund] });
    expect(screen.getByText("Mutual Funds")).toBeTruthy();
  });

  // WEB-031 — when exchange absent, second line shows type label only (no separator)
  it("shows type label only when exchange is absent", () => {
    const noExchange: AssetLookupResult = {
      name: "iShares Core ETF",
      reference: "IVV",
      currency: "USD",
      asset_class: "ETF",
      exchange: null,
    };
    renderPanel({ status: "results", results: [noExchange] });
    expect(screen.getByText("ETF")).toBeTruthy();
    expect(screen.queryByText(/·/)).toBeNull();
  });

  // WEB-031 — unknown type fallback shown when asset_class is null
  it("shows type_unknown fallback when asset_class is absent", () => {
    const unknown: AssetLookupResult = {
      name: "Mystery Instrument",
      reference: null,
      currency: null,
      asset_class: null,
      exchange: null,
    };
    renderPanel({ status: "results", results: [unknown] });
    expect(screen.getByText("asset.web_lookup.type_unknown")).toBeTruthy();
  });

  // WEB-046 — reference prefix omitted when reference is absent
  it("omits the reference prefix when reference is null", () => {
    const noRef: AssetLookupResult = {
      name: "No Reference Fund",
      reference: null,
      currency: "EUR",
      asset_class: "MutualFunds",
      exchange: null,
    };
    const { container } = renderPanel({ status: "results", results: [noRef] });
    // The mono-span for the reference code should not exist
    const monoSpans = container.querySelectorAll("span.font-mono");
    expect(monoSpans.length).toBe(0);
  });

  // WEB-040 — clicking a result calls onSelect with the result
  it("calls onSelect with the result when a row is clicked", () => {
    const onSelect = vi.fn();
    renderPanel({ status: "results", results: [stockResult] }, onSelect);
    fireEvent.click(screen.getByRole("button", { name: "Apple Inc." }));
    expect(onSelect).toHaveBeenCalledWith(stockResult);
  });
});

// ---------------------------------------------------------------------------
// WEB-013 — Fill manually bypass
// ---------------------------------------------------------------------------

describe("SearchPanel — fill manually bypass (WEB-013)", () => {
  it("fill manually button is always visible", () => {
    renderPanel({ status: "idle" });
    expect(
      screen.getByRole("button", { name: "asset.web_lookup.action_fill_manually" }),
    ).toBeTruthy();
  });

  it("fill manually button is visible even in error state", () => {
    renderPanel({ status: "error" });
    expect(
      screen.getByRole("button", { name: "asset.web_lookup.action_fill_manually" }),
    ).toBeTruthy();
  });
});
