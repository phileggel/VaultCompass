import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AccountPerformanceResponse, PerformancePeriod } from "@/bindings";
import * as gateway from "../gateway";
import { AccountPerformancePage } from "./AccountPerformancePage";

// Mock the gateway so no real Tauri calls fire (F27, docs/test_convention.md § Mocking gateway modules)
vi.mock("../gateway");

// Mock the router — AccountPerformancePage reads accountId from route params
vi.mock("@tanstack/react-router", () => ({
  useParams: () => ({ accountId: "account-1" }),
  useNavigate: () => vi.fn(),
  Link: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

// Identity i18n — t(key) === key so tests assert on stable keys (F24)
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "en-US" },
  }),
}));

// ---- Fixtures ---------------------------------------------------------------

const makeMetric = (gain = 1_000_000_000, pct: number | null = 8_000_000) => ({
  gain,
  pct,
});

const makeYearRow = (overrides: Partial<PerformancePeriod> = {}): PerformancePeriod => ({
  year: 2025,
  month: null,
  end_value: 10_000_000_000,
  period_over_period: makeMetric(),
  year_to_date: null,
  since_inception: makeMetric(2_000_000_000, 20_000_000),
  ...overrides,
});

const makeMonthRow = (
  month: number,
  overrides: Partial<PerformancePeriod> = {},
): PerformancePeriod => ({
  year: 2025,
  month,
  end_value: 10_000_000_000,
  period_over_period: makeMetric(),
  year_to_date: makeMetric(350_000_000, 3_500_000),
  since_inception: makeMetric(2_000_000_000, 20_000_000),
  ...overrides,
});

const makeResponse = (
  overrides: Partial<AccountPerformanceResponse> = {},
): AccountPerformanceResponse => ({
  account_name: "My Portfolio",
  currency: "EUR",
  month_view_available: true,
  yearly: [makeYearRow()],
  monthly: [makeMonthRow(5), makeMonthRow(4), makeMonthRow(3), makeMonthRow(2), makeMonthRow(1)],
  ...overrides,
});

// ---- Tests ------------------------------------------------------------------

