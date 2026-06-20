import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AccountDetailsView } from "./AccountDetailsView";

// ── Controlled orchestration hook ───────────────────────────────────────────
const { mockUseAccountDetailsView, mockUseRefreshAccountPrices } = vi.hoisted(() => ({
  mockUseAccountDetailsView: vi.fn(),
  mockUseRefreshAccountPrices: vi.fn(),
}));

vi.mock("./useAccountDetailsView", () => ({
  useAccountDetailsView: () => mockUseAccountDetailsView(),
}));

vi.mock("../refresh_prices/useRefreshAccountPrices", () => ({
  useRefreshAccountPrices: () => mockUseRefreshAccountPrices(),
}));

vi.mock("@tanstack/react-router", () => ({
  useParams: () => ({ accountId: "acc-1" }),
  useNavigate: () => vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

vi.mock("@/lib/logger", () => ({ logger: { info: vi.fn(), error: vi.fn() } }));

// Stub the child modals / rows so the view renders in isolation (no gateway,
// no Tauri, no nested hooks) — this test exercises AccountDetailsView's own JSX.
vi.mock("../buy_transaction/BuyTransactionModal", () => ({ BuyTransactionModal: () => null }));
vi.mock("../sell_transaction/SellTransactionModal", () => ({ SellTransactionModal: () => null }));
vi.mock("../deposit_transaction/DepositTransactionModal", () => ({
  DepositTransactionModal: () => null,
}));
vi.mock("../withdrawal_transaction/WithdrawalTransactionModal", () => ({
  WithdrawalTransactionModal: () => null,
}));
vi.mock("../open_balance/OpenBalanceModal", () => ({ OpenBalanceModal: () => null }));
vi.mock("../price_history/PriceHistoryModal", () => ({ PriceHistoryModal: () => null }));
vi.mock("../dividend_transaction/DividendTransactionModal", () => ({
  DividendTransactionModal: () => <div data-testid="dividend-modal-mounted" />,
}));
vi.mock("./HoldingRow", () => ({ HoldingRow: () => <tr data-testid="holding-row" /> }));
vi.mock("./ClosedHoldingRow", () => ({ ClosedHoldingRow: () => <tr /> }));

const handlers = {
  handleDepositOpen: vi.fn(),
  handleWithdrawalOpen: vi.fn(),
  handleOpenBalanceOpen: vi.fn(),
  handleDividendOpen: vi.fn(),
  handleAddTransaction: vi.fn(),
};

const makeView = (overrides: Record<string, unknown> = {}) => ({
  isLoading: false,
  error: null,
  retry: vi.fn(),
  summary: {
    accountName: "Main",
    totalCostBasis: "1.000,00",
    totalRealizedPnl: "0,00",
    totalRealizedPnlRaw: 0,
    totalUnrealizedPnl: "—",
    totalGlobalValue: "1.100,00",
    totalDividendsReceived: "100,00",
    totalDividendsReceivedRaw: 100_000_000,
    isEmpty: false,
    isAllClosed: false,
    hasClosedHoldings: false,
  },
  holdings: [],
  closedHoldings: [],
  accountCurrency: "EUR",
  hasNonCashActiveHoldings: false,
  hasClosedHoldings: false,
  dividendPayingAssets: [],
  buyTarget: null,
  sellTarget: null,
  historyTarget: null,
  openBalanceOpen: false,
  depositOpen: false,
  withdrawalOpen: false,
  dividendOpen: false,
  ...handlers,
  handleBuyOpen: vi.fn(),
  handleBuyClose: vi.fn(),
  handleBuySuccess: vi.fn(),
  handleSellOpen: vi.fn(),
  handleSellClose: vi.fn(),
  handleSellSuccess: vi.fn(),
  handlePriceHistory: vi.fn(),
  handleHistoryClose: vi.fn(),
  handleOpenBalanceClose: vi.fn(),
  handleOpenBalanceSuccess: vi.fn(),
  handleDepositClose: vi.fn(),
  handleDepositSuccess: vi.fn(),
  handleWithdrawalClose: vi.fn(),
  handleWithdrawalSuccess: vi.fn(),
  handleDividendClose: vi.fn(),
  handleDividendSuccess: vi.fn(),
  handleTogglePriceRefreshLock: vi.fn(),
  ...overrides,
});

describe("AccountDetailsView — header Record menu (DIV-012)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseRefreshAccountPrices.mockReturnValue({ isPending: false, refresh: vi.fn() });
    mockUseAccountDetailsView.mockReturnValue(makeView());
  });

  it("renders the consolidated Record menu trigger and hides items until opened", () => {
    render(<AccountDetailsView />);
    expect(document.querySelector("#account-details-add-menu")).toBeInTheDocument();
    // Closed by default — items absent.
    expect(document.querySelector("#add-menu-dividend")).toBeNull();
  });

  it("opens the menu showing Open balance / Dividend / Free shares, with NO cash actions (DIV-012/CSH-019)", () => {
    render(<AccountDetailsView />);
    fireEvent.click(document.querySelector("#account-details-add-menu")!);
    // CSH-019 — cash Deposit/Withdraw live on the cash row, not this menu.
    expect(document.querySelector("#add-menu-deposit")).toBeNull();
    expect(document.querySelector("#add-menu-withdraw")).toBeNull();
    // Non-cash entries remain.
    expect(document.querySelector("#add-menu-open-balance")).toBeInTheDocument();
    expect(document.querySelector("#add-menu-dividend")).toBeInTheDocument();
    expect(document.querySelector("#add-menu-free-shares")).toBeInTheDocument();
  });

  it("invokes the dividend handler and closes the menu when Dividend is chosen (DIV-010)", () => {
    render(<AccountDetailsView />);
    fireEvent.click(document.querySelector("#account-details-add-menu")!);
    fireEvent.click(document.querySelector("#add-menu-dividend")!);
    expect(handlers.handleDividendOpen).toHaveBeenCalledTimes(1);
    // Menu closes after selection.
    expect(document.querySelector("#add-menu-dividend")).toBeNull();
  });

  it("routes the Open balance menu item to its handler", () => {
    render(<AccountDetailsView />);
    fireEvent.click(document.querySelector("#account-details-add-menu")!);
    fireEvent.click(document.querySelector("#add-menu-open-balance")!);
    expect(handlers.handleOpenBalanceOpen).toHaveBeenCalledTimes(1);
  });

  it("mounts the dividend modal only when dividendOpen is true (DIV-010/020)", () => {
    const { rerender } = render(<AccountDetailsView />);
    expect(screen.queryByTestId("dividend-modal-mounted")).toBeNull();

    mockUseAccountDetailsView.mockReturnValue(makeView({ dividendOpen: true }));
    rerender(<AccountDetailsView />);
    expect(screen.getByTestId("dividend-modal-mounted")).toBeInTheDocument();
  });

  it("surfaces the total dividends received in the header when non-zero (DIV-073)", () => {
    render(<AccountDetailsView />);
    expect(document.querySelector("#account-details-total-dividends")).toBeInTheDocument();
  });

  it("hides the total-dividends tile when none recorded (DIV-073)", () => {
    mockUseAccountDetailsView.mockReturnValue(
      makeView({
        summary: { ...makeView().summary, totalDividendsReceivedRaw: 0 },
      }),
    );
    render(<AccountDetailsView />);
    expect(document.querySelector("#account-details-total-dividends")).toBeNull();
  });

  it("renders the Dividends and Total Return column headers when holdings exist (DIV-072)", () => {
    mockUseAccountDetailsView.mockReturnValue(
      makeView({ hasNonCashActiveHoldings: true, holdings: [{ assetId: "a1" }] }),
    );
    render(<AccountDetailsView />);
    expect(screen.getByText("account_details.column_dividends_received")).toBeInTheDocument();
    expect(screen.getByText("account_details.column_total_return_pct")).toBeInTheDocument();
    expect(screen.getByTestId("holding-row")).toBeInTheDocument();
  });
});

