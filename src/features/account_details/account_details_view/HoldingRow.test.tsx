import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Asset } from "@/bindings";
import { useAppStore } from "@/lib/store";
import type { HoldingRowViewModel } from "../shared/presenter";
import { HoldingRow } from "./HoldingRow";

const navigateMock = vi.fn();
vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => navigateMock,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "en-US" },
  }),
}));

const baseRow: HoldingRowViewModel = {
  assetId: "asset-1",
  assetName: "Apple Inc",
  assetReference: "AAPL",
  assetCurrency: "USD",
  quantity: "2.000000",
  quantityMicro: 2_000_000,
  averagePrice: "100.00",
  costBasis: "200.00",
  realizedPnl: "0.00",
  realizedPnlRaw: 0,
  canEnterPrice: true,
  currentPrice: { kind: "present", formatted: "150.00" },
  currentPriceDate: "2024-01-15",
  unrealizedPnl: "100.00",
  unrealizedPnlRaw: 100_000_000,
  performancePct: "50.00%",
  isCash: false,
  staleness: { key: "mkt.staleness_today" },
  sourceLabel: "mkt.source_stooq",
};

const renderInTable = (row: HoldingRowViewModel) =>
  render(
    <table>
      <tbody>
        <HoldingRow
          row={row}
          accountId="account-1"
          onBuy={vi.fn()}
          onSell={vi.fn()}
          onPriceHistory={vi.fn()}
        />
      </tbody>
    </table>,
  );

describe("HoldingRow — price cell (MKT-030, MKT-140, MKT-142)", () => {
  beforeEach(() => {
    useAppStore.setState({ assets: [], accounts: [] });
    navigateMock.mockClear();
  });

  it("renders the current price and the source + staleness sub-line", () => {
    renderInTable(baseRow);
    expect(screen.getByText("150.00")).toBeInTheDocument();
    expect(screen.getByText("mkt.source_stooq")).toBeInTheDocument();
    expect(screen.getByText("mkt.staleness_today")).toBeInTheDocument();
  });

  it("does not render a standalone 'as of date' sub-line (compact cell)", () => {
    renderInTable(baseRow);
    // The price_as_of key was removed in the compact-cell pass — staleness
    // conveys recency on its own.
    expect(screen.queryByText(/account_details\.price_as_of/)).not.toBeInTheDocument();
    expect(screen.queryByText(/as of 2024-01-15/)).not.toBeInTheDocument();
  });

  it("renders 'Missing ticker' diagnostic when asset_reference is empty (MKT-032)", () => {
    renderInTable({
      ...baseRow,
      currentPrice: { kind: "missing_ticker" },
      staleness: null,
      sourceLabel: null,
    });
    expect(screen.getByText("mkt.price_state.missing_ticker")).toBeInTheDocument();
    // No source or staleness when there's no price
    expect(screen.queryByText("mkt.source_stooq")).not.toBeInTheDocument();
    expect(screen.queryByText("mkt.staleness_today")).not.toBeInTheDocument();
  });

  it("renders 'No price available' diagnostic when reference present but no price (MKT-032)", () => {
    renderInTable({
      ...baseRow,
      currentPrice: { kind: "no_price_available" },
      staleness: null,
      sourceLabel: null,
    });
    expect(screen.getByText("mkt.price_state.no_price_available")).toBeInTheDocument();
    expect(screen.queryByText("mkt.source_stooq")).not.toBeInTheDocument();
    expect(screen.queryByText("mkt.staleness_today")).not.toBeInTheDocument();
  });

  it("'Missing ticker' is a clickable button with stable id (MKT-032 / E1)", () => {
    renderInTable({
      ...baseRow,
      assetId: "asset-42",
      currentPrice: { kind: "missing_ticker" },
      staleness: null,
      sourceLabel: null,
    });
    const button = screen.getByRole("button", { name: "mkt.price_state.missing_ticker" });
    expect(button).toBeInTheDocument();
    expect(button.id).toBe("action-edit-missing-ticker-asset-42");
  });

  it("clicking 'Missing ticker' navigates with modal+editAssetId+focusField search params", () => {
    renderInTable({
      ...baseRow,
      assetId: "asset-42",
      currentPrice: { kind: "missing_ticker" },
      staleness: null,
      sourceLabel: null,
    });
    fireEvent.click(screen.getByRole("button", { name: "mkt.price_state.missing_ticker" }));
    expect(navigateMock).toHaveBeenCalledTimes(1);
    const firstCall = navigateMock.mock.calls[0];
    if (!firstCall) throw new Error("expected navigate to be called");
    const arg = firstCall[0] as { search: (prev: object) => object };
    expect(arg.search({})).toEqual({
      modal: "edit-asset",
      editAssetId: "asset-42",
      focusField: "reference",
    });
  });

  it("'No price available' diagnostic stays non-interactive", () => {
    renderInTable({
      ...baseRow,
      currentPrice: { kind: "no_price_available" },
      staleness: null,
      sourceLabel: null,
    });
    expect(screen.queryByRole("button", { name: "mkt.price_state.no_price_available" })).toBeNull();
    expect(screen.queryByRole("link", { name: "mkt.price_state.no_price_available" })).toBeNull();
  });
});