describe("AccountPerformancePage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // PRF-050 — loading skeleton displayed while fetch is in-flight
  it("renders a loading skeleton while data is being fetched (PRF-050)", async () => {
    // Never resolves during this test — keeps the component in loading state
    vi.mocked(gateway.accountPerformanceGateway.getAccountPerformance).mockReturnValue(
      new Promise(() => {}),
    );

    render(<AccountPerformancePage />);

    // loading indicator must be in the DOM before the promise resolves
    expect(screen.getByTestId("account-performance-loading")).toBeInTheDocument();
  });

  // PRF-051 — empty state + Add Transaction affordance when no transactions
  it("renders empty state with Add Transaction affordance when account has no data (PRF-051)", async () => {
    vi.mocked(gateway.accountPerformanceGateway.getAccountPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse({ yearly: [], monthly: [] }),
    });

    render(<AccountPerformancePage />);

    expect(await screen.findByTestId("account-performance-empty")).toBeInTheDocument();
    expect(screen.getByTestId("account-performance-add-transaction")).toBeInTheDocument();
  });

  // PRF-052 — error state rendered with Retry button on gateway failure
  it("renders error state with Retry button on gateway error (PRF-052)", async () => {
    vi.mocked(gateway.accountPerformanceGateway.getAccountPerformance).mockResolvedValue({
      status: "error",
      error: { code: "DatabaseError" },
    });

    render(<AccountPerformancePage />);

    expect(await screen.findByTestId("account-performance-error")).toBeInTheDocument();
    expect(screen.getByTestId("account-performance-retry")).toBeInTheDocument();
  });

  // PRF-052 — Retry button triggers a re-fetch
  it("re-fetches when Retry is clicked (PRF-052)", async () => {
    vi.mocked(gateway.accountPerformanceGateway.getAccountPerformance).mockResolvedValue({
      status: "error",
      error: { code: "DatabaseError" },
    });

    render(<AccountPerformancePage />);
    await screen.findByTestId("account-performance-retry");

    await userEvent.click(screen.getByTestId("account-performance-retry"));

    expect(gateway.accountPerformanceGateway.getAccountPerformance).toHaveBeenCalledTimes(2);
  });

  // PRF-053 — back navigation link to Account Details is present
  it("renders a back navigation link to Account Details (PRF-053)", async () => {
    vi.mocked(gateway.accountPerformanceGateway.getAccountPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse(),
    });

    render(<AccountPerformancePage />);

    expect(await screen.findByTestId("account-performance-back")).toBeInTheDocument();
  });

  // PRF-011 — view-mode toggle present when month_view_available is true
  it("renders the month/year view-mode toggle when month view is available (PRF-011, PRF-013)", async () => {
    vi.mocked(gateway.accountPerformanceGateway.getAccountPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse({ month_view_available: true }),
    });

    render(<AccountPerformancePage />);

    expect(await screen.findByTestId("account-performance-view-toggle")).toBeInTheDocument();
  });

  // PRF-013 — toggle hidden or disabled when month_view_available is false
  it("hides or disables the view-mode toggle when month view is not available (PRF-013)", async () => {
    vi.mocked(gateway.accountPerformanceGateway.getAccountPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse({ month_view_available: false, monthly: [] }),
    });

    render(<AccountPerformancePage />);

    await screen.findByTestId("account-performance-table");

    // Toggle must be absent from the DOM OR be disabled
    const toggle = screen.queryByTestId("account-performance-view-toggle");
    if (toggle) {
      expect(toggle).toBeDisabled();
    } else {
      expect(toggle).toBeNull();
    }
  });

  // PRF-014 — page opens in month view when month_view_available is true
  it("opens in month view by default when month view is available (PRF-014)", async () => {
    vi.mocked(gateway.accountPerformanceGateway.getAccountPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse({ month_view_available: true }),
    });

    render(<AccountPerformancePage />);

    await screen.findByTestId("account-performance-table");

    // Year selector is only present in month view (PRF-015)
    expect(screen.getByTestId("account-performance-year-selector")).toBeInTheDocument();
  });

  // PRF-014 — page opens in year view when month view is not available
  it("opens in year view by default when month view is not available (PRF-014)", async () => {
    vi.mocked(gateway.accountPerformanceGateway.getAccountPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse({ month_view_available: false, monthly: [] }),
    });

    render(<AccountPerformancePage />);

    await screen.findByTestId("account-performance-table");

    // Year selector must not be present in year view
    expect(screen.queryByTestId("account-performance-year-selector")).not.toBeInTheDocument();
  });

  // PRF-015 — year selector is present in month view
  it("renders a year selector in month view (PRF-015)", async () => {
    vi.mocked(gateway.accountPerformanceGateway.getAccountPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse({ month_view_available: true }),
    });

    render(<AccountPerformancePage />);

    expect(await screen.findByTestId("account-performance-year-selector")).toBeInTheDocument();
  });

  // PRF-037 — YTD column absent in year view
  it("does not render the YTD column in year view (PRF-037)", async () => {
    vi.mocked(gateway.accountPerformanceGateway.getAccountPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse({ month_view_available: false, monthly: [] }),
    });

    render(<AccountPerformancePage />);

    await screen.findByTestId("account-performance-table");

    // The YTD column header must be absent in year view
    expect(screen.queryByTestId("account-performance-col-ytd")).not.toBeInTheDocument();
  });

  // PRF-037 — YTD column present in month view
  it("renders the YTD column in month view (PRF-037)", async () => {
    vi.mocked(gateway.accountPerformanceGateway.getAccountPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse({ month_view_available: true }),
    });

    render(<AccountPerformancePage />);

    // In month view the YTD column header must be present
    expect(await screen.findByTestId("account-performance-col-ytd")).toBeInTheDocument();
  });

  // PRF-011 — switching from month view to year view removes the YTD column
  it("removes the YTD column when the user switches to year view (PRF-011, PRF-037)", async () => {
    vi.mocked(gateway.accountPerformanceGateway.getAccountPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse({ month_view_available: true }),
    });

    render(<AccountPerformancePage />);

    // Wait for month view to be active (YTD column visible)
    await screen.findByTestId("account-performance-col-ytd");

    // Switch to year view
    await userEvent.click(screen.getByTestId("account-performance-view-toggle-year"));

    expect(screen.queryByTestId("account-performance-col-ytd")).not.toBeInTheDocument();
  });

  // PRF-041 — rows rendered as-is (most-recent first from backend)
  it("renders period rows in the order returned by the backend (PRF-041)", async () => {
    vi.mocked(gateway.accountPerformanceGateway.getAccountPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse({
        month_view_available: false,
        monthly: [],
        yearly: [makeYearRow({ year: 2025 }), makeYearRow({ year: 2024 })],
      }),
    });

    render(<AccountPerformancePage />);

    await screen.findByTestId("account-performance-table");

    const rows = screen.getAllByTestId(/^account-performance-row-/);
    // Most recent year (2025) must appear before 2024
    expect(rows[0]).toHaveAttribute("data-testid", "account-performance-row-2025");
    expect(rows[1]).toHaveAttribute("data-testid", "account-performance-row-2024");
  });

  // PRF-060 — re-fetch subscribes to TransactionUpdated, AssetPriceUpdated, AccountUpdated
  it("calls getAccountPerformance on mount (PRF-060 setup)", async () => {
    vi.mocked(gateway.accountPerformanceGateway.getAccountPerformance).mockResolvedValue({
      status: "ok",
      data: makeResponse(),
    });

    render(<AccountPerformancePage />);

    await screen.findByTestId("account-performance-table");

    expect(gateway.accountPerformanceGateway.getAccountPerformance).toHaveBeenCalledWith(
      "account-1",
    );
  });
});
