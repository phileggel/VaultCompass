import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { HoldingsAsOfModal } from "./HoldingsAsOfModal";
import type { HoldingAsOfRowViewModel } from "./presenter";

const { mockUseHoldingsAsOf } = vi.hoisted(() => ({
  mockUseHoldingsAsOf: vi.fn(),
}));

vi.mock("./useHoldingsAsOf", () => ({
  useHoldingsAsOf: () => mockUseHoldingsAsOf(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "en" },
  }),
}));

const ROW: HoldingAsOfRowViewModel = {
  assetId: "asset-1",
  assetName: "Apple",
  quantity: "2",
  averageCost: "100.00",
  price: "120.00",
  priceDate: "2024-03-01",
  marketValue: "240.00",
  unrealizedPnl: "40.00",
  unrealizedPnlRaw: 40_000_000,
  isCash: false,
};

const makeView = (overrides: Record<string, unknown> = {}) => ({
  date: "2024-06-01",
  setDate: vi.fn(),
  rows: [ROW],
  totalCostBasis: "200.00",
  totalMarketValue: "240.00",
  accountCurrency: "EUR",
  isLoading: false,
  error: null,
  ...overrides,
});

const BASE_PROPS = {
  isOpen: true,
  onClose: vi.fn(),
  accountId: "account-1",
};

describe("HoldingsAsOfModal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseHoldingsAsOf.mockReturnValue(makeView());
  });

  it("renders the date field and a holding row", () => {
    render(<HoldingsAsOfModal {...BASE_PROPS} />);
    expect(screen.getByTestId("holdings-as-of-date")).toBeInTheDocument();
    expect(screen.getByText("Apple")).toBeInTheDocument();
    // "120.00" is the price cell (the total + market-value cells share "240.00").
    expect(screen.getByText("120.00")).toBeInTheDocument();
    expect(screen.getAllByText("240.00").length).toBeGreaterThanOrEqual(1);
  });

  it("renders the empty state when there are no rows", () => {
    mockUseHoldingsAsOf.mockReturnValue(makeView({ rows: [] }));
    render(<HoldingsAsOfModal {...BASE_PROPS} />);
    expect(screen.getByText("holdings_as_of.empty")).toBeInTheDocument();
  });

  it("renders the loading skeleton while loading", () => {
    mockUseHoldingsAsOf.mockReturnValue(makeView({ isLoading: true, rows: [] }));
    render(<HoldingsAsOfModal {...BASE_PROPS} />);
    expect(screen.getByTestId("holdings-as-of-loading")).toBeInTheDocument();
  });

  it("renders an error alert when error is set", () => {
    mockUseHoldingsAsOf.mockReturnValue(
      makeView({ error: { key: "error.DateInFuture" }, rows: [] }),
    );
    render(<HoldingsAsOfModal {...BASE_PROPS} />);
    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("error.DateInFuture");
  });
});
