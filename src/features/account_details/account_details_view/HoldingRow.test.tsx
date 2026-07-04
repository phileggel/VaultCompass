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
  currentValue: "300.00",
  realizedPnl: "0.00",
  realizedPnlRaw: 0,
  canEnterPrice: true,
  currentPrice: { kind: "present", formatted: "150.00" },
  currentPriceDate: "2024-01-15",
  unrealizedPnl: "100.00",
  unrealizedPnlRaw: 100_000_000,
  performancePct: "50.00%",
  dividendsReceived: "0.00",
  managementFees: "0.00",
  weightPct: "—",
  feeRatePct: null,
  totalReturnPct: "50.00%",
  totalReturnPctRaw: 50_000_000,
  isCash: false,
  staleness: { key: "mkt.staleness_today" },
  sourceLabel: "mkt.source_yahoo",
};

const renderInTable = (row: HoldingRowViewModel, readOnly = false) =>
  render(
    <table>
      <tbody>
        <HoldingRow
          row={row}
          accountId="account-1"
          onBuy={vi.fn()}
          onSell={vi.fn()}
          onPriceHistory={vi.fn()}
          onDeposit={vi.fn()}
          onWithdraw={vi.fn()}
          readOnly={readOnly}
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
    expect(screen.getByText("mkt.source_yahoo")).toBeInTheDocument();
    expect(screen.getByText("mkt.staleness_today")).toBeInTheDocument();
  });

  // ACD-052 — weight % cell renders after the current value
  it("renders the weight % cell", () => {
    const { container } = renderInTable({ ...baseRow, weightPct: "22,00%" });
    const cell = container.querySelector("#holding-weight-pct-asset-1");
    expect(cell).not.toBeNull();
    expect(cell?.textContent).toBe("22,00%");
  });

  // FEE-074 — the active schedule's rate rides along in the fees cell
  it("renders the fee rate next to the management fees when a schedule exists", () => {
    const { container } = renderInTable({
      ...baseRow,
      managementFees: "12,34",
      feeRatePct: "1,50%",
    });
    const cell = container.querySelector("#holding-management-fees-asset-1");
    expect(cell?.textContent).toContain("12,34");
    expect(container.querySelector("#holding-fee-rate-asset-1")?.textContent).toBe("· 1,50%");
  });

  // FEE-074 — no schedule → the fees cell shows the amount alone
  it("renders no fee rate marker without a schedule", () => {
    const { container } = renderInTable({ ...baseRow, managementFees: "12,34" });
    expect(container.querySelector("#holding-fee-rate-asset-1")).toBeNull();
  });

  // FEE-076 — the fees cell is absent when the account has the mechanism disabled
  it("omits the management fees cell when showManagementFees is false", () => {
    const { container } = render(
      <table>
        <tbody>
          <HoldingRow
            row={baseRow}
            accountId="account-1"
            onBuy={vi.fn()}
            onSell={vi.fn()}
            onPriceHistory={vi.fn()}
            showManagementFees={false}
          />
        </tbody>
      </table>,
    );
    expect(container.querySelector("#holding-management-fees-asset-1")).toBeNull();
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
    expect(screen.queryByText("mkt.source_yahoo")).not.toBeInTheDocument();
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
    expect(screen.queryByText("mkt.source_yahoo")).not.toBeInTheDocument();
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

describe("HoldingRow — dividend columns (DIV-072)", () => {
  beforeEach(() => {
    useAppStore.setState({ assets: [], accounts: [] });
    navigateMock.mockClear();
  });

  it("renders the dividends-received amount and the total-return %", () => {
    renderInTable({
      ...baseRow,
      dividendsReceived: "42.50",
      totalReturnPct: "12.75%",
      totalReturnPctRaw: 12_750_000,
    });
    expect(screen.getByText("42.50")).toBeInTheDocument();
    expect(screen.getByText("12.75%")).toBeInTheDocument();
  });

  it("renders an em dash for total return when not computable", () => {
    // weightPct is given a value so total-return is the only dash in the row.
    renderInTable({
      ...baseRow,
      totalReturnPct: "—",
      totalReturnPctRaw: null,
      weightPct: "12,00%",
    });
    // The dividends cell still shows its (zero) amount; total-return is the dash.
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("colors a negative total return with the loss style", () => {
    renderInTable({
      ...baseRow,
      totalReturnPct: "-8.25%",
      totalReturnPctRaw: -8_250_000,
    });
    const cell = screen.getByText("-8.25%");
    expect(cell).toBeInTheDocument();
    expect(cell.className).toContain("text-m3-loss");
  });
});

describe("HoldingRow — read-only as-of view", () => {
  beforeEach(() => {
    useAppStore.setState({ assets: [], accounts: [] });
    navigateMock.mockClear();
  });

  it("hides Buy/Sell/price-history mutating actions but keeps view-transactions", () => {
    renderInTable(baseRow, true);
    expect(document.querySelector("#action-buy-asset-1")).toBeNull();
    expect(document.querySelector("#action-sell-asset-1")).toBeNull();
    expect(document.querySelector("#action-price-history-asset-1")).toBeNull();
    expect(document.querySelector("#action-view-transactions-asset-1")).toBeInTheDocument();
  });

  it("hides Deposit/Withdraw on the cash row in read-only mode", () => {
    const cashRow: HoldingRowViewModel = {
      ...baseRow,
      assetId: "system-cash-eur",
      assetReference: "EUR",
      isCash: true,
    };
    renderInTable(cashRow, true);
    expect(document.querySelector("#action-record-deposit-system-cash-eur")).toBeNull();
    expect(document.querySelector("#action-record-withdrawal-system-cash-eur")).toBeNull();
    expect(document.querySelector("#action-view-transactions-system-cash-eur")).toBeInTheDocument();
  });

  it("does not open the Edit Asset modal on double-click in read-only mode", () => {
    renderInTable(baseRow, true);
    const row = screen.getByText("Apple Inc").closest("tr");
    if (!row) throw new Error("expected a holding row");
    fireEvent.doubleClick(row);
    expect(navigateMock).not.toHaveBeenCalled();
  });

  it("hides the missing-ticker edit shortcut (shows plain text) in read-only mode", () => {
    renderInTable(
      {
        ...baseRow,
        currentPrice: { kind: "missing_ticker" },
        staleness: null,
        sourceLabel: null,
      },
      true,
    );
    expect(document.querySelector("#action-edit-missing-ticker-asset-1")).toBeNull();
    // The plain state text still renders, just not as a clickable write affordance.
    expect(screen.getByText("mkt.price_state.missing_ticker")).toBeInTheDocument();
  });

  it("hides the Record-FX-rate shortcut in read-only mode", () => {
    renderInTable(
      {
        ...baseRow,
        assetId: "asset-usd-1",
        assetCurrency: "USD",
        unrealizedPnl: "—",
        unrealizedPnlRaw: null,
        performancePct: "—",
        currentPrice: { kind: "present", formatted: "150.00" },
      },
      true,
    );
    expect(screen.queryByTestId("action-record-fx-rate-asset-usd-1")).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// FXR-012 — foreign-currency "—" cell becomes a clickable shortcut
// FXR-090 — staleness label renders when a converted value is shown
// ---------------------------------------------------------------------------

describe("HoldingRow — FX shortcut on foreign-currency holding (FXR-012)", () => {
  beforeEach(() => {
    useAppStore.setState({ assets: [], accounts: [] });
    navigateMock.mockClear();
  });

  const foreignRow: HoldingRowViewModel = {
    ...baseRow,
    assetId: "asset-usd-1",
    assetCurrency: "USD",
    // unrealizedPnl "—" signals no usable rate (FXR-034/091)
    unrealizedPnl: "—",
    unrealizedPnlRaw: null,
    performancePct: "—",
    currentPrice: { kind: "present", formatted: "150.00" },
  };

  it("renders the FX shortcut button on a foreign-currency holding row (FXR-012)", () => {
    renderInTable(foreignRow);
    expect(screen.getByTestId("action-record-fx-rate-asset-usd-1")).toBeInTheDocument();
  });

  it("clicking the FX shortcut navigates with modal=record-fx-rate plus fxFrom/fxTo (FXR-012)", () => {
    useAppStore.setState({
      assets: [
        { id: "asset-usd-1", currency: "USD", is_archived: false },
      ] as unknown as import("@/bindings").Asset[],
      accounts: [{ id: "account-1", currency: "EUR" }] as unknown as import("@/bindings").Account[],
    });

    renderInTable({ ...foreignRow, accountId: "account-1" } as HoldingRowViewModel & {
      accountId: string;
    });
    fireEvent.click(screen.getByTestId("action-record-fx-rate-asset-usd-1"));

    expect(navigateMock).toHaveBeenCalledTimes(1);
    const arg = navigateMock.mock.calls[0]?.[0] as { search: (prev: object) => object };
    expect(arg.search({})).toEqual({
      modal: "record-fx-rate",
      fxFrom: "USD",
      fxTo: "EUR",
    });
  });
});

describe("HoldingRow — FX rate staleness label (FXR-090)", () => {
  beforeEach(() => {
    useAppStore.setState({ assets: [], accounts: [] });
    navigateMock.mockClear();
  });

  it("renders the FX staleness label when a converted value is shown (FXR-090)", () => {
    renderInTable({
      ...baseRow,
      fxStaleness: { key: "currency.rate_staleness_today" },
    } as HoldingRowViewModel & { fxStaleness: unknown });
    expect(screen.getByText("currency.rate_staleness_today")).toBeInTheDocument();
  });

  it("does not render FX staleness when no converted value (FXR-090)", () => {
    renderInTable({
      ...baseRow,
      fxStaleness: null,
    } as HoldingRowViewModel & { fxStaleness: unknown });
    expect(screen.queryByText(/currency\.rate_staleness/)).not.toBeInTheDocument();
  });
});

describe("HoldingRow — cash row (CSH-110 view transactions)", () => {
  beforeEach(() => {
    useAppStore.setState({ assets: [], accounts: [] });
    navigateMock.mockClear();
  });

  const cashRow: HoldingRowViewModel = {
    ...baseRow,
    assetId: "system-cash-eur",
    assetName: "Cash EUR",
    assetReference: "EUR",
    isCash: true,
    quantityMicro: 500_000_000,
  };

  it("renders a View-transactions inspect action on the cash row (CSH-110)", () => {
    render(
      <table>
        <tbody>
          <HoldingRow
            row={cashRow}
            accountId="account-1"
            onBuy={vi.fn()}
            onSell={vi.fn()}
            onPriceHistory={vi.fn()}
            onDeposit={vi.fn()}
            onWithdraw={vi.fn()}
          />
        </tbody>
      </table>,
    );
    expect(document.querySelector("#action-view-transactions-system-cash-eur")).toBeInTheDocument();
  });

  it("navigates to the cash asset's transaction list when inspect is clicked (CSH-110)", () => {
    render(
      <table>
        <tbody>
          <HoldingRow
            row={cashRow}
            accountId="account-1"
            onBuy={vi.fn()}
            onSell={vi.fn()}
            onPriceHistory={vi.fn()}
            onDeposit={vi.fn()}
            onWithdraw={vi.fn()}
          />
        </tbody>
      </table>,
    );
    fireEvent.click(document.querySelector("#action-view-transactions-system-cash-eur")!);
    expect(navigateMock).toHaveBeenCalledTimes(1);
    const arg = navigateMock.mock.calls[0]?.[0] as { params: { assetId: string } };
    expect(arg.params.assetId).toBe("system-cash-eur");
  });
});

describe("HoldingRow — management fees column + action (FEE-052/011)", () => {
  beforeEach(() => {
    useAppStore.setState({ assets: [], accounts: [] });
    navigateMock.mockClear();
  });

  const renderWithManageFee = (
    row: HoldingRowViewModel,
    onManageFee: (assetId: string, assetName: string) => void,
    readOnly = false,
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
            onManageFee={onManageFee}
            readOnly={readOnly}
          />
        </tbody>
      </table>,
    );

  it("renders the management fees cell value (FEE-052)", () => {
    renderWithManageFee({ ...baseRow, managementFees: "12.34" }, vi.fn());
    const cell = document.querySelector("#holding-management-fees-asset-1");
    expect(cell).toHaveTextContent("12.34");
  });

  it("opens the fee-schedule modal with asset id + name on manage-fee click (FEE-011)", () => {
    const onManageFee = vi.fn();
    renderWithManageFee(baseRow, onManageFee);
    fireEvent.click(document.querySelector("#action-manage-fee-asset-1")!);
    expect(onManageFee).toHaveBeenCalledWith("asset-1", "Apple Inc");
  });

  it("hides the manage-fee action in the read-only as-of view", () => {
    renderWithManageFee(baseRow, vi.fn(), true);
    expect(document.querySelector("#action-manage-fee-asset-1")).toBeNull();
  });
});