describe("HoldingRow — double-click opens Edit Asset modal", () => {
  beforeEach(() => {
    useAppStore.setState({ assets: [], accounts: [] });
    navigateMock.mockClear();
  });

  it("double-clicking a holding row navigates with modal=edit-asset+editAssetId", () => {
    renderInTable(baseRow);
    const row = screen.getByText("Apple Inc").closest("tr");
    if (!row) throw new Error("expected a holding row");
    fireEvent.doubleClick(row);
    expect(navigateMock).toHaveBeenCalledTimes(1);
    const arg = navigateMock.mock.calls[0]?.[0] as { search: (prev: object) => object };
    expect(arg.search({})).toEqual({ modal: "edit-asset", editAssetId: "asset-1" });
  });

  it("does not open the modal when the asset is archived", () => {
    useAppStore.setState({
      assets: [{ id: "asset-1", is_archived: true, currency: "USD" }] as unknown as Asset[],
      accounts: [],
    });
    renderInTable(baseRow);
    const row = screen.getByText("Apple Inc").closest("tr");
    if (!row) throw new Error("expected a holding row");
    fireEvent.doubleClick(row);
    expect(navigateMock).not.toHaveBeenCalled();
  });
});

// MKT-153/156 — price-refresh lock IconButton on the holding row.
describe("HoldingRow — price-refresh lock toggle", () => {
  const renderWithToggle = (
    row: HoldingRowViewModel,
    onTogglePriceRefreshLock: (assetId: string, currentlyBlocked: boolean) => void,
  ) =>
    render(
      <table>
        <tbody>
          <HoldingRow
            row={row}
            accountId="account-1"
            onBuy={vi.fn()}
            onSell={vi.fn()}
            onPriceHistory={vi.fn()}
            onTogglePriceRefreshLock={onTogglePriceRefreshLock}
          />
        </tbody>
      </table>,
    );

  beforeEach(() => {
    useAppStore.setState({ assets: [], accounts: [] });
  });

  it("renders the unlock affordance and calls the handler with currentlyBlocked=false when the asset is unlocked", () => {
    useAppStore.setState({
      assets: [
        { id: "asset-1", is_archived: false, price_refresh_blocked: false, currency: "USD" },
      ] as unknown as Asset[],
      accounts: [],
    });
    const toggle = vi.fn();
    renderWithToggle(baseRow, toggle);
    const button = screen.getByRole("button", { name: "mkt.lock.action_block" });
    expect(button.id).toBe("action-toggle-price-refresh-asset-1");
    fireEvent.click(button);
    expect(toggle).toHaveBeenCalledWith("asset-1", false);
  });

  it("renders the lock affordance and calls the handler with currentlyBlocked=true when the asset is locked", () => {
    useAppStore.setState({
      assets: [
        { id: "asset-1", is_archived: false, price_refresh_blocked: true, currency: "USD" },
      ] as unknown as Asset[],
      accounts: [],
    });
    const toggle = vi.fn();
    renderWithToggle(baseRow, toggle);
    const button = screen.getByRole("button", { name: "mkt.lock.action_unblock" });
    fireEvent.click(button);
    expect(toggle).toHaveBeenCalledWith("asset-1", true);
  });

  it("omits the lock button entirely when no handler is provided (backward-compatible)", () => {
    render(
      <table>
        <tbody>
          <HoldingRow
            row={baseRow}
            accountId="account-1"
            onBuy={vi.fn()}
            onSell={vi.fn()}
            onPriceHistory={vi.fn()}
          />
        </tbody>
      </table>,
    );
    expect(screen.queryByRole("button", { name: "mkt.lock.action_block" })).toBeNull();
    expect(screen.queryByRole("button", { name: "mkt.lock.action_unblock" })).toBeNull();
  });
});
