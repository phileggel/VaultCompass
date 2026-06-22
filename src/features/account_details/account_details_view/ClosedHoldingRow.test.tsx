import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ClosedHoldingRowViewModel } from "../shared/presenter";
import { ClosedHoldingRow } from "./ClosedHoldingRow";

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "en-US" },
  }),
}));

const baseRow: ClosedHoldingRowViewModel = {
  assetId: "asset-1",
  assetName: "Apple Inc",
  assetReference: "AAPL",
  realizedPnl: "150.00",
  realizedPnlRaw: 150_000_000,
  dividendsReceived: "20.00",
  dividendsReceivedRaw: 20_000_000,
  totalRevenues: "170.00",
  totalRevenuesRaw: 170_000_000,
  lastSoldDate: "2024-01-15",
};

const renderInTable = (row: ClosedHoldingRowViewModel) =>
  render(
    <table>
      <tbody>
        <ClosedHoldingRow row={row} accountId="account-1" />
      </tbody>
    </table>,
  );

describe("ClosedHoldingRow", () => {
  it("renders asset name and reference", () => {
    renderInTable(baseRow);
    expect(screen.getByText("Apple Inc")).toBeInTheDocument();
    expect(screen.getByText("AAPL")).toBeInTheDocument();
  });

  it("renders dividends received and total revenues", () => {
    renderInTable(baseRow);
    expect(screen.getByText("20.00")).toBeInTheDocument();
    expect(screen.getByText("170.00")).toBeInTheDocument();
  });

  it("renders the lastSoldDate via formatIsoDate threaded with i18n.language", () => {
    // With language "en-US", "2024-01-15" formats to a string containing "2024" and "15".
    // The raw ISO string must NOT appear (formatIsoDate transforms it).
    renderInTable(baseRow);
    expect(screen.queryByText("2024-01-15")).not.toBeInTheDocument();
    const dateCell = screen.getByText(/2024/);
    expect(dateCell.textContent).toContain("15");
  });
});