describe("AccountDetailsView — add-transaction FAB (ACD-035/036)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseRefreshAccountPrices.mockReturnValue({ isPending: false, refresh: vi.fn() });
  });

  it("renders a single add-transaction FAB and triggers handleAddTransaction on click", () => {
    mockUseAccountDetailsView.mockReturnValue(makeView());
    render(<AccountDetailsView />);

    const fab = screen.getByRole("button", { name: "account_details.add_transaction" });
    fireEvent.click(fab);
    expect(handlers.handleAddTransaction).toHaveBeenCalledTimes(1);
  });

  it("shows the FAB in the empty state with no inline CTA button", () => {
    // No non-cash active holdings → the asset-positions empty message renders
    // (the always-present cash row is excluded from the count, CSH-098).
    mockUseAccountDetailsView.mockReturnValue(
      makeView({
        summary: { ...makeView().summary, isEmpty: true },
        hasNonCashActiveHoldings: false,
      }),
    );
    render(<AccountDetailsView />);

    expect(screen.getByText("account_details.empty_no_positions")).toBeInTheDocument();
    // The only add-transaction affordance is the FAB (exactly one).
    expect(screen.getAllByRole("button", { name: "account_details.add_transaction" })).toHaveLength(
      1,
    );
  });

  it("shows 'All positions closed' when closed holdings exist but no active non-cash (ACD-034)", () => {
    mockUseAccountDetailsView.mockReturnValue(
      makeView({
        summary: { ...makeView().summary, isAllClosed: true },
        hasNonCashActiveHoldings: false,
        hasClosedHoldings: true,
      }),
    );
    render(<AccountDetailsView />);
    expect(screen.getByText("account_details.empty_all_closed")).toBeInTheDocument();
    expect(screen.queryByText("account_details.empty_no_positions")).toBeNull();
  });
});
