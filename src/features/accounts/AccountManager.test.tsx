import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PriceMovementReport } from "@/bindings";
import { AccountManager } from "./AccountManager";

const { mockNavigate, mockUsePriceMovementReport } = vi.hoisted(() => ({
  mockNavigate: vi.fn(),
  mockUsePriceMovementReport: vi.fn(),
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => mockNavigate,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

vi.mock("@/lib/logger", () => ({ logger: { info: vi.fn(), error: vi.fn() } }));

// Stub the children so the manager renders in isolation (no gateway, no Tauri).
vi.mock("./account_table/AccountTable", () => ({
  AccountTable: () => <div data-testid="account-table" />,
}));
vi.mock("./add_account/AddAccountModal", () => ({ AddAccountModal: () => null }));
vi.mock("./refresh_prices/useRefreshGlobalPrices", () => ({
  useRefreshGlobalPrices: () => ({ isPending: false, refresh: vi.fn() }),
}));

// PMV-013/016/061 — the panel and its mount-scoped hook are covered on their
// own (PriceMovementPanel.test.tsx, usePriceMovementReport.test.ts); here we
// only verify AccountManager wires the hook's output into the panel.
vi.mock("./price_movement/usePriceMovementReport", () => ({
  usePriceMovementReport: () => mockUsePriceMovementReport(),
}));
vi.mock("./price_movement/PriceMovementPanel", () => ({
  PriceMovementPanel: (props: { report: unknown; onDismiss: () => void }) => (
    <button type="button" data-testid="price-movement-panel-stub" onClick={props.onDismiss} />
  ),
}));

describe("AccountManager — global performance entry point (GPF)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUsePriceMovementReport.mockReturnValue({ report: null, dismiss: vi.fn() });
  });

  it("renders the performance entry button next to the search field", () => {
    render(<AccountManager />);
    expect(document.querySelector("#accounts-performance")).toBeInTheDocument();
  });

  it("navigates to /performance when the entry button is clicked", () => {
    render(<AccountManager />);
    fireEvent.click(document.querySelector("#accounts-performance")!);
    expect(mockNavigate).toHaveBeenCalledWith({ to: "/performance" });
  });
});

// PMV-013 — AccountManager mounts usePriceMovementReport and renders the
// panel above the account rows whenever a report is present. Both the hook
// and the panel are mocked here — their own behavior is covered in
// usePriceMovementReport.test.ts and PriceMovementPanel.test.tsx.
describe("AccountManager — price movement panel (PMV-013)", () => {
  const makeReport = (): PriceMovementReport => ({
    rows: [],
    total_before: 0,
    total_after: 0,
    total_currency: "EUR",
    total_movement_pct: null,
    observed_from: null,
    observed_to: null,
    incomplete: false,
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mockUsePriceMovementReport.mockReturnValue({ report: null, dismiss: vi.fn() });
  });

  it("does not render the panel when the hook has no report", () => {
    render(<AccountManager />);

    expect(screen.queryByTestId("price-movement-panel-stub")).not.toBeInTheDocument();
  });

  // PMV-013 — the panel appears above the account rows, not in place of them.
  it("renders the panel above the account rows when the hook has a report", () => {
    mockUsePriceMovementReport.mockReturnValue({ report: makeReport(), dismiss: vi.fn() });

    render(<AccountManager />);

    expect(screen.getByTestId("price-movement-panel-stub")).toBeInTheDocument();
    expect(screen.getByTestId("account-table")).toBeInTheDocument();
  });

  // PMV-061 — dismissing routes through the hook, which is the only place
  // that decides the report is gone for good.
  it("wires the hook's dismiss into the panel", () => {
    const dismiss = vi.fn();
    mockUsePriceMovementReport.mockReturnValue({ report: makeReport(), dismiss });

    render(<AccountManager />);
    fireEvent.click(screen.getByTestId("price-movement-panel-stub"));

    expect(dismiss).toHaveBeenCalledTimes(1);
  });
});
