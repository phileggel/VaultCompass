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
  DepositTransactionModal: () => <div data-testid="deposit-modal-mounted" />,
}));
vi.mock("../withdrawal_transaction/WithdrawalTransactionModal", () => ({
  WithdrawalTransactionModal: () => <div data-testid="withdrawal-modal-mounted" />,
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
  handleFreeSharesOpen: vi.fn(),
  handleAddTransaction: vi.fn(),
};

const makeView = (overrides: Record<string, unknown> = {}) => ({
  isLoading: false,
  error: null,
  retry: vi.fn(),
  summary: {
    accountName: "Main",
    totalGlobalValue: "1.100,00",
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
  asOfDate: "",
  asOfDisplayDate: "2024-06-01",
  isAsOf: false,
  setAsOfDate: vi.fn(),
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

describe("AccountDetailsView — header actions (DIV-012)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseRefreshAccountPrices.mockReturnValue({ isPending: false, refresh: vi.fn() });
    mockUseAccountDetailsView.mockReturnValue(makeView());
  });

  it("renders the record actions as direct buttons — no dropdown trigger", () => {
    render(<AccountDetailsView />);
    // The former consolidated dropdown is gone; actions are direct icon buttons.
    expect(document.querySelector("#account-details-add-menu")).toBeNull();
    expect(document.querySelector("#add-menu-open-balance")).toBeInTheDocument();
    expect(document.querySelector("#add-menu-dividend")).toBeInTheDocument();
    expect(document.querySelector("#add-menu-free-shares")).toBeInTheDocument();
  });

  it("shows Open balance / Dividend / Free shares, with NO cash actions (DIV-012/CSH-019)", () => {
    render(<AccountDetailsView />);
    // CSH-019 — cash Deposit/Withdraw live on the cash row, not the header.
    expect(document.querySelector("#add-menu-deposit")).toBeNull();
    expect(document.querySelector("#add-menu-withdraw")).toBeNull();
    expect(document.querySelector("#add-menu-open-balance")).toBeInTheDocument();
    expect(document.querySelector("#add-menu-dividend")).toBeInTheDocument();
    expect(document.querySelector("#add-menu-free-shares")).toBeInTheDocument();
  });

  it("invokes the dividend handler when the Dividend button is clicked (DIV-010)", () => {
    render(<AccountDetailsView />);
    fireEvent.click(document.querySelector("#add-menu-dividend")!);
    expect(handlers.handleDividendOpen).toHaveBeenCalledTimes(1);
  });

  it("routes the Open balance button to its handler", () => {
    render(<AccountDetailsView />);
    fireEvent.click(document.querySelector("#add-menu-open-balance")!);
    expect(handlers.handleOpenBalanceOpen).toHaveBeenCalledTimes(1);
  });

  it("routes the Free shares button to its handler (FSD-010)", () => {
    render(<AccountDetailsView />);
    fireEvent.click(document.querySelector("#add-menu-free-shares")!);
    expect(handlers.handleFreeSharesOpen).toHaveBeenCalledTimes(1);
  });

  it("mounts the dividend modal only when dividendOpen is true (DIV-010/020)", () => {
    const { rerender } = render(<AccountDetailsView />);
    expect(screen.queryByTestId("dividend-modal-mounted")).toBeNull();

    mockUseAccountDetailsView.mockReturnValue(makeView({ dividendOpen: true }));
    rerender(<AccountDetailsView />);
    expect(screen.getByTestId("dividend-modal-mounted")).toBeInTheDocument();
  });

  it("mounts the deposit modal only while depositOpen is true (CSH-022)", () => {
    const { rerender } = render(<AccountDetailsView />);
    expect(screen.queryByTestId("deposit-modal-mounted")).toBeNull();

    mockUseAccountDetailsView.mockReturnValue(makeView({ depositOpen: true }));
    rerender(<AccountDetailsView />);
    expect(screen.getByTestId("deposit-modal-mounted")).toBeInTheDocument();
  });

  it("mounts the withdrawal modal only while withdrawalOpen is true (CSH-032)", () => {
    const { rerender } = render(<AccountDetailsView />);
    expect(screen.queryByTestId("withdrawal-modal-mounted")).toBeNull();

    mockUseAccountDetailsView.mockReturnValue(makeView({ withdrawalOpen: true }));
    rerender(<AccountDetailsView />);
    expect(screen.getByTestId("withdrawal-modal-mounted")).toBeInTheDocument();
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

describe("AccountDetailsView — read-only as-of view", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseRefreshAccountPrices.mockReturnValue({ isPending: false, refresh: vi.fn() });
  });

  it("renders the as-of date selector in the header", () => {
    mockUseAccountDetailsView.mockReturnValue(makeView());
    render(<AccountDetailsView />);
    expect(document.querySelector("#account-details-as-of-date")).toBeInTheDocument();
  });

  it("shows the as-of banner + reset and hides the mutating controls when isAsOf", () => {
    const setAsOfDate = vi.fn();
    mockUseAccountDetailsView.mockReturnValue(
      makeView({ isAsOf: true, asOfDate: "2024-06-01", setAsOfDate }),
    );
    render(<AccountDetailsView />);

    // Banner + reset present.
    expect(document.querySelector("#account-details-as-of-banner")).toBeInTheDocument();
    const reset = document.querySelector("#account-details-as-of-reset")!;
    expect(reset).toBeInTheDocument();
    fireEvent.click(reset);
    expect(setAsOfDate).toHaveBeenCalledWith("");

    // Mutating controls hidden.
    expect(document.querySelector("#account-details-refresh-prices")).toBeNull();
    expect(document.querySelector("#add-menu-open-balance")).toBeNull();
    expect(document.querySelector("#add-menu-dividend")).toBeNull();
    expect(document.querySelector("#add-menu-free-shares")).toBeNull();
    expect(document.querySelector("#account-details-add-transaction-fab")).toBeNull();
  });

  it("keeps the mutating controls in the live view (isAsOf false)", () => {
    mockUseAccountDetailsView.mockReturnValue(makeView({ isAsOf: false }));
    render(<AccountDetailsView />);
    expect(document.querySelector("#account-details-as-of-banner")).toBeNull();
    expect(document.querySelector("#account-details-refresh-prices")).toBeInTheDocument();
    expect(document.querySelector("#add-menu-open-balance")).toBeInTheDocument();
    expect(document.querySelector("#add-menu-dividend")).toBeInTheDocument();
    expect(document.querySelector("#add-menu-free-shares")).toBeInTheDocument();
    expect(document.querySelector("#account-details-add-transaction-fab")).toBeInTheDocument();
  });
});
